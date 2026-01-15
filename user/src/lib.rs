#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]

use core::arch::asm;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        // Using $crate to refer to the current crate is required
        // when the macro is exported and used in other crates
        $crate::console::print(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    ($fmt:literal $(, $($arg:tt)*)?) => {
        $crate::print!(concat!($fmt, "\n") $(, $($arg)*)?);
    }
}

pub mod console {
    use core::fmt::{self, Write};

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            // syscall 64 is write, fd 1 is stdout
            let buffer = s.as_bytes();
            unsafe {
                super::syscall(64, 1, buffer.as_ptr() as usize, buffer.len());
            }
            Ok(())
        }
    }

    pub fn print(args: fmt::Arguments) {
        Stdout.write_fmt(args).unwrap();
    }
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

fn clear_bss() {
    extern "C" {
        fn start_bss();
        fn end_bss();
    }
    (start_bss as usize..end_bss as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("Panicked: {}", info.message());
    }
    exit(-1);
    loop {}
}

// System calls
pub fn syscall(id: usize, args0: usize, args1: usize, args2: usize) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args0 => ret,
            in("x11") args1,
            in("x12") args2,
            in("x17") id,
        );
    }
    ret
}

pub fn exit(exit_code: i32) -> isize {
    syscall(93, exit_code as usize, 0, 0)
}

pub fn write(fd: usize, buffer: &[u8]) -> isize {
    syscall(64, fd, buffer.as_ptr() as usize, buffer.len())
}

pub fn yield_() -> isize {
    syscall(124, 0, 0, 0)
}

pub fn get_time() -> isize {
    syscall(169, 0, 0, 0)
}
