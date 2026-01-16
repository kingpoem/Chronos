use std::env;
use std::path::PathBuf;

fn main() {
    // Tell Cargo to re-run this build script if linker script changes
    println!("cargo:rerun-if-changed=linker.ld");
    
    // Set linker script path
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let linker_script = manifest_dir.join("linker.ld");
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
}

