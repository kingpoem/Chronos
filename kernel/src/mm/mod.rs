//! Memory Management Module
//!
//! This module provides comprehensive memory management functionality:
//! - Physical memory allocation (frame allocator)
//! - Virtual memory management (page tables)
//! - Heap allocation
//! - Memory layout definitions
//! - Memory set management

pub mod frame_allocator;
pub mod heap;
pub mod memory_layout;
pub mod memory_set;
pub mod page_table;

pub use frame_allocator::{FrameAllocator, FRAME_ALLOCATOR};
pub use memory_set::{MapPermission, MapType, MemorySet};
pub use page_table::{PTEFlags, PageTable, PageTableEntry};

use crate::config::memory_layout::*;
use crate::{println, sbi};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    /// Kernel address space
    pub static ref KERNEL_SPACE: Mutex<MemorySet> = Mutex::new(MemorySet::new_kernel());
}

/// Initialize the memory management system
///
/// # Arguments
/// * `dtb` - Device Tree Blob address to determine available memory
pub fn init(_dtb: usize) {
    sbi::console_putstr("[MM] Initializing memory management system...\n");

    // Parse DTB to get memory regions (simplified - assume 128MB at 0x80000000)
    let mem_start = KERNEL_HEAP_START;
    let mem_end = MEMORY_END;

    sbi::console_putstr("[MM] Memory range: 0x");
    print_hex(mem_start);
    sbi::console_putstr(" - 0x");
    print_hex(mem_end);
    sbi::console_putstr("\n");

    // Initialize frame allocator
    unsafe {
        frame_allocator::init(mem_start, mem_end);
    }
    sbi::console_putstr("[MM] Frame allocator initialized\n");

    // Initialize heap allocator
    unsafe {
        heap::init_heap();
    }
    sbi::console_putstr("[MM] Heap allocator initialized\n");

    // Initialize Kernel Space and enable paging
    // Note: KERNEL_SPACE depends on heap allocation, so this must be after heap::init_heap
    KERNEL_SPACE.lock().activate();
    sbi::console_putstr("[MM] Kernel address space initialized and paging enabled\n");

    sbi::console_putstr("[MM] Memory management system initialized successfully\n");
}

/// Print a usize as hexadecimal
fn print_hex(n: usize) {
    let hex_digits = b"0123456789abcdef";
    let mut buffer = [0u8; 16];
    let mut num = n;
    let mut i = 0;

    if num == 0 {
        sbi::console_putchar(b'0');
        return;
    }

    while num > 0 {
        buffer[i] = hex_digits[(num & 0xF) as usize];
        num >>= 4;
        i += 1;
    }

    for j in (0..i).rev() {
        sbi::console_putchar(buffer[j]);
    }
}

/// Test memory management
pub fn test() {
    // Test frame allocation
    if let Some(frame1) = FRAME_ALLOCATOR.alloc() {
        println!("  Frame allocated at PPN: {:#x}", frame1.as_usize());

        if let Some(frame2) = FRAME_ALLOCATOR.alloc() {
            println!("  Second frame allocated at PPN: {:#x}", frame2.as_usize());
            FRAME_ALLOCATOR.dealloc(frame2);
        }

        FRAME_ALLOCATOR.dealloc(frame1);
        println!("  Frames deallocated");
    }

    println!(
        "  Free frames: {} / {}",
        FRAME_ALLOCATOR.free_frames(),
        FRAME_ALLOCATOR.total_frames()
    );

    // Test heap allocation
    use alloc::vec::Vec;
    let mut v = Vec::new();
    for i in 0..10 {
        v.push(i);
    }
    println!("  Heap allocation test: vec = {:?}", v);
}
