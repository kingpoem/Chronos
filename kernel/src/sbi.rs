//! SBI helpers using `sbi-rt` (RustSBI prototyper friendly)
#![allow(dead_code)]

use sbi_rt as sbi;

/// Print a single character to console (legacy)
pub fn console_putchar(ch: u8) {
    let _ = sbi::legacy::console_putchar(ch as usize);
}

/// Print a string to console
pub fn console_putstr(s: &str) {
    for ch in s.bytes() {
        console_putchar(ch);
    }
}

/// Get a character from console (non-blocking)
pub fn console_getchar() -> Option<u8> {
    let ch = sbi::legacy::console_getchar();
    if ch == usize::MAX {
        None
    } else {
        Some(ch as u8)
    }
}

/// Set timer for next timer event
pub fn set_timer(stime_value: u64) {
    // Timer extension is optional; no-op placeholder for now
    let _ = stime_value;
}

/// Get current time value (fallback to MMIO mtime)
pub fn get_time() -> u64 {
    const MTIME_ADDR: *const u64 = 0x200bff8 as *const u64;
    unsafe { core::ptr::read_volatile(MTIME_ADDR) }
}

/// Initialize SBI (just a banner)
pub fn init() {
    console_putstr("SBI: RustSBI prototyper (via sbi-rt)\n");
}

/// Shutdown the system
pub fn shutdown() -> ! {
    // Try System Reset Extension (SRST)
    // Type: Shutdown (0), Reason: No Reason (0)
    let _ = sbi::system_reset(sbi::Shutdown, sbi::NoReason);

    // Fallback to legacy shutdown if SRST fails or returns
    // Note: sbi-rt's legacy::shutdown() panics if it returns, so this is a last resort
    let _ = sbi::legacy::shutdown();

    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}
