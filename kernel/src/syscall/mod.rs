//! System call handling module
use core::arch::asm;

mod fs;
mod process;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm! {
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") syscall_id
        };
    }
    ret
}
