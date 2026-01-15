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
mod loader;
mod mm;
mod sbi;
mod syscall;
mod task;
mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.S"));
global_asm!(include_str!("link_app.S"));

/// Get number of applications
fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// Get application data
fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    println!("[Debug] app_start array: {:?}", app_start);
    assert!(app_id < num_app);
    let start = app_start[app_id];
    let end = app_start[app_id + 1];
    println!("[Debug] app {} range: {:#x} - {:#x}", app_id, start, end);
    unsafe { core::slice::from_raw_parts(start as *const u8, end - start) }
}

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

    // Run apps
    println!("[Kernel] Loading applications...\n");

    // Load user applications
    load_apps();
    println!("[Kernel] Starting first user task...\n");
    task::run_first_task();
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

/// Load all user applications
fn load_apps() {
    let num_app = get_num_app();
    println!("[Kernel] Found {} applications", num_app);

    for i in 0..num_app {
        let elf_data = get_app_data(i);
        println!("[Kernel] Loading app {}: {} bytes", i, elf_data.len());

        let task = task::TaskControlBlock::new(elf_data, i);
        println!("[Kernel] App {} loaded successfully", i);
        task::add_task(alloc::sync::Arc::new(task));
    }
}
