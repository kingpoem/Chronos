#![no_std]
#![no_main]

mod sbi;
mod boot;

use boot::rust_main;

/// Panic handler
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

/// Entry point called by RustSBI
/// 
/// RustSBI will set:
/// - a0: hart ID
/// - a1: DTB physical address
#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _start() -> ! {
    // Set stack pointer
    core::arch::asm!(
        "la sp, _stack_top",
        options(nostack, nomem)
    );
    
    // Clear BSS section
    let mut bss_start: usize;
    let mut bss_end: usize;
    core::arch::asm!(
        "la {0}, _bss_start",
        "la {1}, _bss_end",
        out(reg) bss_start,
        out(reg) bss_end,
    );
    
    while bss_start < bss_end {
        core::ptr::write_volatile(bss_start as *mut u64, 0);
        bss_start += 8;
    }
    
    // Get hart ID and DTB from registers
    let hartid: usize;
    let dtb: usize;
    core::arch::asm!(
        "mv {}, a0",
        "mv {}, a1",
        out(reg) hartid,
        out(reg) dtb,
    );
    
    // Call rust_main
    rust_main(hartid, dtb)
}

