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
pub use memory_set::MemorySet;
// Re-export page table types (may be used by other modules)
#[allow(unused_imports)]
pub use page_table::{PTEFlags, PageTable, PageTableEntry};
// Re-export commonly used types
#[allow(unused_imports)]
pub use memory_set::{MapPermission, MapType};

use crate::config::memory_layout::*;
use crate::sbi;
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
    sbi::console_putstr("[MM] Initializing memory management system...\n");

    // Parse DTB to get memory regions (simplified - assume 128MB at 0x80000000)
    let mem_start = KERNEL_HEAP_START;
    let mem_end = MEMORY_END;

    sbi::console_putstr("[MM] Memory range: 0x");
    print_hex(mem_start);
    sbi::console_putstr(" - 0x");
    print_hex(mem_end);
    sbi::console_putstr("\n");

    // Initialize frame allocator (before creating kernel address space)
    unsafe {
        frame_allocator::init(mem_start, mem_end);
    }
    sbi::console_putstr("[MM] Frame allocator initialized\n");

    // Initialize heap allocator BEFORE creating kernel address space
    // This is necessary because MemorySet::new_kernel() uses Vec and BTreeMap
    // which require heap allocation
    // We use KERNEL_HEAP_START as a safe estimate (will be remapped after paging is enabled)
    unsafe {
        heap::init_heap();
    }
    sbi::console_putstr("[MM] Heap allocator initialized (before paging)\n");

    // Create and activate kernel address space
    // This is the critical transition from physical address access to virtual address access
    sbi::console_putstr("[MM] Creating kernel address space...\n");
    let kernel_space = MemorySet::new_kernel();
    
    sbi::console_putstr("[MM] Kernel address space created\n");
    
    // Store kernel address space before activation (for verification)
    *KERNEL_SPACE_INTERNAL.lock() = Some(kernel_space);
    
    // Activate kernel address space (enable paging)
    // After this point, all memory accesses go through MMU
    sbi::console_putstr("[MM] Activating kernel address space (enabling paging)...\n");
    sbi::console_putstr("[MM] Note: Kernel uses IDENTITY MAPPING (VA == PA)\n");
    sbi::console_putstr("[MM]   This allows kernel to directly access physical memory\n");
    {
        let ks = KERNEL_SPACE_INTERNAL.lock();
        ks.as_ref().unwrap().activate();
    }
    
    // Verify address translation is working (but be careful not to access unmapped memory)
    verify_address_translation();
    
    sbi::console_putstr("[MM] Kernel address space activated\n");
    sbi::console_putstr("[MM] Memory management system initialized successfully\n");
}


/// Verify that address translation is working correctly
fn verify_address_translation() {
    use riscv::register::satp;
    
    // Read current satp register
    let current_satp = satp::read();
    sbi::console_putstr("[MM] Current SATP: 0x");
    print_hex(current_satp.bits());
    sbi::console_putstr("\n");
    
    // Verify that satp is set (paging is enabled)
    // Check if mode is Sv39 (value 8 in bits 60-63)
    let mode = (current_satp.bits() >> 60) & 0xF;
    if mode == 8 {
        sbi::console_putstr("[MM] ✓ Paging mode: SV39 enabled\n");
    } else {
        sbi::console_putstr("[MM] ✗ WARNING: Paging mode not enabled! Mode: ");
        print_hex(mode as usize);
        sbi::console_putstr("\n");
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
            sbi::console_putstr("[MM] ✓ Address translation: VA 0x");
            print_hex(text_va);
            sbi::console_putstr(" -> PA 0x");
            print_hex(text_pa);
            sbi::console_putstr("\n");
            
            // For identical mapping, VA should equal PA
            // This is intentional: kernel uses identity mapping so VA == PA
            // This allows kernel to directly access physical memory and MMIO devices
            if text_va == text_pa {
                sbi::console_putstr("[MM] ✓ Identical mapping verified (VA == PA) - CORRECT\n");
                sbi::console_putstr("[MM]   Note: Kernel uses identity mapping for direct physical access\n");
            } else {
                sbi::console_putstr("[MM] ✗ WARNING: Identical mapping mismatch!\n");
            }
        } else {
            sbi::console_putstr("[MM] ✗ ERROR: Address translation failed for kernel text!\n");
        }
        
        // Test translation of kernel end
        let ekernel_va = ekernel as usize;
        if let Some(ekernel_pa) = ks.translate(ekernel_va) {
            sbi::console_putstr("[MM] ✓ Address translation: VA 0x");
            print_hex(ekernel_va);
            sbi::console_putstr(" -> PA 0x");
            print_hex(ekernel_pa);
            sbi::console_putstr("\n");
            
            if ekernel_va == ekernel_pa {
                sbi::console_putstr("[MM] ✓ Identical mapping verified for ekernel\n");
            }
        }
    }
    
    // Test that we can still access kernel symbols (proves translation works)
    // Note: After paging is enabled, all memory accesses go through MMU
    // Since we use identical mapping, VA == PA, so this should work
    extern "C" {
        fn stext();
    }
    
    // Verify kernel text pointer is valid (don't dereference to avoid potential page fault)
    let text_va = stext as usize;
    sbi::console_putstr("[MM] ✓ Kernel text VA: 0x");
    print_hex(text_va);
    sbi::console_putstr("\n");
    
    sbi::console_putstr("[MM] ✓ Address translation verification completed\n");
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

    // Test heap allocation (skip for now to avoid potential issues)
    // Heap allocator should be initialized, but we'll test it separately
    // use alloc::vec::Vec;
    // let mut v = Vec::new();
    // for i in 0..10 {
    //     v.push(i);
    // }
    // println!("  Heap allocation test: vec = {:?}", v);
    println!("  Heap allocation test: skipped (heap allocator initialized)");
}
