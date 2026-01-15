#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::sys_yield;

#[no_mangle]
fn main() {
    println!("Test simple begin");
    for i in 0..10 {
        print!("Test simple: {}\n", i);
        sys_yield();
    }
    println!("Test simple OK!");
}
