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
pub const SYS_MMAP: usize = 222;
pub const SYS_MUNMAP: usize = 215;

/// System call wrapper functions

#[inline(always)]
fn syscall_3(id: usize, args: [usize; 3]) -> isize {
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

#[inline(always)]
fn syscall_6(id: usize, a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
            in("a5") a5,
            in("a7") id,
        );
    }
    ret
}

/// Write to console
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    syscall_3(SYS_WRITE, [fd, buf.as_ptr() as usize, buf.len()])
}

/// Exit with code
pub fn sys_exit(exit_code: i32) -> ! {
    syscall_3(SYS_EXIT, [exit_code as usize, 0, 0]);
    unreachable!()
}

/// Yield CPU
pub fn sys_yield() -> isize {
    syscall_3(SYS_YIELD, [0, 0, 0])
}

/// Get time in microseconds
pub fn sys_get_time() -> isize {
    syscall_3(SYS_GET_TIME, [0, 0, 0])
}

/// Protection flags for mmap
pub const PROT_READ: usize = 0x1;
pub const PROT_WRITE: usize = 0x2;
pub const PROT_EXEC: usize = 0x4;

/// Mapping flags for mmap
pub const MAP_PRIVATE: usize = 0x02;
pub const MAP_SHARED: usize = 0x01;
pub const MAP_ANONYMOUS: usize = 0x20;
pub const MAP_FIXED: usize = 0x10;

/// Error return value for mmap
pub const MAP_FAILED: isize = -1;

/// Map memory region
/// 
/// # Arguments
/// * `addr` - Suggested virtual address (0 means let kernel choose)
/// * `length` - Size of mapping in bytes
/// * `prot` - Protection flags (PROT_READ, PROT_WRITE, PROT_EXEC)
/// * `flags` - Mapping flags (MAP_PRIVATE, MAP_SHARED, MAP_ANONYMOUS, MAP_FIXED)
/// * `fd` - File descriptor (ignored for anonymous mappings, use -1)
/// * `offset` - File offset (ignored for anonymous mappings, use 0)
/// 
/// # Returns
/// * Success: Virtual address of mapped region
/// * Failure: MAP_FAILED (-1)
pub fn sys_mmap(
    addr: usize,
    length: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    offset: usize,
) -> isize {
    syscall_6(SYS_MMAP, addr, length, prot, flags, fd, offset)
}

/// Unmap memory region
/// 
/// # Arguments
/// * `addr` - Virtual address of mapped region (must be page-aligned)
/// * `length` - Size of region to unmap in bytes
/// 
/// # Returns
/// * Success: 0
/// * Failure: -1
pub fn sys_munmap(addr: usize, length: usize) -> isize {
    syscall_6(SYS_MUNMAP, addr, length, 0, 0, 0, 0)
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

