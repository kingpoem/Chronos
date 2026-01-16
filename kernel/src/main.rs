#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

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
global_asm!(include_str!("link_app.S"));

/// kernel entry point
/// called by bootloader
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
    mm::init(dtb);
    trap::init();
    task::init();

    println!("\n[Kernel] All subsystems initialized!");

    // Run tests
    println!("\n[Kernel] Running tests...\n");
    test_kernel();

    println!("\n[Kernel] Tests completed!");

    // Load and run user programs
    println!("\n[Kernel] Loading user programs...");
    task::load_apps();

    // Enable timer interrupt AFTER tasks are loaded
    // This ensures the system is fully initialized before handling interrupts
    println!("[Kernel] Enabling timer interrupt for preemptive scheduling...");
    trap::enable_timer_interrupt();

    // Start first task
    println!("[Kernel] Starting first task...");
    task::switch_task();

    // Should never reach here
    println!("\n[Kernel] All tasks completed, shutting down...");
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
    println!("Memory management OK");
}
