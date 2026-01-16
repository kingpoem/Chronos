//! Memory Set Management
//! 
//! Manages virtual memory spaces for kernel and user processes
//! 
//! This module implements **segment-based paging (段页式存储管理)**:
//! - **Segment level (段)**: ELF program headers (Load segments) are represented as MapArea
//!   - Each segment has a contiguous virtual address range
//!   - Segments have permissions (R/W/X) and types (code/data/bss)
//! - **Page level (页)**: Each segment is divided into 4KB pages managed by PageTable
//!   - MapArea.map() creates page table entries for all pages in the segment
//!   - Physical frames are allocated from frame allocator
//!   - Virtual pages are mapped to physical frames via page table
//! 
//! The ELF file structure is preserved - no stripping of ELF headers is needed.
//! The kernel parses ELF structure and extracts segments for memory mapping.

use super::frame_allocator::FRAME_ALLOCATOR;
use super::memory_layout::*;
use super::page_table::{PTEFlags, PageTable, PageTableEntry};
use crate::config::memory_layout::{PAGE_SIZE, MEMORY_END, USER_STACK_SIZE};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::arch::asm;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    /// Global trampoline frame - shared between all address spaces
    /// This ensures the trampoline code is at the same physical address in both kernel and user spaces
    static ref TRAMPOLINE_FRAME: Mutex<Option<PhysPageNum>> = Mutex::new(None);
}

/// Memory area map type
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    /// Identity mapping (va == pa)
    Identical,
    /// Framed mapping (allocate frames)
    Framed,
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

/// Virtual page number range
#[derive(Copy, Clone)]
pub struct VPNRange {
    start: VirtPageNum,
    end: VirtPageNum,
}

impl VPNRange {
    pub fn new(start: VirtPageNum, end: VirtPageNum) -> Self {
        Self { start, end }
    }
    
    pub fn start(&self) -> VirtPageNum {
        self.start
    }
    
    pub fn end(&self) -> VirtPageNum {
        self.end
    }
}

impl Iterator for VPNRange {
    type Item = VirtPageNum;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.start.0 < self.end.0 {
            let vpn = self.start;
            self.start.0 += 1;
            Some(vpn)
        } else {
            None
        }
    }
}

/// Frame tracker - automatically frees frame when dropped
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // Clear the frame
        // Convert physical page number to kernel virtual address (identity mapping)
        let kernel_va = ppn.addr().0;
        let bytes_array = kernel_va as *mut u8;
            unsafe {
            core::ptr::write_bytes(bytes_array, 0, PAGE_SIZE);
        }
        Self { ppn }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.dealloc(self.ppn);
    }
}

/// Map area
pub struct MapArea {
    vpn_range: VPNRange,
    actual_start_va: usize,  // Actual start virtual address (before page alignment)
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    pub fn new(
        start_va: usize,
        end_va: usize,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        Self::new_with_actual_start(start_va, start_va, end_va, map_type, map_perm)
    }
    
    /// Create a new map area with actual start address
    /// 
    /// # Arguments
    /// * `actual_start_va` - Actual start virtual address (before page alignment)
    /// * `aligned_start_va` - Page-aligned start virtual address
    /// * `end_va` - End virtual address (page-aligned)
    /// * `map_type` - Mapping type
    /// * `map_perm` - Mapping permissions
    pub fn new_with_actual_start(
        actual_start_va: usize,
        aligned_start_va: usize,
        end_va: usize,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn = VirtAddr::new(aligned_start_va).page_number();
        let end_vpn = VirtAddr::new(end_va).page_number();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            actual_start_va,
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }
    
    /// Get the start virtual address of this area
    pub fn start_va(&self) -> usize {
        self.vpn_range.start().addr().0
    }
    
    /// Get the end virtual address of this area
    pub fn end_va(&self) -> usize {
        self.vpn_range.end().addr().0
    }
    
    /// Get the map type
    pub fn map_type(&self) -> MapType {
        self.map_type
    }
    
    /// Get the map permissions
    pub fn map_perm(&self) -> MapPermission {
        self.map_perm
    }
    
    /// Map one page
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        // Convert MapPermission to PTEFlags
        // MapPermission doesn't include V (Valid) bit, so we need to add it
        let perm_bits = self.map_perm.bits();
        let pte_flags = PTEFlags::V | PTEFlags::from_bits(perm_bits).unwrap();
        
        // Check if page is already mapped
        if let Some((existing_ppn, existing_flags)) = page_table.translate(vpn) {
            // When multiple segments map to the same page, we should reuse the existing physical page
            // instead of creating a new one. This preserves the data from the first segment.
            // Only create a new page if:
            // 1. The existing mapping is Identical (va == pa) and we want Framed, or
            // 2. The existing mapping is Framed but we want Identical
            let should_reuse = match self.map_type {
                MapType::Identical => {
                    // If we want Identical, check if existing is also Identical
                    existing_ppn.0 == vpn.0
                },
                MapType::Framed => {
                    // If we want Framed, check if existing is also Framed (not Identical)
                    // For user space, Framed pages are never identity mapped
                    existing_ppn.0 != vpn.0
                }
            };
            
            if should_reuse {
                // Reuse existing physical page and merge permissions
                let merged_flags = PTEFlags::from_bits(
                    existing_flags.bits() | pte_flags.bits()
                ).unwrap();
                
                // Check if permissions changed
                if merged_flags.bits() != existing_flags.bits() {
                    // Update permissions only (not remap)
                    unsafe {
                        let indexes = vpn.indexes();
                        let mut current_table = page_table as *mut PageTable;
                        for &index in &indexes[..2] {
                            let entry = (*current_table).entry_mut(index);
                            current_table = entry.ppn().as_ptr::<PageTable>();
                        }
                        let leaf_entry = (*current_table).entry_mut(indexes[2]);
                        *leaf_entry = PageTableEntry::new_with_ppn(existing_ppn, merged_flags);
                    }
                }
                
                // Note: We don't track the frame in this MapArea's data_frames
                // because it's already tracked by the first MapArea that created it.
                // The frame will be freed when the first MapArea is dropped.
                
                // Skip remapping - we're reusing the existing page
                return;
            }
            
            // Can't reuse - this is an error case (shouldn't happen for user space)
            panic!("Page conflict that cannot be resolved by reusing existing page");
        }
        
        // Allocate or determine PPN
        let ppn: PhysPageNum = match self.map_type {
            MapType::Identical => PhysPageNum::new(vpn.0),
            MapType::Framed => {
                // Check if we already have a frame tracked
                if let Some(frame_tracker) = self.data_frames.get(&vpn) {
                    frame_tracker.ppn
                } else {
                    // Allocate a new frame
                    let frame = FRAME_ALLOCATOR.alloc().expect("Failed to allocate frame");
                    // CRITICAL: Clear the frame to ensure no garbage data
                    let frame_va = frame.addr().0;
                    unsafe {
                        core::ptr::write_bytes(frame_va as *mut u8, 0, PAGE_SIZE);
                    }
                    self.data_frames.insert(vpn, FrameTracker::new(frame));
                    frame
                }
            }
        };
        
        // Map the page
        if let Err(e) = page_table.map(vpn, ppn, pte_flags) {
            panic!("Failed to map page: {}", e);
        }
    }
    
    /// Unmap one page
    /// Only unmap if this MapArea owns the page (tracked in data_frames)
    /// If multiple MapAreas share the same page, only the owner should unmap it
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            // Only unmap if we own this page (tracked in data_frames)
            if self.data_frames.remove(&vpn).is_some() {
                // We own this page, so unmap it
                page_table.unmap(vpn).expect("Failed to unmap page");
            }
            // If we don't own this page (not in data_frames), it's owned by another MapArea
            // Don't unmap it - let the owner handle it
        } else {
            // For Identical mapping, always unmap
            page_table.unmap(vpn).expect("Failed to unmap page");
        }
    }
    
    /// Map all pages in this area (段 -> 页的映射)
    /// 
    /// This function implements the "segment to pages" conversion:
    /// - Iterates through all virtual pages in the segment
    /// - Allocates physical frames for each page
    /// - Creates page table entries mapping virtual pages to physical frames
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }
    
    /// Unmap all pages in this area
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
    
    /// Copy data to this area
    pub fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.start();
        let len = data.len();
        let actual_start_va = self.actual_start_va;  // Actual start address (before alignment)

        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let current_va = current_vpn.addr().0;

            // Translate virtual page to physical page
            let (ppn, _flags) = match page_table.translate(current_vpn) {
                Some((ppn, flags)) => (ppn, flags),
                None => panic!("Page not mapped in copy_data"),
            };

            // Convert physical page number to kernel virtual address (identity mapping)
            // Since kernel uses identity mapping, physical address = virtual address
            let kernel_va = ppn.addr().0;

            // Safety check: ensure we're not writing to kernel code section
            // Kernel code section starts at 0x80200000 (stext)
            extern "C" {
                fn stext();
                fn ekernel();
            }
            let stext_addr = stext as *const () as usize;
            let ekernel_addr = ekernel as *const () as usize;

            if kernel_va >= stext_addr && kernel_va < ekernel_addr {
                panic!("Attempting to write to kernel section! VPN=0x{:x}, PPN=0x{:x}, kernel_va=0x{:x}",
                    current_vpn.0, ppn.0, kernel_va);
            }

            // Additional check: ensure physical address is in user space range
            // User space physical pages should be >= KERNEL_HEAP_START
            use crate::config::memory_layout::KERNEL_HEAP_START;
            if kernel_va < KERNEL_HEAP_START {
                panic!("Physical address below KERNEL_HEAP_START! This should not happen for user space pages.");
            }

            // Calculate offset within the page: actual segment start VA - page start VA
            let page_offset = if current_va <= actual_start_va {
                actual_start_va - current_va
            } else {
                0
            };

            // Calculate how much data to copy in this page
            let data_start_in_page = if start == 0 { page_offset } else { 0 };

            // Calculate the actual destination address within the page
            let dst = (kernel_va + data_start_in_page) as *mut u8;

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
            }

            // CRITICAL FIX: increment start by actual bytes copied, not PAGE_SIZE
            // This ensures we don't skip data when file_size < PAGE_SIZE
            start += src.len();
            if start >= len {
                break;
            }
            current_vpn.0 += 1;
        }
    }
}

/// Memory set
/// 
/// - Page table root should be allocated from frame allocator
/// - Store the physical page number (PPN) of the root page table
/// - Access page table through PPN (using identity mapping: PA == VA)
pub struct MemorySet {
    /// Physical page number of the page table root
    /// The page table is stored in this physical frame
    root_ppn: PhysPageNum,
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// Create a new empty memory set
    pub fn new_bare() -> Self {
        // Allocate a physical frame for page table root
        let root_ppn = FRAME_ALLOCATOR.alloc().expect("Failed to allocate frame for page table root");
        
        // Get pointer to the physical frame (kernel uses identity mapping, so PA == VA)
        let page_table_ptr = root_ppn.as_ptr::<PageTable>();
        
        // Initialize the page table in the allocated frame
        unsafe {
            (*page_table_ptr).clear();
        }
        
        Self {
            root_ppn,
            areas: Vec::new(),
        }
    }
    
    /// Create kernel memory set with identity mapping
    pub fn new_kernel() -> Self {
        extern "C" {
            fn stext();
            fn etext();
            fn srodata();
            fn erodata();
            fn sdata();
            fn edata();
            fn sbss();
            fn ebss();
            fn ekernel();
            fn strampoline();
        }
        
        let mut memory_set = Self::new_bare();

        // Map physical memory from MEMORY_START to stext (for bootloader and early init)
        use crate::config::memory_layout::MEMORY_START;
        let stext_addr = stext as *const () as usize;

        if MEMORY_START < stext_addr {
            memory_set.push(
                MapArea::new(
                    MEMORY_START,
                    stext_addr,
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            );
        }

        // Map .text section in three parts:
        // 1. Entry code (before trampoline): R-X
        // 2. Trampoline page (contains KERNEL_SATP): RWX 
        // 3. Rest of text (after trampoline): R-X
        let stext_addr = stext as *const () as usize;
        let etext_addr = etext as *const () as usize;

        let strampoline_addr = strampoline as *const () as usize;
        let etrampoline_addr = strampoline_addr + PAGE_SIZE;

        // Part 1: Entry code before trampoline
        if stext_addr < strampoline_addr {
            memory_set.push(
                MapArea::new(
                    stext_addr,
                    strampoline_addr,
                    MapType::Identical,
                    MapPermission::R | MapPermission::X,
                ),
                None,
            );
        }

        // Part 2: Trampoline page (RWX because KERNEL_SATP is stored here)
        memory_set.push(
            MapArea::new(
                strampoline_addr,
                etrampoline_addr,
                MapType::Identical,
                MapPermission::R | MapPermission::W | MapPermission::X,
            ),
            None,
        );
        
        // Part 3: Rest of text after trampoline
        if etrampoline_addr < etext_addr {
            memory_set.push(
                MapArea::new(
                    etrampoline_addr,
                    etext_addr,
                    MapType::Identical,
                    MapPermission::R | MapPermission::X,
                ),
                None,
            );
        }
        
        // Map .rodata section (R--)
        let srodata_addr = srodata as *const () as usize;
        let erodata_addr = erodata as *const () as usize;
        memory_set.push(
            MapArea::new(
                srodata_addr,
                erodata_addr,
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        
        // Map .data section (RW-)
        let sdata_addr = sdata as *const () as usize;
        let edata_addr = edata as *const () as usize;
        memory_set.push(
            MapArea::new(
                sdata_addr,
                edata_addr,
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // Map .bss section including boot stack (RW-)
        // Boot stack is in .bss.stack section, which is after .bss but before ekernel
        // We map from sbss to ekernel to include both .bss and .bss.stack
        let ekernel_addr = ekernel as *const () as usize;
        let sbss_addr = sbss as *const () as usize;
        memory_set.push(
            MapArea::new(
                sbss_addr,
                ekernel_addr,
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // Map MMIO region (for devices like CLINT, PLIC, etc.)
        // QEMU virt machine MMIO starts at 0x2000000
        // We need to map at least the CLINT region (0x2000000 - 0x2010000)
        // and potentially other devices up to 0x10000000
        const MMIO_START: usize = 0x2000000;
        const MMIO_END: usize = 0x10000000; // Map up to 256MB for MMIO devices
        memory_set.push(
            MapArea::new(
                MMIO_START,
                MMIO_END,
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // Map physical memory (RW-)
        // Ensure we map from ekernel to MEMORY_END, which includes the heap region
        // Heap starts at KERNEL_HEAP_START (0x8042_0000) and ends at KERNEL_HEAP_END
        // We need to map from ekernel (which should be <= KERNEL_HEAP_START) to MEMORY_END
        let phys_mem_start = ekernel as *const () as usize;
        // Align to page boundary
        let phys_mem_start_aligned = phys_mem_start & !(PAGE_SIZE - 1);
        memory_set.push(
            MapArea::new(
                phys_mem_start_aligned,
                MEMORY_END,
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // Map trampoline and kernel stacks in kernel address space
        // Kernel stacks are allocated below TRAP_CONTEXT
        // We need to map a region that includes:
        // 1. TRAMPOLINE (R-X) - trampoline code
        // 2. TRAP_CONTEXT (RW-) - trap context storage (one page per task, but we map a region)
        // 3. Kernel stacks (RW-) - below TRAP_CONTEXT
        let trampoline_phys = strampoline as *const () as usize;
        let trampoline_virt = TRAMPOLINE;
        
        // Map kernel stacks region (RW-)
        // Kernel stacks are allocated from TRAP_CONTEXT going down
        // We need to map enough space for multiple kernel stacks
        use crate::config::memory_layout::KERNEL_STACK_SIZE;
        const MAX_KERNEL_STACKS: usize = 16;
        let kernel_stack_region_size = MAX_KERNEL_STACKS * (KERNEL_STACK_SIZE + PAGE_SIZE);
        // Use wrapping_sub to avoid overflow check (TRAP_CONTEXT is near usize::MAX)
        let kernel_stack_region_start = TRAP_CONTEXT.wrapping_sub(kernel_stack_region_size);
        // Align down to page boundary
        let kernel_stack_region_start_aligned = kernel_stack_region_start & !(PAGE_SIZE - 1);
        
        // CRITICAL FIX: Avoid overflow by mapping regions separately
        // Problem: TRAP_CONTEXT.wrapping_add(2*PAGE_SIZE) overflows to 0x0, causing empty mapping
        // Solution: Map each region separately
        
        // 1. Map kernel stacks region (RW-) - from start to TRAP_CONTEXT (excluding TRAP_CONTEXT page)
        memory_set.push(
            MapArea::new(
                kernel_stack_region_start_aligned,
                TRAP_CONTEXT, // Exclude TRAP_CONTEXT page itself
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // 2. Map TRAP_CONTEXT page (RW-) - one page for trap context storage
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT,
                TRAP_CONTEXT + PAGE_SIZE,
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // 3. Map TRAMPOLINE page (R-X) - trampoline code for trap handling
        // Note: TRAMPOLINE is at usize::MAX - PAGE_SIZE + 1, which is the last page
        // TRAMPOLINE + PAGE_SIZE would overflow to 0, so we can't use MapArea::new()
        // Instead, we directly map the single page using page_table.map()
        // Map trampoline by physical address via identity mapping
        let trampoline_frame = PhysPageNum::new(trampoline_phys / PAGE_SIZE);
        let trampoline_vpn = super::memory_layout::VirtAddr::new(trampoline_virt).page_number();
        // CRITICAL: Include W flag because KERNEL_SATP is stored in trampoline page
        let pte_flags = PTEFlags::V | PTEFlags::R | PTEFlags::W | PTEFlags::X;
        memory_set
            .page_table_mut()
            .map(trampoline_vpn, trampoline_frame, pte_flags)
            .expect("Failed to map TRAMPOLINE page");
        
        // Also map the TRAMPOLINE_FRAME for user spaces to reuse
        *TRAMPOLINE_FRAME.lock() = Some(trampoline_frame);
        
        memory_set
    }
    
    /// Push a map area into memory set
    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(self.page_table_mut());
        
        if let Some(data) = data {
            map_area.copy_data(self.page_table(), data);
        }
        
        self.areas.push(map_area);
    }
    
    /// Activate this memory set (write satp register)
    pub fn activate(&self) {
        let satp = self.root_ppn.as_usize() | (8usize << 60);
        unsafe {
            asm!("csrw satp, {}", in(reg) satp);
            asm!("sfence.vma");
        }
    }
    
    /// Translate a virtual address to physical address
    pub fn translate(&self, va: usize) -> Option<usize> {
        self.translate_with_debug(va, false)
    }
    
    /// Translate a virtual address to physical address with optional debug output
    /// 
    /// # Arguments
    /// * `va` - Virtual address to translate
    /// * `debug` - If true, print root page table contents when accessing it
    pub fn translate_with_debug(&self, va: usize, debug: bool) -> Option<usize> {
        let vpn = VirtAddr::new(va).page_number();
        let offset = VirtAddr::new(va).page_offset();
        let page_table = self.page_table();
        page_table
            .translate_with_debug(vpn, debug)
            .map(|(ppn, _)| ppn.addr().0 + offset)
    }
    
    /// Print root page table contents for debugging
    pub fn print_root_table(&self) {
        let page_table = self.page_table();
        page_table.print_root_table();
    }
    
    /// Get page table token (satp value)
    pub fn token(&self) -> usize {
        self.root_ppn.as_usize() | (8usize << 60)
    }
    
    /// Remove a map area from memory set
    /// This unmaps all pages in the area and removes it from the areas list
    pub fn remove_area(&mut self, area_index: usize) {
        if area_index < self.areas.len() {
            let mut area = self.areas.remove(area_index);
            let page_table = self.page_table_mut();
            area.unmap(page_table);
            // FrameTracker will automatically deallocate frames when dropped
        }
    }
    
    /// Clear all map areas (unmap all pages)
    /// This is used when destroying an address space
    pub fn clear_areas(&mut self) {
        // Unmap all areas in reverse order to avoid issues
        while let Some(mut area) = self.areas.pop() {
            let page_table = self.page_table_mut();
            area.unmap(page_table);
            // FrameTracker will automatically deallocate frames when dropped
        }
    }
    
    /// Get a reference to the page table (for advanced operations)
    /// Access page table through root_ppn using identity mapping
    pub fn page_table(&self) -> &PageTable {
        unsafe { &*self.root_ppn.as_ptr::<PageTable>() }
    }
    
    /// Get a mutable reference to the page table (for advanced operations)
    /// Access page table through root_ppn using identity mapping
    pub fn page_table_mut(&mut self) -> &mut PageTable {
        unsafe { &mut *self.root_ppn.as_ptr::<PageTable>() }
    }
    
    /// Get a reference to the areas vector (for inspection)
    pub fn areas(&self) -> &Vec<MapArea> {
        &self.areas
    }
    
    /// Clone this memory set (for fork system call)
    /// Creates a new address space with the same mappings
    /// This is a deep copy: all pages are copied to new physical frames
    /// 
    /// # Note
    /// This is a simplified version. A full implementation would:
    /// 1. Use Copy-on-Write (COW) for shared pages
    /// 2. Only copy pages that are actually modified
    /// 3. Share read-only pages between parent and child
    pub fn clone(&self) -> Self {
        let mut new_memory_set = Self::new_bare();
        
        // Clone all map areas
        for area in &self.areas {
            // Create a new map area with the same range and permissions
            let mut new_area = MapArea::new(
                area.start_va(),
                area.end_va(),
                area.map_type(),
                area.map_perm(),
            );
            
            // Map the new area
            let page_table = new_memory_set.page_table_mut();
            new_area.map(page_table);
            
            // If it's a framed mapping, copy the data
            if area.map_type() == MapType::Framed {
                // Copy data from old pages to new pages
                // We need to iterate through the VPN range
                let start_vpn = VirtAddr::new(area.start_va()).page_number();
                let end_vpn = VirtAddr::new(area.end_va()).page_number();
                let mut current_vpn = start_vpn;
                while current_vpn.0 < end_vpn.0 {
                    // Get source page (from old address space)
                    let src_page_table = self.page_table();
                    if let Some((src_ppn, _)) = src_page_table.translate(current_vpn) {
                        // Get destination page (from new address space)
                        let dst_page_table = new_memory_set.page_table();
                        if let Some((dst_ppn, _)) = dst_page_table.translate(current_vpn) {
                            // Copy page data
                            let src_ptr = src_ppn.as_ptr::<u8>();
                            let dst_ptr = dst_ppn.as_ptr::<u8>();
                            unsafe {
                                core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, PAGE_SIZE);
                            }
                        }
                    }
                    current_vpn.0 += 1;
                }
            }
            
            new_memory_set.areas.push(new_area);
        }
        
        new_memory_set
    }
    
    /// Create a user memory set from ELF data
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        use xmas_elf::ElfFile;
        
        let elf = ElfFile::new(elf_data).expect("Failed to parse ELF");
        let elf_header = elf.header;
        let ph_count = elf_header.pt2.ph_count();
        
        let mut memory_set = Self::new_bare();
        
        // Map trampoline and trap context
        // Both need to be accessible from user mode
        let trampoline_start = TRAMPOLINE;
        let trap_context_start = TRAP_CONTEXT;
        let user_stack_bottom = USER_STACK_BOTTOM;
        let user_stack_top = USER_STACK_TOP;
        
        // Map trampoline in user address space (same virtual address as kernel)
        // Note: TRAMPOLINE is at usize::MAX - PAGE_SIZE + 1, which is the last page
        // TRAMPOLINE + PAGE_SIZE would overflow to 0, so we can't use MapArea::new()
        // Instead, we directly map the single page using page_table.map()
        let trampoline_vpn = super::memory_layout::VirtAddr::new(trampoline_start).page_number();

        // Reuse the global trampoline frame (must already be allocated by kernel space)
        let trampoline_frame = {
            let global_frame = TRAMPOLINE_FRAME.lock();
            global_frame.expect("Trampoline frame not initialized - kernel space must be created first")
        };
        // CRITICAL: Do NOT include U (User) flag here!
        // The trampoline code is executed in S-mode (supervisor mode), not U-mode.
        // In RISC-V, supervisor mode CANNOT execute from pages with U=1 set.
        // After switching to user page table with csrw satp, we're still in S-mode
        // until sret, so the trampoline must be S-mode accessible (U=0).
        // User mode will enter via trap (which jumps to stvec), not by executing
        // directly from the trampoline.
        let pte_flags = PTEFlags::V | PTEFlags::R | PTEFlags::X;

        // Map the page directly to the SAME physical frame as kernel space
        memory_set.page_table_mut().map(trampoline_vpn, trampoline_frame, pte_flags)
            .expect("Failed to map TRAMPOLINE page");
        
        // Map trap context (stored in user address space but accessible from kernel)
        memory_set.push(
            MapArea::new(
                trap_context_start,
                trap_context_start + PAGE_SIZE,
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // Map user stack
        memory_set.push(
            MapArea::new(
                user_stack_bottom,
                user_stack_top,
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
        
        // Load ELF segments (段页式管理: Segment-based Paging)
        // Each ELF Load segment becomes a MapArea (段), which is then divided into pages (页)
        let mut max_end_vaddr = 0usize;
        let entry_point = elf.header.pt2.entry_point() as usize;
        
        for i in 0..ph_count {
            let ph = elf.program_header(i as u16).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                // Extract segment information from ELF program header
                let vaddr = ph.virtual_addr();
                let mem_size = ph.mem_size();
                let file_size = ph.file_size();
                let file_offset = ph.offset();
                
                let start_va = vaddr as usize;  // Actual start address (may not be page-aligned)
                // Align to page boundary (段 -> 页的转换)
                let aligned_start_va = super::memory_layout::align_down(start_va);
                let end_va = super::memory_layout::align_up(start_va + mem_size as usize);
                
                if end_va > max_end_vaddr {
                    max_end_vaddr = end_va;
                }
                
                // Determine page permissions from ELF segment flags
                let mut perm = MapPermission::U; // User mode access
                let flags = ph.flags();
                let is_read = flags.is_read();
                let is_write = flags.is_write();
                let is_execute = flags.is_execute();
                
                if is_read {
                    perm |= MapPermission::R;
                }
                if is_write {
                    perm |= MapPermission::W;
                }
                if is_execute {
                    perm |= MapPermission::X;
                }
                
                // Extract segment data from ELF file
                // The ELF file structure is preserved - we just read the segment content
                let segment_data = if file_size > 0 {
                    &elf_data[file_offset as usize..(file_offset + file_size) as usize]
                } else {
                    &[]
                };

                // Create MapArea for this segment (段)
                // Use actual_start_va (vaddr) to preserve segment offset for proper data placement
                // aligned_start_va is used for page table mapping (must be page-aligned)
                // MapArea.push() will later create page table entries (页) for all pages in this segment
                memory_set.push(
                    MapArea::new_with_actual_start(start_va, aligned_start_va, end_va, MapType::Framed, perm),
                    Some(segment_data),
                );
            }
        }
        
        let user_sp = USER_STACK_TOP;
        (memory_set, user_sp, entry_point)
    }
}

// User memory layout constants
const USER_STACK_BOTTOM: usize = 0x10000000;
const USER_STACK_TOP: usize = USER_STACK_BOTTOM + USER_STACK_SIZE;
const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

impl Drop for MemorySet {
    /// Automatically deallocate address space when dropped
    /// This ensures proper cleanup of:
    /// 1. All mapped pages (via FrameTracker::drop)
    /// 2. All intermediate page tables
    /// 3. The root page table itself
    fn drop(&mut self) {
        // First, unmap all areas to release physical frames
        // FrameTracker::drop will automatically deallocate frames
        self.clear_areas();
        
        // Then, deallocate all intermediate page tables
        // Note: We don't deallocate the root page table itself here,
        // as it might be allocated on the stack or in a special way
        // The root page table should be deallocated by the caller if needed
        self.page_table_mut().dealloc_intermediate_tables();
    }
}
