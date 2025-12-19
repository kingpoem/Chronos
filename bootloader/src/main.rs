#![no_std]
#![no_main]
mod loader;

use core::arch::global_asm;

global_asm!(include_str!("entry.S"));

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[inline(always)]
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let mut ptr = sbss as usize;
    let end = ebss as usize;
    while ptr < end {
        unsafe { core::ptr::write_volatile(ptr as *mut u64, 0) };
        ptr += 8;
    }
}

#[no_mangle]
pub extern "C" fn rust_main(hartid: usize, dtb: usize) -> ! {
    clear_bss();
    loader::load_kernel(hartid, dtb)
}
