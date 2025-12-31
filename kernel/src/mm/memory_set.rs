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
        let bytes_array = ppn.as_ptr::<u8>();
        for i in 0..PAGE_SIZE {
            unsafe {
                *bytes_array.add(i) = 0;
            }
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
    
    /// Map one page
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum::new(vpn.0);
            }
            MapType::Framed => {
                let frame = FRAME_ALLOCATOR.alloc().expect("Failed to allocate frame");
                ppn = frame;
                self.data_frames.insert(vpn, FrameTracker::new(frame));
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        // Check if page is already mapped
        if let Some((existing_ppn, existing_flags)) = page_table.translate(vpn) {
            // Page is already mapped - this might be intentional (e.g., trampoline shared across tasks)
            // But for user address spaces, each should have its own mapping
            // For now, we'll skip remapping if it's already mapped with the same PPN and flags
            if existing_ppn == ppn && existing_flags.bits() == pte_flags.bits() {
                // Same mapping, skip
                return;
            } else {
                // Different mapping, this is an error
                crate::sbi::console_putstr("[MapArea] WARNING: Page already mapped with different PPN/flags: VPN=0x");
                crate::trap::print_hex_usize(vpn.0);
                crate::sbi::console_putstr(", existing PPN=0x");
                crate::trap::print_hex_usize(existing_ppn.0);
                crate::sbi::console_putstr(", new PPN=0x");
                crate::trap::print_hex_usize(ppn.0);
                crate::sbi::console_putstr("\n");
                // For user address spaces, we should unmap first, then remap
                // But for now, we'll just skip
                return;
            }
        }
        // Page is not mapped, proceed with mapping
        if let Err(e) = page_table.map(vpn, ppn, pte_flags) {
            crate::sbi::console_putstr("[MapArea] ERROR: Failed to map page: VPN=0x");
            crate::trap::print_hex_usize(vpn.0);
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
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = page_table
                .translate(current_vpn)
                .unwrap()
                .0
                .as_ptr::<u8>();
            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
            }
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.0 += 1;
        }
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
        
        let mut memory_set = Self::new_bare();
        
        // Map physical memory from MEMORY_START to stext (for bootloader and early init)
        // This ensures we can access memory before kernel code starts
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
        
        // Map .text section (R-X)
        let stext_addr = stext as *const () as usize;
        let etext_addr = etext as *const () as usize;
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
        
        // Map kernel stacks region (RW-) - includes TRAP_CONTEXT area
        // This region will be used for kernel stacks and trap contexts
        // Note: TRAMPOLINE is at usize::MAX - PAGE_SIZE + 1
        // We need to map up to TRAMPOLINE + PAGE_SIZE, but that would overflow
        // Instead, we map up to TRAP_CONTEXT + PAGE_SIZE (which includes TRAP_CONTEXT page)
        // TRAP_CONTEXT = TRAMPOLINE - PAGE_SIZE, so TRAP_CONTEXT + PAGE_SIZE = TRAMPOLINE
        // But we also need to include the TRAMPOLINE page itself, so we map to TRAMPOLINE + PAGE_SIZE
        // Since TRAMPOLINE + PAGE_SIZE would overflow, we use usize::MAX + 1 which wraps to 0
        // Actually, we should map to the end of the address space
        // For 64-bit RISC-V, the maximum virtual address is 0xFFFFFFFFFFFFFFFF (usize::MAX)
        // So we map from kernel_stack_region_start_aligned to usize::MAX + 1 (which wraps to 0, but we'll use a different approach)
        // Actually, let's just map to TRAP_CONTEXT + 2*PAGE_SIZE to include both TRAP_CONTEXT and TRAMPOLINE
        let region_end = TRAP_CONTEXT.wrapping_add(2 * PAGE_SIZE);
        memory_set.push(
            MapArea::new(
                kernel_stack_region_start_aligned,
                region_end, // Include TRAP_CONTEXT and TRAMPOLINE pages
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        
        // Map trampoline page (R-X) - it's already included in the region above, but we need X permission
        let trampoline_vpn = super::memory_layout::VirtAddr::new(trampoline_virt).page_number();
        
        // Copy trampoline code to the mapped page
        if let Some((mapped_ppn, _)) = memory_set.page_table.translate(trampoline_vpn) {
            let trampoline_dst = mapped_ppn.as_ptr::<u8>();
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
            
            // Now we need to change the page permission to R-X
            // We'll do this by directly modifying the page table entry
            use super::page_table::{PTEFlags, PageTableEntry};
            if let Some(leaf_entry) = unsafe { memory_set.page_table.get_pte_mut(trampoline_vpn) } {
                if leaf_entry.is_valid() {
                    // Reconstruct the PTE with R-X permissions instead of RW-
                    let ppn = leaf_entry.ppn();
                    *leaf_entry = PageTableEntry::new_with_ppn(
                        ppn,
                        PTEFlags::V | PTEFlags::R | PTEFlags::X,
                    );
                }
            }
        }
        
        memory_set
    }
    
    /// Push a map area into memory set
    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
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
        // We need to copy trampoline code to a frame and map it
        // Note: TRAMPOLINE + PAGE_SIZE would overflow, so we use wrapping_add
        let trampoline_end = trampoline_start.wrapping_add(PAGE_SIZE);
        memory_set.push(
            MapArea::new(
                trampoline_start,
                trampoline_end,
                MapType::Framed,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        
        // Copy trampoline code to the mapped frame
        let trampoline_vpn = super::memory_layout::VirtAddr::new(trampoline_start).page_number();
        if let Some((trampoline_ppn, _)) = memory_set.page_table.translate(trampoline_vpn) {
            let trampoline_dst = trampoline_ppn.as_ptr::<u8>();
            let trampoline_src = trampoline_phys as *const u8;
            unsafe {
                // Clear the page first
                core::ptr::write_bytes(trampoline_dst, 0, PAGE_SIZE);
                // Copy trampoline code (assume it's less than PAGE_SIZE)
                let trampoline_size = 4096; // Should be enough for trampoline
                core::ptr::copy_nonoverlapping(
                    trampoline_src,
                    trampoline_dst,
                    trampoline_size.min(PAGE_SIZE),
                );
            }
        }
        
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
