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

pub use frame_allocator::FRAME_ALLOCATOR;
#[allow(unused_imports)]
pub use frame_allocator::FrameAllocator;
pub use memory_set::MemorySet;
// Re-export page table types (may be used by other modules)
#[allow(unused_imports)]
pub use page_table::{PTEFlags, PageTable, PageTableEntry};
// Re-export commonly used types
#[allow(unused_imports)]
pub use memory_set::{MapPermission, MapType};

use crate::config::memory_layout::*;
use lazy_static::*;
use spin::Mutex;

lazy_static! {
    pub(crate) static ref KERNEL_SPACE_INTERNAL: Mutex<Option<MemorySet>> = Mutex::new(None);
}

/// Get kernel address space token
/// This should only be called after mm::init() has completed
pub fn get_kernel_token() -> usize {
    KERNEL_SPACE_INTERNAL.lock().as_ref()
        .expect("Kernel address space not initialized")
        .token()
}

/// Get kernel address space token (for task module)
/// This should only be called after mm::init() has completed
pub fn get_kernel_space_token() -> Option<usize> {
    KERNEL_SPACE_INTERNAL.lock().as_ref().map(|ks| ks.token())
}

/// Initialize the memory management system
///
/// # Arguments
/// * `dtb` - Device Tree Blob address to determine available memory
pub fn init(_dtb: usize) {
    // Parse DTB to get memory regions (simplified - assume 128MB at 0x80000000)
    let mem_start = KERNEL_HEAP_START;
    let mem_end = MEMORY_END;

    // Initialize frame allocator (before creating kernel address space)
    unsafe {
        frame_allocator::init(mem_start, mem_end);
    }

    // Initialize heap allocator BEFORE creating kernel address space
    // This is necessary because MemorySet::new_kernel() uses Vec and BTreeMap
    // which require heap allocation
    // We use KERNEL_HEAP_START as a safe estimate (will be remapped after paging is enabled)
    unsafe {
        heap::init_heap();
    }

    // Create and activate kernel address space
    // This is the critical transition from physical address access to virtual address access
    let kernel_space = MemorySet::new_kernel();

    // Store kernel address space before activation (for verification)
    *KERNEL_SPACE_INTERNAL.lock() = Some(kernel_space);

    // Activate kernel address space (enable paging)
    // After this point, all memory accesses go through MMU
    {
        let ks = KERNEL_SPACE_INTERNAL.lock();
        ks.as_ref().unwrap().activate();
    }

    // Kernel satp is embedded into the trampoline page in MemorySet::new_kernel().

    // Verify address translation is working (but be careful not to access unmapped memory)
    verify_address_translation();
}


/// Verify that address translation is working correctly
fn verify_address_translation() {
    use riscv::register::satp;

    // Read current satp register
    let current_satp = satp::read();

    // Verify that satp is set (paging is enabled)
    // Check if mode is Sv39 (value 8 in bits 60-63)
    let mode = (current_satp.bits() >> 60) & 0xF;
    if mode != 8 {
        panic!("Paging mode not enabled");
    }

    // Test address translation using kernel space
    let kernel_space = KERNEL_SPACE_INTERNAL.lock();
    if let Some(ref ks) = *kernel_space {
        extern "C" {
            fn stext();
            fn ekernel();
        }

        // Test translation of kernel text section
        let text_va = stext as usize;
        if let Some(text_pa) = ks.translate(text_va) {
            if text_va != text_pa {
                panic!("Identical mapping mismatch for kernel text");
            }
        } else {
            panic!("Address translation failed for kernel text");
        }

        // Test translation of kernel end
        let ekernel_va = ekernel as usize;
        if let Some(ekernel_pa) = ks.translate(ekernel_va) {
            if ekernel_va != ekernel_pa {
                panic!("Identical mapping mismatch for ekernel");
            }
        }
    }
}

/// Test memory management
pub fn test() {
    // Test frame allocation
    if let Some(frame1) = FRAME_ALLOCATOR.alloc() {
        if let Some(frame2) = FRAME_ALLOCATOR.alloc() {
            FRAME_ALLOCATOR.dealloc(frame2);
        }

        FRAME_ALLOCATOR.dealloc(frame1);
    }

    // Test heap allocation (skip for now to avoid potential issues)
    // Heap allocator should be initialized, but we'll test it separately
    // use alloc::vec::Vec;
    // let mut v = Vec::new();
    // for i in 0..10 {
    //     v.push(i);
    // }
    // println!("  Heap allocation test: vec = {:?}", v);
    let _ = FRAME_ALLOCATOR.free_frames();
    let _ = FRAME_ALLOCATOR.total_frames();
}
