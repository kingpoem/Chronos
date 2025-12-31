use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../user");
    println!("cargo:rerun-if-changed=build.rs");
    
    let target = "riscv64gc-unknown-none-elf";
    let mode = "release";
    
    // Build user programs
    println!("Building user programs...");
    let status = std::process::Command::new("cargo")
        .args(&["build", "--target", target, "--release"])
        .current_dir("../user")
        .status()
        .expect("Failed to build user programs");
    
    if !status.success() {
        panic!("Failed to build user programs");
    }
    
    // Collect user program ELF binaries
    // Note: We keep the ELF format intact - no stripping of ELF headers
    // The kernel's MemorySet::from_elf() will parse the ELF structure
    // and extract segments for segment-based paging
    let user_target_dir = format!("../user/target/{}/{}", target, mode);
    let apps = [
        "hello_world",
        "power_3",
        "power_5",
        "power_7",
        "test_simple",
    ];
    
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    
    // Write app ELF binaries to OUT_DIR and generate assembly file
    // Reference: rCore uses .S assembly file to embed binaries
    let asm_file = out_dir.join("link_apps.S");
    let mut f = fs::File::create(&asm_file).unwrap();
    
    // Write assembly file header
    writeln!(f, ".align 3").unwrap();
    writeln!(f, ".section .data.apps").unwrap();
    writeln!(f, ".global _num_app").unwrap();
    writeln!(f, "_num_app:").unwrap();
    writeln!(f, ".quad {}", apps.len()).unwrap();
    
    // First pass: write app pointers
    for (i, app) in apps.iter().enumerate() {
        // Read ELF file (not binary - ELF format is preserved)
        let elf_path = format!("{}/{}", user_target_dir, app);
        if !PathBuf::from(&elf_path).exists() {
            panic!("User program ELF {} not found at {}", app, elf_path);
        }
        
        // Copy ELF binary to OUT_DIR (keeping ELF structure intact)
        let elf_binary = fs::read(&elf_path).expect(&format!("Failed to read ELF {}", app));
        let elf_file = out_dir.join(app);
        fs::write(&elf_file, &elf_binary).unwrap();
        
        // Generate linker script entries for app pointers
        writeln!(f, ".quad _app_{}_start", i).unwrap();
        writeln!(f, ".quad _app_{}_end", i).unwrap();
    }
    
    // Second pass: write app binary data
    for (i, app) in apps.iter().enumerate() {
        writeln!(f, ".global _app_{}_start", i).unwrap();
        writeln!(f, ".section .data.app_{}", i).unwrap();
        writeln!(f, ".align 3").unwrap();
        writeln!(f, "_app_{}_start:", i).unwrap();
        // Use .incbin to include the binary file
        // The path is relative to OUT_DIR, so we need the full path
        let elf_file = out_dir.join(app);
        writeln!(f, ".incbin \"{}\"", elf_file.display()).unwrap();
        writeln!(f, "_app_{}_end:", i).unwrap();
    }
    
    // The assembly file will be included via global_asm! in loader.rs
    // No need to generate a separate Rust file
}
