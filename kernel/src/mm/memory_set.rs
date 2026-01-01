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
use super::page_table::{PTEFlags, PageTable};
use crate::config::memory_layout::{PAGE_SIZE, MEMORY_END, USER_STACK_SIZE};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::arch::asm;

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
        let start_vpn = VirtAddr::new(start_va).page_number();
        let end_vpn = VirtAddr::new(end_va).page_number();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
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
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        
        // Check if page is already mapped
        if let Some((existing_ppn, existing_flags)) = page_table.translate(vpn) {
            // Determine expected PPN based on map type
            let expected_ppn = match self.map_type {
                MapType::Identical => PhysPageNum::new(vpn.0),
                MapType::Framed => {
                    // For Framed mapping, check if we already have a frame tracked
                    if let Some(frame_tracker) = self.data_frames.get(&vpn) {
                        frame_tracker.ppn
                    } else {
                        // No frame tracked yet, allocate a new one
                        let frame = FRAME_ALLOCATOR.alloc().expect("Failed to allocate frame");
                        self.data_frames.insert(vpn, FrameTracker::new(frame));
                        frame
                    }
                }
            };
            
            // Check if existing mapping matches what we want
            if existing_ppn == expected_ppn && existing_flags.bits() == pte_flags.bits() {
                // Same mapping, skip silently
                return;
            }
            
            // Different mapping - this should not happen in normal flow
            // For user address spaces, we should unmap and remap to ensure consistency
            let va = vpn.addr().0;
            crate::sbi::console_putstr("[WARNING] Page conflict: VA=0x");
            crate::trap::print_hex_usize(va);
            crate::sbi::console_putstr(", existing PPN=0x");
            crate::trap::print_hex_usize(existing_ppn.0);
            crate::sbi::console_putstr(", expected PPN=0x");
            crate::trap::print_hex_usize(expected_ppn.0);
            crate::sbi::console_putstr("\n");
            
            // Unmap the existing page and remap with the correct PPN
            // This ensures copy_data() will access the correct physical address
            if let Err(e) = page_table.unmap(vpn) {
                crate::sbi::console_putstr("[ERROR] Failed to unmap conflicting page: ");
                crate::sbi::console_putstr(e);
                crate::sbi::console_putstr("\n");
                panic!("Failed to unmap conflicting page: {}", e);
            }
            // Continue to map with the expected PPN below
        }
        
        // Allocate or determine PPN
        let ppn: PhysPageNum = match self.map_type {
            MapType::Identical => PhysPageNum::new(vpn.0),
            MapType::Framed => {
                // Check if we already have a frame tracked (from the check above)
                if let Some(frame_tracker) = self.data_frames.get(&vpn) {
                    frame_tracker.ppn
                } else {
                    // Allocate a new frame
                let frame = FRAME_ALLOCATOR.alloc().expect("Failed to allocate frame");
                self.data_frames.insert(vpn, FrameTracker::new(frame));
                    frame
                }
            }
        };
        
        // Map the page
        if let Err(e) = page_table.map(vpn, ppn, pte_flags) {
            let va = vpn.addr().0;
            crate::sbi::console_putstr("[ERROR] Failed to map page: VA=0x");
            crate::trap::print_hex_usize(va);
            crate::sbi::console_putstr(", PPN=0x");
            crate::trap::print_hex_usize(ppn.0);
            crate::sbi::console_putstr(", Error: ");
            crate::sbi::console_putstr(e);
            crate::sbi::console_putstr("\n");
            panic!("Failed to map page: {}", e);
        }
    }
    
    /// Unmap one page
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn).expect("Failed to unmap page");
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
        let start_va = self.start_va();
        let end_va = self.end_va();
        
        crate::sbi::console_putstr("[copy_data] Starting: VA=0x");
        crate::trap::print_hex_usize(start_va);
        crate::sbi::console_putstr("-0x");
        crate::trap::print_hex_usize(end_va);
        crate::sbi::console_putstr(", data_len=");
        crate::trap::print_hex_usize(len);
        crate::sbi::console_putstr("\n");
        
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let current_va = current_vpn.addr().0;
            
            crate::sbi::console_putstr("[copy_data] Processing page: VPN=0x");
            crate::trap::print_hex_usize(current_vpn.0);
            crate::sbi::console_putstr(", VA=0x");
            crate::trap::print_hex_usize(current_va);
            crate::sbi::console_putstr(", data_offset=");
            crate::trap::print_hex_usize(start);
            crate::sbi::console_putstr(", src_len=");
            crate::trap::print_hex_usize(src.len());
            crate::sbi::console_putstr("\n");
            
            // Translate virtual page to physical page
            let (ppn, _flags) = match page_table.translate(current_vpn) {
                Some((ppn, flags)) => {
                    crate::sbi::console_putstr("[copy_data] Translation: VPN=0x");
                    crate::trap::print_hex_usize(current_vpn.0);
                    crate::sbi::console_putstr(" -> PPN=0x");
                    crate::trap::print_hex_usize(ppn.0);
                    crate::sbi::console_putstr("\n");
                    (ppn, flags)
                },
                None => {
                    crate::sbi::console_putstr("[copy_data] ERROR: Page not mapped: VPN=0x");
                    crate::trap::print_hex_usize(current_vpn.0);
                    crate::sbi::console_putstr(", VA=0x");
                    crate::trap::print_hex_usize(current_va);
                    crate::sbi::console_putstr("\n");
                    panic!("Page not mapped in copy_data");
                }
            };
            
            // Convert physical page number to kernel virtual address (identity mapping)
            // Since kernel uses identity mapping, physical address = virtual address
            let kernel_va = ppn.addr().0;
            
            crate::sbi::console_putstr("[copy_data] PPN=0x");
            crate::trap::print_hex_usize(ppn.0);
            crate::sbi::console_putstr(" -> PA=0x");
            crate::trap::print_hex_usize(kernel_va);
            crate::sbi::console_putstr(" (kernel_va)\n");
            
            // Check if this physical page is tracked in data_frames
            if let Some(frame_tracker) = self.data_frames.get(&current_vpn) {
                let tracked_ppn = frame_tracker.ppn;
                if tracked_ppn != ppn {
                    crate::sbi::console_putstr("[copy_data] WARNING: data_frames mismatch! VPN=0x");
                    crate::trap::print_hex_usize(current_vpn.0);
                    crate::sbi::console_putstr(", tracked PPN=0x");
                    crate::trap::print_hex_usize(tracked_ppn.0);
                    crate::sbi::console_putstr(", page_table PPN=0x");
                    crate::trap::print_hex_usize(ppn.0);
                    crate::sbi::console_putstr("\n");
                    crate::sbi::console_putstr("[copy_data] Using page_table PPN (correct)\n");
                }
            } else {
                crate::sbi::console_putstr("[copy_data] INFO: VPN=0x");
                crate::trap::print_hex_usize(current_vpn.0);
                crate::sbi::console_putstr(" not in data_frames (may be from another MapArea)\n");
            }
            
            // Safety check: ensure we're not writing to kernel code section
            // Kernel code section starts at 0x80200000 (stext)
            extern "C" {
                fn stext();
                fn ekernel();
            }
            let stext_addr = stext as *const () as usize;
            let ekernel_addr = ekernel as *const () as usize;
            
            crate::sbi::console_putstr("[copy_data] Safety check: kernel_va=0x");
            crate::trap::print_hex_usize(kernel_va);
            crate::sbi::console_putstr(", stext=0x");
            crate::trap::print_hex_usize(stext_addr);
            crate::sbi::console_putstr(", ekernel=0x");
            crate::trap::print_hex_usize(ekernel_addr);
            crate::sbi::console_putstr("\n");
            
            if kernel_va >= stext_addr && kernel_va < ekernel_addr {
                crate::sbi::console_putstr("[copy_data] ERROR: Attempting to write to kernel section!\n");
                crate::sbi::console_putstr("[copy_data] ERROR: VPN=0x");
                crate::trap::print_hex_usize(current_vpn.0);
                crate::sbi::console_putstr(", VA=0x");
                crate::trap::print_hex_usize(current_va);
                crate::sbi::console_putstr(", PPN=0x");
                crate::trap::print_hex_usize(ppn.0);
                crate::sbi::console_putstr(", kernel_va=0x");
                crate::trap::print_hex_usize(kernel_va);
                crate::sbi::console_putstr("\n");
                crate::sbi::console_putstr("[copy_data] ERROR: This indicates a serious bug in page allocation or mapping!\n");
                panic!("Attempting to write to kernel section! VPN=0x{:x}, PPN=0x{:x}, kernel_va=0x{:x}", 
                    current_vpn.0, ppn.0, kernel_va);
            }
            
            // Additional check: ensure physical address is in user space range
            // User space physical pages should be >= KERNEL_HEAP_START
            use crate::config::memory_layout::KERNEL_HEAP_START;
            if kernel_va < KERNEL_HEAP_START {
                crate::sbi::console_putstr("[copy_data] ERROR: Physical address is below KERNEL_HEAP_START!\n");
                crate::sbi::console_putstr("[copy_data] ERROR: kernel_va=0x");
                crate::trap::print_hex_usize(kernel_va);
                crate::sbi::console_putstr(", KERNEL_HEAP_START=0x");
                crate::trap::print_hex_usize(KERNEL_HEAP_START);
                crate::sbi::console_putstr("\n");
                panic!("Physical address below KERNEL_HEAP_START! This should not happen for user space pages.");
            }
            
            let dst = kernel_va as *mut u8;
            
            crate::sbi::console_putstr("[copy_data] Copying ");
            crate::trap::print_hex_usize(src.len());
            crate::sbi::console_putstr(" bytes to kernel_va=0x");
            crate::trap::print_hex_usize(kernel_va);
            crate::sbi::console_putstr("\n");
            
            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
            }
            
            crate::sbi::console_putstr("[copy_data] Copy completed for VPN=0x");
            crate::trap::print_hex_usize(current_vpn.0);
            crate::sbi::console_putstr("\n");
            
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.0 += 1;
        }
        
        crate::sbi::console_putstr("[copy_data] All data copied successfully\n");
    }
}

/// Memory set
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// Create a new empty memory set
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
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
        }
        
        crate::sbi::console_putstr("[MemorySet::new_kernel] Creating kernel address space\n");
        let mut memory_set = Self::new_bare();
        
        // Map physical memory from MEMORY_START to stext (for bootloader and early init)
        // This ensures we can access memory before kernel code starts
        use crate::config::memory_layout::MEMORY_START;
        let stext_addr = stext as *const () as usize;
        crate::sbi::console_putstr("[MemorySet::new_kernel] MEMORY_START=0x");
        crate::trap::print_hex_usize(MEMORY_START);
        crate::sbi::console_putstr(", stext=0x");
        crate::trap::print_hex_usize(stext_addr);
        crate::sbi::console_putstr("\n");
        
        if MEMORY_START < stext_addr {
            crate::sbi::console_putstr("[MemorySet::new_kernel] Step 1: Mapping pre-kernel memory\n");
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
        
        // Map .text section (R-X)
        let stext_addr = stext as *const () as usize;
        let etext_addr = etext as *const () as usize;
        crate::sbi::console_putstr("[MemorySet::new_kernel] Step 2: Mapping .text section: 0x");
        crate::trap::print_hex_usize(stext_addr);
        crate::sbi::console_putstr(" - 0x");
        crate::trap::print_hex_usize(etext_addr);
        crate::sbi::console_putstr(" (R-X)\n");
        memory_set.push(
            MapArea::new(
                stext_addr,
                etext_addr,
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        
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
        extern "C" {
            fn __alltraps();
        }
        let trampoline_phys = __alltraps as *const () as usize;
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
        let trampoline_vpn = super::memory_layout::VirtAddr::new(trampoline_virt).page_number();
        
        // Allocate a frame for trampoline
        let trampoline_frame = FRAME_ALLOCATOR.alloc().expect("Failed to allocate frame for trampoline");
        let pte_flags = PTEFlags::V | PTEFlags::R | PTEFlags::X; // R-X for trampoline code
        
        // Map the page directly
        memory_set.page_table.map(trampoline_vpn, trampoline_frame, pte_flags)
            .expect("Failed to map TRAMPOLINE page");
        
        // Copy trampoline code to the mapped page
        // Convert physical page number to kernel virtual address (identity mapping)
        let trampoline_dst_va = trampoline_frame.addr().0;
        let trampoline_dst = trampoline_dst_va as *mut u8;
        
        // trampoline_src is already a kernel virtual address (identity mapped)
        let trampoline_src = trampoline_phys as *const u8;
        
        unsafe {
            // Clear the page first
            core::ptr::write_bytes(trampoline_dst, 0, PAGE_SIZE);
            // Copy trampoline code (trampoline is small, less than PAGE_SIZE)
            let trampoline_size = PAGE_SIZE; // Copy full page to be safe
            core::ptr::copy_nonoverlapping(
                trampoline_src,
                trampoline_dst,
                trampoline_size,
            );
        }
        
        memory_set
    }
    
    /// Push a map area into memory set
    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        let start_va = map_area.start_va();
        let end_va = map_area.end_va();
        let area_type = match map_area.map_type() {
            MapType::Identical => "Identical",
            MapType::Framed => "Framed",
        };
        crate::sbi::console_putstr("[MapArea] VA=0x");
        crate::trap::print_hex_usize(start_va);
        crate::sbi::console_putstr("-0x");
        crate::trap::print_hex_usize(end_va);
        crate::sbi::console_putstr(", type=");
        crate::sbi::console_putstr(area_type);
        if let Some(d) = data {
            crate::sbi::console_putstr(", data=");
            crate::trap::print_hex_usize(d.len());
            crate::sbi::console_putstr(" bytes");
        }
        crate::sbi::console_putstr("\n");
        
        map_area.map(&mut self.page_table);
        
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        
        self.areas.push(map_area);
    }
    
    /// Activate this memory set (write satp register)
    pub fn activate(&self) {
        let satp = self.page_table.as_ppn().as_usize() | (8usize << 60);
        unsafe {
            asm!("csrw satp, {}", in(reg) satp);
            asm!("sfence.vma");
        }
    }
    
    /// Translate a virtual address to physical address
    pub fn translate(&self, va: usize) -> Option<usize> {
        let vpn = VirtAddr::new(va).page_number();
        let offset = VirtAddr::new(va).page_offset();
        self.page_table
            .translate(vpn)
            .map(|(ppn, _)| ppn.addr().0 + offset)
    }
    
    /// Get page table token (satp value)
    pub fn token(&self) -> usize {
        self.page_table.as_ppn().as_usize() | (8usize << 60)
    }
    
    /// Remove a map area from memory set
    /// This unmaps all pages in the area and removes it from the areas list
    pub fn remove_area(&mut self, area_index: usize) {
        if area_index < self.areas.len() {
            let mut area = self.areas.remove(area_index);
            area.unmap(&mut self.page_table);
            // FrameTracker will automatically deallocate frames when dropped
        }
    }
    
    /// Clear all map areas (unmap all pages)
    /// This is used when destroying an address space
    pub fn clear_areas(&mut self) {
        // Unmap all areas in reverse order to avoid issues
        while let Some(mut area) = self.areas.pop() {
            area.unmap(&mut self.page_table);
            // FrameTracker will automatically deallocate frames when dropped
        }
    }
    
    /// Get a reference to the page table (for advanced operations)
    pub fn page_table(&self) -> &PageTable {
        &self.page_table
    }
    
    /// Get a mutable reference to the page table (for advanced operations)
    pub fn page_table_mut(&mut self) -> &mut PageTable {
        &mut self.page_table
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
            new_area.map(&mut new_memory_set.page_table);
            
            // If it's a framed mapping, copy the data
            if area.map_type() == MapType::Framed {
                // Copy data from old pages to new pages
                // We need to iterate through the VPN range
                let start_vpn = VirtAddr::new(area.start_va()).page_number();
                let end_vpn = VirtAddr::new(area.end_va()).page_number();
                let mut current_vpn = start_vpn;
                while current_vpn.0 < end_vpn.0 {
                    // Get source page (from old address space)
                    if let Some((src_ppn, _)) = self.page_table.translate(current_vpn) {
                        // Get destination page (from new address space)
                        if let Some((dst_ppn, _)) = new_memory_set.page_table.translate(current_vpn) {
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
        
        crate::println!("[MemorySet] Parsing ELF file ({} bytes)", elf_data.len());
        let elf = ElfFile::new(elf_data).expect("Failed to parse ELF");
        crate::println!("[MemorySet] ELF parsed successfully");
        let elf_header = elf.header;
        let ph_count = elf_header.pt2.ph_count();
        
        let mut memory_set = Self::new_bare();
        
        // Map trampoline and trap context
        // Both need to be accessible from user mode
        let trampoline_start = TRAMPOLINE;
        let trap_context_start = TRAP_CONTEXT;
        let user_stack_bottom = USER_STACK_BOTTOM;
        let user_stack_top = USER_STACK_TOP;
        
        // Get trampoline's physical address from kernel space
        extern "C" {
            fn __alltraps();
        }
        let trampoline_phys = __alltraps as *const () as usize;
        
        // Map trampoline in user address space (same virtual address as kernel)
        // Note: TRAMPOLINE is at usize::MAX - PAGE_SIZE + 1, which is the last page
        // TRAMPOLINE + PAGE_SIZE would overflow to 0, so we can't use MapArea::new()
        // Instead, we directly map the single page using page_table.map()
        let trampoline_vpn = super::memory_layout::VirtAddr::new(trampoline_start).page_number();
        
        // Allocate a frame for trampoline
        let trampoline_frame = FRAME_ALLOCATOR.alloc().expect("Failed to allocate frame for trampoline");
        let pte_flags = PTEFlags::V | PTEFlags::R | PTEFlags::X; // R-X for trampoline code
        
        // Map the page directly
        memory_set.page_table.map(trampoline_vpn, trampoline_frame, pte_flags)
            .expect("Failed to map TRAMPOLINE page");
        
        // Copy trampoline code to the mapped page
        // Convert physical page number to kernel virtual address (identity mapping)
        let trampoline_dst_va = trampoline_frame.addr().0;
        let trampoline_dst = trampoline_dst_va as *mut u8;
        
        // trampoline_src is already a kernel virtual address (identity mapped)
        let trampoline_src = trampoline_phys as *const u8;
        
        unsafe {
            // Clear the page first
            core::ptr::write_bytes(trampoline_dst, 0, PAGE_SIZE);
            // Copy trampoline code (trampoline is small, less than PAGE_SIZE)
            let trampoline_size = PAGE_SIZE; // Copy full page to be safe
            core::ptr::copy_nonoverlapping(
                trampoline_src,
                trampoline_dst,
                trampoline_size,
            );
        }
        
        // Add a debug output for the mapped trampoline
        crate::sbi::console_putstr("[MapArea] VA=0x");
        crate::trap::print_hex_usize(trampoline_start);
        crate::sbi::console_putstr(" (TRAMPOLINE), type=Framed, R-X\n");
        
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
                
                let start_va = vaddr as usize;
                // Align to page boundary (段 -> 页的转换)
                let end_va = super::memory_layout::align_up(start_va + mem_size as usize);
                
                if end_va > max_end_vaddr {
                    max_end_vaddr = end_va;
                }
                
                // Determine page permissions from ELF segment flags
                let mut perm = MapPermission::U; // User mode access
                let flags = ph.flags();
                if flags.is_read() {
                    perm |= MapPermission::R;
                }
                if flags.is_write() {
                    perm |= MapPermission::W;
                }
                if flags.is_execute() {
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
                // MapArea.push() will later create page table entries (页) for all pages in this segment
                memory_set.push(
                    MapArea::new(start_va, end_va, MapType::Framed, perm),
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
        self.page_table.dealloc_intermediate_tables();
    }
}
