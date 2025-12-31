#![no_std]
#![feature(linkage)]
#![no_main]

mod lang_items;

use core::arch::asm;

/// System call numbers
pub const SYS_WRITE: usize = 64;
pub const SYS_EXIT: usize = 93;
pub const SYS_YIELD: usize = 124;
pub const SYS_GET_TIME: usize = 169;

/// System call wrapper functions

#[inline(always)]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") args[0] => ret,
            in("a1") args[1],
            in("a2") args[2],
            in("a7") id,
        );
    }
    ret
}

/// Write to console
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    syscall(SYS_WRITE, [fd, buf.as_ptr() as usize, buf.len()])
}

/// Exit with code
pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYS_EXIT, [exit_code as usize, 0, 0]);
    unreachable!()
}

/// Yield CPU
pub fn sys_yield() -> isize {
    syscall(SYS_YIELD, [0, 0, 0])
}

/// Get time in microseconds
pub fn sys_get_time() -> isize {
    syscall(SYS_GET_TIME, [0, 0, 0])
}

/// Print string
pub fn print(s: &str) {
    sys_write(1, s.as_bytes());
}

/// Print string with newline
pub fn println(s: &str) {
    print(s);
    print("\n");
}

/// Print number
pub fn print_num(n: usize) {
    if n == 0 {
        print("0");
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
        let mut buf = [0u8; 1];
        buf[0] = digits[j];
        sys_write(1, &buf);
    }
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    extern "C" {
        fn main();
    }
    unsafe {
        main();
    }
    sys_exit(0)
}

