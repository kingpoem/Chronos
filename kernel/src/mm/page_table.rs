//! Page Table Management
//!
//! Implements SV39 page table for RISC-V 64-bit systems.
//! SV39 uses 3-level page tables with 39-bit virtual addresses.

use super::frame_allocator::FRAME_ALLOCATOR;
use super::memory_layout::*;
use crate::config::memory_layout::*;
use core::fmt::{self, Debug, Formatter};
use alloc::vec::Vec;

/// Page Table Entry (PTE) flags
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct PTEFlags(u8);

impl PTEFlags {
    /// Valid flag
    pub const V: Self = Self(1 << 0);
    /// Readable flag
    pub const R: Self = Self(1 << 1);
    /// Writable flag
    pub const W: Self = Self(1 << 2);
    /// Executable flag
    pub const X: Self = Self(1 << 3);
    /// User accessible flag
    pub const U: Self = Self(1 << 4);
    /// Global mapping flag
    pub const G: Self = Self(1 << 5);
    /// Accessed flag (set by hardware)
    pub const A: Self = Self(1 << 6);
    /// Dirty flag (set by hardware)
    pub const D: Self = Self(1 << 7);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(&self) -> u8 {
        self.0
    }

    pub fn from_bits(bits: u8) -> Option<Self> {
        Some(Self(bits))
    }

    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for PTEFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for PTEFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl Debug for PTEFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PTEFlags(")?;
        if self.contains(Self::V) {
            write!(f, "V")?;
        }
        if self.contains(Self::R) {
            write!(f, "R")?;
        }
        if self.contains(Self::W) {
            write!(f, "W")?;
        }
        if self.contains(Self::X) {
            write!(f, "X")?;
        }
        if self.contains(Self::U) {
            write!(f, "U")?;
        }
        if self.contains(Self::G) {
            write!(f, "G")?;
        }
        if self.contains(Self::A) {
            write!(f, "A")?;
        }
        if self.contains(Self::D) {
            write!(f, "D")?;
        }
        write!(f, ")")
    }
}

/// Page Table Entry
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    bits: usize,
}

impl PageTableEntry {
    /// Create a new invalid PTE
    pub const fn new() -> Self {
        Self { bits: 0 }
    }

    /// Create a PTE from physical page number and flags
    pub fn new_with_ppn(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self {
            bits: (ppn.as_usize() << 10) | flags.bits() as usize,
        }
    }

    /// Get physical page number from PTE
    pub fn ppn(&self) -> PhysPageNum {
        PhysPageNum::new((self.bits >> 10) & 0x0FFF_FFFF_FFFF)
    }

    /// Get flags from PTE
    pub fn flags(&self) -> PTEFlags {
        PTEFlags((self.bits & 0xFF) as u8)
    }

    /// Check if PTE is valid
    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::V)
    }

    /// Check if PTE is a leaf (R/W/X set)
    pub fn is_leaf(&self) -> bool {
        let flags = self.flags();
        flags.contains(PTEFlags::R) || flags.contains(PTEFlags::W) || flags.contains(PTEFlags::X)
    }

    /// Clear the PTE
    pub fn clear(&mut self) {
        self.bits = 0;
    }
}

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PTE")
            .field("ppn", &self.ppn())
            .field("flags", &self.flags())
            .finish()
    }
}

/// Page Table (512 entries for SV39)
#[repr(C)]
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); 512],
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }

    /// Get a reference to an entry
    pub fn entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get a mutable reference to an entry
    pub fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// Map a virtual page to a physical page
    ///
    /// # Arguments
    /// * `vpn` - Virtual page number
    /// * `ppn` - Physical page number
    /// * `flags` - Page table entry flags
    pub fn map(
        &mut self,
        vpn: VirtPageNum,
        ppn: PhysPageNum,
        flags: PTEFlags,
    ) -> Result<(), &'static str> {
        let indexes = vpn.indexes();
        let mut current_table = self as *mut PageTable;

        // Traverse page table levels
        for (_level, &index) in indexes.iter().enumerate().take(2) {
            let entry = unsafe { (*current_table).entry_mut(index) };

            if !entry.is_valid() {
                // Allocate a new page table
                let new_ppn = FRAME_ALLOCATOR.alloc().ok_or("Out of memory")?;
                let new_table = new_ppn.as_ptr::<PageTable>();

                // Clear the new page table
                unsafe {
                    (*new_table).clear();
                }

                // Set the entry to point to the new table
                *entry = PageTableEntry::new_with_ppn(new_ppn, PTEFlags::V);
            }

            if entry.is_leaf() {
                return Err("Encountered leaf PTE in intermediate level");
            }

            // Move to next level
            current_table = entry.ppn().as_ptr::<PageTable>();
        }

        // Set the leaf entry
        let leaf_entry = unsafe { (*current_table).entry_mut(indexes[2]) };
        if leaf_entry.is_valid() {
            return Err("Page already mapped");
        }

        *leaf_entry = PageTableEntry::new_with_ppn(ppn, flags | PTEFlags::V);
        Ok(())
    }

    /// Unmap a virtual page
    pub fn unmap(&mut self, vpn: VirtPageNum) -> Result<PhysPageNum, &'static str> {
        let indexes = vpn.indexes();
        let mut current_table = self as *mut PageTable;

        // Traverse page table levels
        for &index in indexes.iter().take(2) {
            let entry = unsafe { (*current_table).entry(index) };

            if !entry.is_valid() {
                return Err("Page not mapped");
            }

            if entry.is_leaf() {
                return Err("Encountered leaf PTE in intermediate level");
            }

            current_table = entry.ppn().as_ptr::<PageTable>();
        }

        // Clear the leaf entry
        let leaf_entry = unsafe { (*current_table).entry_mut(indexes[2]) };
        if !leaf_entry.is_valid() {
            return Err("Page not mapped");
        }

        let ppn = leaf_entry.ppn();
        leaf_entry.clear();
        Ok(ppn)
    }

    /// Translate virtual page number to physical page number
    pub fn translate(&self, vpn: VirtPageNum) -> Option<(PhysPageNum, PTEFlags)> {
        let indexes = vpn.indexes();
        let mut current_table = self as *const PageTable;

        for &index in indexes.iter().take(2) {
            let entry = unsafe { (*current_table).entry(index) };

            if !entry.is_valid() {
                return None;
            }

            if entry.is_leaf() {
                return Some((entry.ppn(), entry.flags()));
            }

            current_table = entry.ppn().as_ptr::<PageTable>();
        }

        let leaf_entry = unsafe { (*current_table).entry(indexes[2]) };
        if leaf_entry.is_valid() {
            Some((leaf_entry.ppn(), leaf_entry.flags()))
        } else {
            None
        }
    }

    /// Get the physical address of this page table
    pub fn as_ppn(&self) -> PhysPageNum {
        PhysPageNum::new((self as *const _ as usize) >> PAGE_SIZE_BITS)
    }
    
    /// Get a mutable reference to a leaf entry (for modifying flags)
    /// This is unsafe because it bypasses the normal page table traversal
    pub unsafe fn get_pte_mut(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let indexes = vpn.indexes();
        let mut current_table = self as *mut PageTable;
        
        // Traverse to the leaf entry
        for &index in &indexes[..2] {
            let entry = (*current_table).entry(index);
            if !entry.is_valid() {
                return None;
            }
            if entry.is_leaf() {
                return None; // Not a leaf entry
            }
            current_table = entry.ppn().as_ptr::<PageTable>();
        }
        
        // Get the leaf entry
        Some((*current_table).entry_mut(indexes[2]))
    }
    
    /// Recursively deallocate this page table and all intermediate page tables
    /// This is used when destroying an address space
    /// 
    /// # Safety
    /// This function is unsafe because it assumes:
    /// 1. This page table is no longer in use (not active in satp)
    /// 2. All leaf entries have been unmapped
    /// 3. The caller is responsible for ensuring no concurrent access
    pub unsafe fn dealloc(&mut self) {
        // Recursively deallocate all intermediate page tables
        for i in 0..512 {
            let entry = self.entry(i);
            if entry.is_valid() && !entry.is_leaf() {
                // This is an intermediate page table, recursively deallocate it
                let sub_table_ppn = entry.ppn();
                let sub_table = sub_table_ppn.as_ptr::<PageTable>();
                (*sub_table).dealloc();
                // Deallocate the intermediate page table frame
                FRAME_ALLOCATOR.dealloc(sub_table_ppn);
            }
            // Clear this entry
            self.entry_mut(i).clear();
        }
    }
    
    /// Deallocate this page table (non-recursive version)
    /// Only deallocates intermediate page tables, not leaf pages
    /// Leaf pages should be unmapped before calling this
    pub fn dealloc_intermediate_tables(&mut self) {
        let mut intermediate_ppns = Vec::new();
            
            // Collect all intermediate page table PPNs
            for i in 0..512 {
                let entry = self.entry(i);
                if entry.is_valid() && !entry.is_leaf() {
                    intermediate_ppns.push(entry.ppn());
                }
            }
            
            // Recursively deallocate intermediate tables
            for ppn in intermediate_ppns {
                let sub_table = ppn.as_ptr::<PageTable>();
                unsafe {
                    (*sub_table).dealloc_intermediate_tables();
                }
                FRAME_ALLOCATOR.dealloc(ppn);
            }
            
            // Clear all entries
            self.clear();
    }
    
    /// Translate a byte buffer from user virtual address space to kernel virtual address space
    /// This is used to safely access user space data from kernel space
    /// 
    /// # Arguments
    /// * `user_va` - User virtual address
    /// * `len` - Length of the buffer in bytes
    /// 
    /// # Returns
    /// * `Vec<&'static mut [u8]>` - Vector of byte slices, each representing a contiguous page
    /// 
    /// # Safety
    /// The returned slices are valid only while the page table is active and the pages are mapped.
    /// The caller must ensure that the page table remains valid for the lifetime of the slices.
    pub fn translated_byte_buffer(
        &self,
        user_va: usize,
        len: usize,
    ) -> Vec<&'static mut [u8]> {
        let mut buffers = Vec::new();
        let mut current_va = user_va;
        let mut remaining = len;
        
        while remaining > 0 {
            let vpn = VirtAddr::new(current_va).page_number();
            let page_offset = VirtAddr::new(current_va).page_offset();
            
            // Translate user virtual address to physical address
            if let Some((ppn, _flags)) = self.translate(vpn) {
                // Convert physical address to kernel virtual address (identity mapping)
                let kernel_va = ppn.addr().0 + page_offset;
                
                // Calculate how many bytes we can read from this page
                let bytes_in_page = (PAGE_SIZE - page_offset).min(remaining);
                
                // Create a slice pointing to kernel virtual address
                unsafe {
                    let ptr = kernel_va as *mut u8;
                    let slice = core::slice::from_raw_parts_mut(ptr, bytes_in_page);
                    buffers.push(slice);
                }
                
                current_va += bytes_in_page;
                remaining -= bytes_in_page;
            } else {
                // Page not mapped, return empty buffer
                break;
            }
        }
        
        buffers
    }
    
    /// Translate a byte buffer from user virtual address space (read-only)
    /// Similar to translated_byte_buffer but returns immutable slices
    pub fn translated_byte_buffer_readonly(
        &self,
        user_va: usize,
        len: usize,
    ) -> Vec<&'static [u8]> {
        let mut buffers = Vec::new();
        let mut current_va = user_va;
        let mut remaining = len;
        
        while remaining > 0 {
            let vpn = VirtAddr::new(current_va).page_number();
            let page_offset = VirtAddr::new(current_va).page_offset();
            
            // Translate user virtual address to physical address
            if let Some((ppn, _flags)) = self.translate(vpn) {
                // Convert physical address to kernel virtual address (identity mapping)
                let kernel_va = ppn.addr().0 + page_offset;
                
                // Calculate how many bytes we can read from this page
                let bytes_in_page = (PAGE_SIZE - page_offset).min(remaining);
                
                // Create a slice pointing to kernel virtual address
                unsafe {
                    let ptr = kernel_va as *const u8;
                    let slice = core::slice::from_raw_parts(ptr, bytes_in_page);
                    buffers.push(slice);
                }
                
                current_va += bytes_in_page;
                remaining -= bytes_in_page;
            } else {
                // Page not mapped, return empty buffer
                break;
            }
        }
        
        buffers
    }
    
    /// Print page table contents (for debugging)
    /// This recursively traverses the page table and prints all valid mappings
    pub fn print_contents(&self, max_entries: usize) {
        crate::sbi::console_putstr("\n[PageTable] Printing page table contents (max ");
        crate::trap::print_hex_usize(max_entries);
        crate::sbi::console_putstr(" entries)...\n");
        
        let mut count = 0;
        self.print_level(self, 0, 0, &mut count, max_entries);
        
        crate::sbi::console_putstr("[PageTable] Total entries printed: ");
        crate::trap::print_hex_usize(count);
        crate::sbi::console_putstr("\n");
    }
    
    /// Recursively print page table entries
    fn print_level(
        &self,
        table: &PageTable,
        level: usize,
        base_vpn: usize,
        count: &mut usize,
        max_entries: usize,
    ) {
        
        if *count >= max_entries {
            return;
        }
        
        for (index, entry) in table.entries.iter().enumerate() {
            if *count >= max_entries {
                break;
            }
            
            if entry.is_valid() {
                let flags = entry.flags();
                let ppn = entry.ppn();
                
                if entry.is_leaf() {
                    // Leaf entry - actual page mapping
                    let vpn = base_vpn | (index << (9 * (2 - level)));
                    let va = vpn << PAGE_SIZE_BITS;
                    let pa = ppn.as_usize() << PAGE_SIZE_BITS;
                    
                    crate::sbi::console_putstr("  L");
                    crate::trap::print_hex_usize(level);
                    crate::sbi::console_putstr(" VA=0x");
                    crate::trap::print_hex_usize(va);
                    crate::sbi::console_putstr(" -> PA=0x");
                    crate::trap::print_hex_usize(pa);
                    crate::sbi::console_putstr(" PPN=0x");
                    crate::trap::print_hex_usize(ppn.as_usize());
                    crate::sbi::console_putstr(" Flags=");
                    
                    if flags.contains(PTEFlags::R) {
                        crate::sbi::console_putstr("R");
                    }
                    if flags.contains(PTEFlags::W) {
                        crate::sbi::console_putstr("W");
                    }
                    if flags.contains(PTEFlags::X) {
                        crate::sbi::console_putstr("X");
                    }
                    if flags.contains(PTEFlags::U) {
                        crate::sbi::console_putstr("U");
                    }
                    if flags.contains(PTEFlags::G) {
                        crate::sbi::console_putstr("G");
                    }
                    crate::sbi::console_putstr("\n");
                    
                    *count += 1;
                } else if level < 2 {
                    // Intermediate entry - recurse to next level
                    let next_base_vpn = base_vpn | (index << (9 * (2 - level)));
                    let next_table_ptr = ppn.as_ptr::<PageTable>();
                    unsafe {
                        let next_table = &*next_table_ptr;
                        self.print_level(next_table, level + 1, next_base_vpn, count, max_entries);
                    }
                }
            }
        }
    }
}

impl Debug for PageTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "PageTable")
    }
}
