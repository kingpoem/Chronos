//! ELF file loader for user programs

use alloc::vec::Vec;

/// ELF file header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    pub magic: [u8; 4],
    pub class: u8,
    pub data: u8,
    pub version: u8,
    pub osabi: u8,
    pub abiversion: u8,
    pub pad: [u8; 7],
    pub typ: u16,
    pub machine: u16,
    pub version2: u32,
    pub entry: u64,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

/// ELF program header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProgramHeader {
    pub typ: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

/// Program header types
pub const PT_LOAD: u32 = 1;

/// Program header flags
pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

/// ELF magic number
pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// Parse ELF file and extract loadable segments
pub fn parse_elf(elf_data: &[u8]) -> Result<(usize, Vec<Segment>), &'static str> {
    if elf_data.len() < core::mem::size_of::<ElfHeader>() {
        return Err("ELF data too short");
    }

    // Parse ELF header
    let elf_header = unsafe { &*(elf_data.as_ptr() as *const ElfHeader) };

    // Verify magic number
    if elf_header.magic != ELF_MAGIC {
        return Err("Invalid ELF magic");
    }

    // Verify 64-bit ELF
    if elf_header.class != 2 {
        return Err("Not a 64-bit ELF");
    }

    // Verify RISC-V architecture
    if elf_header.machine != 0xF3 {
        return Err("Not a RISC-V ELF");
    }

    let entry = elf_header.entry as usize;
    let phoff = elf_header.phoff as usize;
    let phnum = elf_header.phnum as usize;

    // Parse program headers
    let mut segments = Vec::new();

    for i in 0..phnum {
        let phdr_offset = phoff + i * core::mem::size_of::<ProgramHeader>();
        if phdr_offset + core::mem::size_of::<ProgramHeader>() > elf_data.len() {
            return Err("Program header out of bounds");
        }

        let phdr =
            unsafe { &*((elf_data.as_ptr() as usize + phdr_offset) as *const ProgramHeader) };

        // Only process loadable segments
        if phdr.typ == PT_LOAD {
            let offset = phdr.offset as usize;
            let vaddr = phdr.vaddr as usize;
            let filesz = phdr.filesz as usize;
            let memsz = phdr.memsz as usize;
            let flags = phdr.flags;

            if offset + filesz > elf_data.len() {
                return Err("Segment data out of bounds");
            }

            segments.push(Segment {
                vaddr,
                memsz,
                data: elf_data[offset..offset + filesz].to_vec(),
                flags,
            });
        }
    }

    Ok((entry, segments))
}

/// Represents a loadable ELF segment
#[derive(Debug, Clone)]
pub struct Segment {
    pub vaddr: usize,
    pub memsz: usize,
    pub data: Vec<u8>,
    pub flags: u32,
}

impl Segment {
    /// Check if segment is readable
    pub fn is_readable(&self) -> bool {
        self.flags & PF_R != 0
    }

    /// Check if segment is writable
    pub fn is_writable(&self) -> bool {
        self.flags & PF_W != 0
    }

    /// Check if segment is executable
    pub fn is_executable(&self) -> bool {
        self.flags & PF_X != 0
    }
}
