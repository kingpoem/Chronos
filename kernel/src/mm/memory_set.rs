//! Memory Set Management
//! 
//! Manages virtual memory spaces for kernel and user processes

use super::frame_allocator::{FrameAllocator, FRAME_ALLOCATOR};
use super::memory_layout::*;
use super::page_table::{PTEFlags, PageTable, PageTableEntry};
use crate::config::memory_layout::{PAGE_SIZE, MEMORY_END};
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

/// Map permission flags
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
        page_table.map(vpn, ppn, pte_flags).expect("Failed to map page");
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
}
