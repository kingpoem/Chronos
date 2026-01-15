#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::sys_yield;

#[no_mangle]
fn main() {
    println!("power_7 begin");
    let mut i = 0;
    while i < 7 {
        print!("power_7: {}\n", i);
        i += 1;
        sys_yield();
    }
    println!("power_7 OK!");
}
