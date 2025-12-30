//! Memory Set Management
//!
//! Manages virtual memory spaces for kernel and user processes

use super::frame_allocator::{FrameAllocator, FRAME_ALLOCATOR};
use super::memory_layout::*;
use super::page_table::{PTEFlags, PageTable, PageTableEntry};
use crate::config::{
    memory_layout::{MEMORY_END, PAGE_SIZE, PAGE_SIZE_BITS, USER_STACK_SIZE},
    TRAMPOLINE, TRAP_CONTEXT, USER_STACK_TOP,
};
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
    #[derive(Debug, Clone, Copy)]
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
    pub fn new(start_va: usize, end_va: usize, map_type: MapType, map_perm: MapPermission) -> Self {
        let start_vpn = VirtAddr::new(start_va).page_number();
        let mut end_vpn = VirtAddr::new(end_va).page_number();
        if VirtAddr::new(end_va).page_offset() != 0 {
            end_vpn.0 += 1;
        }
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
        let mut flags = PTEFlags::V | PTEFlags::A;
        if self.map_perm.contains(MapPermission::R) {
            flags |= PTEFlags::R;
        }
        if self.map_perm.contains(MapPermission::W) {
            flags |= PTEFlags::W | PTEFlags::D; // Assuming W implies D (dirty) for simplicity in this context, or it will be set by HW
        }
        if self.map_perm.contains(MapPermission::X) {
            flags |= PTEFlags::X;
        }
        if self.map_perm.contains(MapPermission::U) {
            flags |= PTEFlags::U;
        }
        let pte_flags = flags;
        //         let mut flags = PTEFlags::V | PTEFlags::A;
        if self.map_perm.contains(MapPermission::R) {
            flags |= PTEFlags::R;
        }
        if self.map_perm.contains(MapPermission::W) {
            flags |= PTEFlags::W | PTEFlags::D; // Assuming W implies D (dirty) for simplicity in this context, or it will be set by HW
        }
        if self.map_perm.contains(MapPermission::X) {
            flags |= PTEFlags::X;
        }
        if self.map_perm.contains(MapPermission::U) {
            flags |= PTEFlags::U;
        }
        let pte_flags = flags;
        // let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        page_table
            .map(vpn, ppn, pte_flags)
            .expect("Failed to map page");
    }

    /// Unmap one page
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn).expect("Failed to unmap page");
    }

    /// Map all pages in this area
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
            let dst_pa = page_table.translate(current_vpn);
            if dst_pa.is_none() {
                panic!(
                    "copy_data: VPN {:?} not mapped! range: {:?}..{:?}",
                    current_vpn,
                    self.vpn_range.start(),
                    self.vpn_range.end()
                );
            }
            let dst = dst_pa.unwrap().0.as_ptr::<u8>();
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

        // Map .text section (R-X)
        memory_set.push(
            MapArea::new(
                stext as usize,
                etext as usize,
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );

        // Map .rodata section (R--)
        memory_set.push(
            MapArea::new(
                srodata as usize,
                erodata as usize,
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );

        // Map .data section (RW-)
        memory_set.push(
            MapArea::new(
                sdata as usize,
                edata as usize,
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        // Map .bss section (RW-)
        memory_set.push(
            MapArea::new(
                sbss as usize,
                ebss as usize,
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        // Map physical memory (RW-)
        memory_set.push(
            MapArea::new(
                ekernel as usize,
                MEMORY_END,
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        // Map trampoline
        extern "C" {
            fn strampoline();
        }
        memory_set.page_table.map(
            VirtAddr::new(TRAMPOLINE).page_number(),
            PhysAddr::new(strampoline as usize).page_number(),
            PTEFlags::R | PTEFlags::X,
        ).expect("Failed to map trampoline in kernel");

        memory_set
    }

    /// Create a new memory set from ELF data
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();
        // map trampoline
        // TRAMPOLINE is at the very top of virtual address space, so we can't do TRAMPOLINE + PAGE_SIZE
        // We manually map the one page at TRAMPOLINE
        extern "C" {
            fn strampoline();
        }
        let _ = memory_set.page_table.map(
            VirtAddr::new(TRAMPOLINE).page_number(),
            PhysAddr::new(strampoline as usize).page_number(),
            PTEFlags::R | PTEFlags::X,
        );

        // map user stack with U flag
        let user_sp = USER_STACK_TOP;
        memory_set.push(
            MapArea::new(
                user_sp - USER_STACK_SIZE,
                user_sp,
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
        // map trap context
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT,
                TRAMPOLINE,
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        
        // First pass: collect all segments and determine the full address range
        use alloc::vec::Vec;
        let mut segments: Vec<(usize, usize, MapPermission, usize, usize)> = Vec::new();
        let mut min_va = usize::MAX;
        let mut max_va = 0usize;
        
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: usize = ph.virtual_addr() as usize;
                let end_va: usize = (ph.virtual_addr() + ph.mem_size()) as usize;
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() { map_perm |= MapPermission::R; }
                if ph_flags.is_write() { map_perm |= MapPermission::W; }
                if ph_flags.is_execute() { map_perm |= MapPermission::X; }
                
                if start_va < min_va { min_va = start_va; }
                if end_va > max_va { max_va = end_va; }
                
                segments.push((start_va, end_va, map_perm, ph.offset() as usize, ph.file_size() as usize));
            }
        }
        
        // Round to page boundaries
        let program_start = min_va & !(PAGE_SIZE - 1);
        let program_end = (max_va + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        
        // Map the entire program region with combined permissions
        // We'll map page by page and merge permissions
        let start_vpn = VirtAddr::new(program_start).page_number();
        let end_vpn = VirtAddr::new(program_end).page_number();
        
        for vpn_idx in start_vpn.0..end_vpn.0 {
            let vpn = VirtPageNum(vpn_idx);
            let va_start = vpn.0 * PAGE_SIZE;
            let va_end = va_start + PAGE_SIZE;
            
            // Determine permissions for this page by checking all overlapping segments
            let mut page_perm = MapPermission::U;
            for (seg_start, seg_end, seg_perm, _, _) in &segments {
                if va_start < *seg_end && va_end > *seg_start {
                    // Overlapping segment
                    page_perm |= *seg_perm;
                }
            }
            
            // Allocate and map the page
            let frame = FRAME_ALLOCATOR.alloc().expect("Failed to allocate frame");
            let ppn = frame;
            
            let mut flags = PTEFlags::V | PTEFlags::A;
            if page_perm.contains(MapPermission::R) { flags |= PTEFlags::R; }
            if page_perm.contains(MapPermission::W) { flags |= PTEFlags::W | PTEFlags::D; }
            if page_perm.contains(MapPermission::X) { flags |= PTEFlags::X; }
            if page_perm.contains(MapPermission::U) { flags |= PTEFlags::U; }
            
            memory_set.page_table.map(vpn, ppn, flags).expect("Failed to map page");
            
            // Clear the page
            let bytes_array = ppn.as_ptr::<u8>();
            for i in 0..PAGE_SIZE {
                unsafe { *bytes_array.add(i) = 0; }
            }
            
            // Copy data from all overlapping segments
            for (seg_start, seg_end, _, file_offset, file_size) in &segments {
                let seg_file_end = *file_offset + *file_size;
                if va_start < *seg_end && va_end > *seg_start {
                    // Calculate overlap
                    let copy_start_va = core::cmp::max(va_start, *seg_start);
                    let copy_end_va = core::cmp::min(va_end, *seg_end);
                    let copy_size = copy_end_va - copy_start_va;
                    
                    // Calculate file offset
                    let offset_in_seg = copy_start_va.saturating_sub(*seg_start);
                    let file_start = *file_offset + offset_in_seg;
                    let file_end = core::cmp::min(file_start + copy_size, seg_file_end);
                    let actual_copy_size = file_end - file_start;
                    
                    if actual_copy_size > 0 {
                        // Copy data
                        let dst_offset = copy_start_va - va_start;
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                elf.input[file_start..file_end].as_ptr(),
                                bytes_array.add(dst_offset),
                                actual_copy_size
                            );
                        }
                    }
                }
            }
        }
        
        let entry_point = elf.header.pt2.entry_point() as usize;
        
        (memory_set, USER_STACK_TOP, entry_point)
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
}
