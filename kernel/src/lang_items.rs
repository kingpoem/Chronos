//! Language items and panic handler

use crate::{println, sbi};
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "\n[Kernel Panic] at {}:{} {:?}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("\n[Kernel Panic] {:?}", info.message());
    }
    sbi::shutdown()
}
