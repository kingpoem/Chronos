#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::{print, println, print_num, sys_yield};

#[no_mangle]
fn main() {
    println("Test simple begin");
    for i in 0..10 {
        print("Test simple: ");
        print_num(i);
        print("\n");
        sys_yield();
    }
    println("Test simple OK!");
}
