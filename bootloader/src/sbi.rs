//! SBI (Supervisor Binary Interface) implementation
//! 
//! This module implements SBI v2.0 specification functions
//! for RISC-V systems.

// SBI implementation - no direct register access needed here

/// SBI Extension IDs
#[allow(dead_code)]
#[repr(usize)]
pub enum ExtensionId {
    Base = 0x10,
    Timer = 0x54494D45,      // "TIME"
    Ipi = 0x735049,          // "sPI"
    Rfence = 0x52464E43,     // "RFNC"
    Hsm = 0x48534D,          // "HSM"
    Srst = 0x53525354,       // "SRST"
    Console = 0x434F4E53,    // "CONS"
    Pm = 0x504D,             // "PM"
    Dbcn = 0x4442434E,       // "DBCN"
    Suspend = 0x53555350,    // "SUSP"
    Cppc = 0x43505043,       // "CPPC"
}

/// SBI Function IDs for Base Extension
#[allow(dead_code)]
#[repr(usize)]
pub enum BaseFunction {
    GetSbiVersion = 0,
    GetSbiImplId = 1,
    GetSbiImplVersion = 2,
    ProbeExtension = 3,
    GetMvendorid = 4,
    GetMarchid = 5,
    GetMimpid = 6,
}

/// SBI Function IDs for Timer Extension
#[repr(usize)]
pub enum TimerFunction {
    SetTimer = 0,
}

/// SBI Function IDs for Console Extension
#[allow(dead_code)]
#[repr(usize)]
pub enum ConsoleFunction {
    PutChar = 0,
    GetChar = 1,
}

/// SBI Function IDs for System Reset Extension
#[repr(usize)]
pub enum SystemResetFunction {
    SystemReset = 0,
}

/// Reset Type
#[allow(dead_code)]
#[repr(usize)]
pub enum ResetType {
    Shutdown = 0,
    ColdReboot = 1,
    WarmReboot = 2,
}

/// Reset Reason
#[allow(dead_code)]
#[repr(usize)]
pub enum ResetReason {
    NoReason = 0,
    SystemFailure = 1,
}

/// SBI error codes
#[allow(dead_code)]
#[repr(isize)]
pub enum SbiError {
    Success = 0,
    Failed = -1,
    NotSupported = -2,
    InvalidParam = -3,
    Denied = -4,
    InvalidAddress = -5,
    AlreadyAvailable = -6,
}

/// Make an SBI call
#[inline(always)]
pub unsafe fn sbi_call(
    extension: usize,
    function: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> (isize, usize) {
    let error: isize;
    let value: usize;
    
    core::arch::asm!(
        "ecall",
        in("a7") extension,
        in("a6") function,
        in("a0") arg0,
        in("a1") arg1,
        in("a2") arg2,
        in("a3") arg3,
        in("a4") arg4,
        in("a5") arg5,
        lateout("a0") error,
        lateout("a1") value,
    );
    
    (error, value)
}

/// Get SBI specification version
pub fn get_sbi_version() -> usize {
    unsafe {
        let (error, value) = sbi_call(
            ExtensionId::Base as usize,
            BaseFunction::GetSbiVersion as usize,
            0, 0, 0, 0, 0, 0,
        );
        if error == 0 {
            value
        } else {
            0
        }
    }
}

/// Probe an SBI extension
pub fn probe_extension(extension_id: usize) -> bool {
    unsafe {
        let (error, value) = sbi_call(
            ExtensionId::Base as usize,
            BaseFunction::ProbeExtension as usize,
            extension_id, 0, 0, 0, 0, 0,
        );
        error == 0 && value != 0
    }
}

/// Print a single character to console
pub fn console_putchar(ch: u8) {
    unsafe {
        sbi_call(
            ExtensionId::Console as usize,
            ConsoleFunction::PutChar as usize,
            ch as usize, 0, 0, 0, 0, 0,
        );
    }
}

/// Print a string to console
pub fn console_putstr(s: &str) {
    for ch in s.bytes() {
        console_putchar(ch);
    }
}

/// Get a character from console (non-blocking)
#[allow(dead_code)]
pub fn console_getchar() -> Option<u8> {
    unsafe {
        let (error, value) = sbi_call(
            ExtensionId::Console as usize,
            ConsoleFunction::GetChar as usize,
            0, 0, 0, 0, 0, 0,
        );
        if error == 0 && value != usize::MAX {
            Some(value as u8)
        } else {
            None
        }
    }
}

/// Set timer for next timer event
pub fn set_timer(stime_value: u64) {
    unsafe {
        let stime_lo = stime_value as usize;
        let stime_hi = (stime_value >> 32) as usize;
        sbi_call(
            ExtensionId::Timer as usize,
            TimerFunction::SetTimer as usize,
            stime_lo, stime_hi, 0, 0, 0, 0,
        );
    }
}

/// Get current time value
/// 
/// Note: This is a simplified implementation. In real hardware,
/// this should read from the memory-mapped mtime register
/// (typically at 0x200bff8 for QEMU virt machine)
pub fn get_time() -> u64 {
    // For QEMU virt machine, mtime is at 0x200bff8
    // This is a simplified version - in production, read from DTB
    const MTIME_ADDR: *const u64 = 0x200bff8 as *const u64;
    unsafe {
        core::ptr::read_volatile(MTIME_ADDR)
    }
}

/// System reset
pub fn system_reset(reset_type: ResetType, reset_reason: ResetReason) -> ! {
    unsafe {
        sbi_call(
            ExtensionId::Srst as usize,
            SystemResetFunction::SystemReset as usize,
            reset_type as usize,
            reset_reason as usize,
            0, 0, 0, 0,
        );
    }
    
    // If reset fails, loop forever
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

/// Initialize SBI environment
pub fn init() {
    // Check SBI version
    let version = get_sbi_version();
    let major = (version >> 24) & 0x7f;
    let minor = (version >> 16) & 0xff;
    
    console_putstr("SBI Version: ");
    print_usize(major);
    console_putstr(".");
    print_usize(minor);
    console_putstr("\n");
    
    // Probe extensions
    if probe_extension(ExtensionId::Console as usize) {
        console_putstr("Console extension: available\n");
    }
    if probe_extension(ExtensionId::Timer as usize) {
        console_putstr("Timer extension: available\n");
    }
    if probe_extension(ExtensionId::Srst as usize) {
        console_putstr("System Reset extension: available\n");
    }
}

/// Print a usize as decimal
fn print_usize(n: usize) {
    if n == 0 {
        console_putchar(b'0');
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
        console_putchar(digits[j]);
    }
}

