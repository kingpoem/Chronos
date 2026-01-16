#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::sys_yield;

#[no_mangle]
fn main() {
    println!("power_3 begin");
    let mut i = 0;
    while i < 3 {
        print!("power_3: {}\n", i);
        i += 1;
        sys_yield();
    }
    println!("power_3 OK!");
}
