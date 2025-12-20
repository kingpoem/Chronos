#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![cfg_attr(test, allow(dead_code))]

extern crate alloc;

mod config;
mod console;
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
    println!("Chronos OS Kernel v0.1.0");
    println!("=================================");
    println!("Hart ID: {}", hartid);
    println!("DTB: {:#x}", dtb);

    // Initialize各子系统
    mm::init(dtb);
    trap::init();

    println!("\n[Kernel] All subsystems initialized!");
    println!("[Kernel] Running tests...\n");

    // Run tests
    test_kernel();

    println!("\n[Kernel] Tests completed! Shutting down...");
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
    println!("Testing memory management...");
    mm::test();
    println!("✓ Memory management OK");
}
