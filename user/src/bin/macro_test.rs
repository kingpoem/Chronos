#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    // Test basic println
    println!("=== Macro Test Program ===");
    
    // Test print without newline
    print!("Testing print: ");
    println!("OK");
    
    // Test format arguments
    println!("Number formatting: {}", 42);
    println!("Multiple args: {} + {} = {}", 1, 2, 3);
    println!("Hex: 0x{:x}", 255);
    
    // Test variables
    let x = 100;
    let y = 200;
    println!("Variables: x={}, y={}", x, y);
    
    // Test expressions
    println!("Expression: 5 * 6 = {}", 5 * 6);
    
    println!("=== All tests passed! ===");
    0
}
