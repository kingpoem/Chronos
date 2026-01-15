#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::{print, println, print_num, sys_yield};

#[no_mangle]
fn main() {
    println("power_5 begin");
    let mut i = 0;
    while i < 5 {
        print("power_5: ");
        print_num(i);
        print("\n");
        i += 1;
        sys_yield();
    }
    println("power_5 OK!");
}
