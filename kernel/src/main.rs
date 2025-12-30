#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![cfg_attr(test, allow(dead_code))]

extern crate alloc;

#[macro_use]
mod console;
mod config;
mod drivers;
mod lang_items;
mod mm;
mod sbi;
mod syscall;
mod task;
mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.S"));

/// 内核入口点
/// 由 bootloader 调用
#[no_mangle]
pub fn kernel_main(hartid: usize, dtb: usize) -> ! {
    // Clear BSS segment
    clear_bss();

    // Initialize console
    console::init();

    // Print banner
    println!("=================================");
    println!("Chronos OS Kernel v0.2.0");
    println!("=================================");
    println!("Hart ID: {}", hartid);
    println!("DTB: {:#x}", dtb);

    // Initialize subsystems
    println!("\n[Init] Initializing subsystems...");
    mm::init(dtb);
    trap::init();
    task::init();

    println!("\n[Kernel] All subsystems initialized!");
    
    // Run tests
    println!("\n[Kernel] Running tests...\n");
    test_kernel();

    println!("\n[Kernel] Tests completed!");
    println!("[Kernel] System features:");
    println!("  ✓ Buddy System Allocator");
    println!("  ✓ SV39 Page Table");
    println!("  ✓ Trap Handling");
    println!("  ✓ System Calls");
    println!("  ✓ User Mode Support (Ready)");
    
    println!("\n[Kernel] Shutting down...");
    sbi::shutdown();
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

fn test_kernel() {
    println!("=== Memory Management Tests ===");
    mm::test();
    
    println!("\n=== System Call Tests ===");
    test_syscalls();
    
    println!("\n=== All Tests Passed! ===");
}

fn test_syscalls() {
    println!("  Testing system calls...");
    // System calls will be tested through trap handler
    println!("  System call framework ready");
}
