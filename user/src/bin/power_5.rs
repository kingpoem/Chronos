#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::sys_yield;

#[no_mangle]
fn main() {
    println!("power_5 begin");
    let mut i = 0;
    while i < 5 {
        print!("power_5: {}\n", i);
        i += 1;
        sys_yield();
    }
    println!("power_5 OK!");
}
