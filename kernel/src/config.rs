//! Kernel configuration constants

pub mod memory_layout {
    /// User stack size (8KB)
    pub const USER_STACK_SIZE: usize = 4096 * 2;

    /// Kernel stack size (16KB)
    pub const KERNEL_STACK_SIZE: usize = 4096 * 4;

    /// Kernel heap size (8MB)
    pub const KERNEL_HEAP_SIZE: usize = 0x80_0000;

    /// Physical memory end (128MB for QEMU virt)
    pub const MEMORY_END: usize = 0x8800_0000;

    /// Page size (4KB)
    pub const PAGE_SIZE: usize = 0x1000;

    /// Page size bits
    pub const PAGE_SIZE_BITS: usize = 12;

    /// Physical memory start address (QEMU virt machine)
    pub const MEMORY_START: usize = 0x8000_0000;

    /// Kernel code start (defined by linker)
    pub const KERNEL_START: usize = 0x8022_0000;

    /// Kernel code end (assume kernel + data < 2MB, heap starts at 2MB offset)
    pub const KERNEL_END: usize = 0x8042_0000;

    /// Kernel heap start (2MB from kernel base)
    pub const KERNEL_HEAP_START: usize = 0x8042_0000;

    /// Kernel heap end
    pub const KERNEL_HEAP_END: usize = KERNEL_HEAP_START + KERNEL_HEAP_SIZE;
}

/// Max number of apps
pub const MAX_APP_NUM: usize = 16;

/// Max syscall number
pub const MAX_SYSCALL_NUM: usize = 500;

/// Clock frequency (10MHz for QEMU)
pub const CLOCK_FREQ: usize = 10_000_000;
