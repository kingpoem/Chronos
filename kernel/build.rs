fn main() {
    println!("cargo:rerun-if-changed=src/link_app.S");
    println!("cargo:rerun-if-changed=../user/target/riscv64gc-unknown-none-elf/release/");
}
