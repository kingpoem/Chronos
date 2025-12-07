use crate::sbi;

/// Kernel entry point called by RustSBI
/// 
/// # Arguments
/// * `hartid` - Hardware thread ID (passed in a0)
/// * `dtb` - Device tree blob physical address (passed in a1)
#[no_mangle]
pub extern "C" fn rust_main(hartid: usize, dtb: usize) -> ! {
    // Stack pointer is already set in _start
    
    // Print boot message
    sbi::console_putstr("Hello from kernel!\n");
    sbi::console_putstr("RustSBI Bootloader initialized\n");
    
    // Print hart ID
    sbi::console_putstr("Hart ID: ");
    print_usize(hartid);
    sbi::console_putstr("\n");
    
    // Print DTB address
    sbi::console_putstr("DTB address: 0x");
    print_hex(dtb);
    sbi::console_putstr("\n");

    // Initialize SBI
    sbi::init();

    // Test timer functionality
    sbi::console_putstr("Testing timer...\n");
    test_timer();

    // Test shutdown
    sbi::console_putstr("Shutting down...\n");
    sbi::system_reset(sbi::ResetType::Shutdown, sbi::ResetReason::NoReason);
    // system_reset never returns (returns !)
}

/// Print a usize as decimal
fn print_usize(n: usize) {
    if n == 0 {
        sbi::console_putchar(b'0');
        return;
    }
    
    let mut num = n;
    let mut digits = [0u8; 20];
    let mut i = 0;
    
    while num > 0 {
        digits[i] = (num % 10) as u8 + b'0';
        num /= 10;
        i += 1;
    }
    
    for j in (0..i).rev() {
        sbi::console_putchar(digits[j]);
    }
}

/// Print a usize as hexadecimal
fn print_hex(n: usize) {
    if n == 0 {
        sbi::console_putchar(b'0');
        return;
    }
    
    let mut num = n;
    let mut digits = [0u8; 16];
    let mut i = 0;
    
    while num > 0 {
        let digit = (num % 16) as u8;
        digits[i] = if digit < 10 {
            digit + b'0'
        } else {
            digit - 10 + b'a'
        };
        num /= 16;
        i += 1;
    }
    
    for j in (0..i).rev() {
        sbi::console_putchar(digits[j]);
    }
}

/// Test timer functionality
fn test_timer() {
    use riscv::register::{mie, mstatus};
    
    // Enable machine timer interrupt
    unsafe {
        mie::set_mtimer();
        mstatus::set_mie();
    }

    // Set timer for 1 second later (assuming 10MHz clock)
    // In real hardware, get clock frequency from DTB
    let timebase = 10_000_000; // 10MHz
    let time = sbi::get_time() + timebase;
    sbi::set_timer(time);

    sbi::console_putstr("Timer set, waiting for interrupt...\n");
    
    // Wait a bit (in real implementation, this would be handled by interrupt)
    for _ in 0..1000 {
        unsafe {
            core::arch::asm!("nop");
        }
    }
}

