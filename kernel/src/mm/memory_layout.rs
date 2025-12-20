//! Memory Layout Definitions
//!
//! Defines the memory layout for QEMU virt machine (RISC-V 64-bit)

/// Page size: 4KB
pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SIZE_BITS: usize = 12;

/// Physical memory start address (QEMU virt machine)
pub const MEMORY_START: usize = 0x8000_0000;

/// Physical memory end address (128MB)
pub const MEMORY_END: usize = 0x8800_0000;

/// Kernel code start (defined by linker)
pub const KERNEL_START: usize = 0x8022_0000;

/// Kernel code end (assume kernel + data < 2MB, heap starts at 2MB offset)
pub const KERNEL_END: usize = 0x8042_0000;

/// Kernel heap start (2MB from kernel base)
pub const KERNEL_HEAP_START: usize = 0x8042_0000;

/// Kernel heap size (8MB)
pub const KERNEL_HEAP_SIZE: usize = 0x80_0000;

/// Kernel heap end
pub const KERNEL_HEAP_END: usize = KERNEL_HEAP_START + KERNEL_HEAP_SIZE;

/// Convert physical address to page number
#[inline]
pub const fn pa_to_ppn(pa: usize) -> usize {
    pa >> PAGE_SIZE_BITS
}

/// Convert page number to physical address
#[inline]
pub const fn ppn_to_pa(ppn: usize) -> usize {
    ppn << PAGE_SIZE_BITS
}

/// Convert virtual address to page number
#[inline]
pub const fn va_to_vpn(va: usize) -> usize {
    va >> PAGE_SIZE_BITS
}

/// Convert page number to virtual address
#[inline]
pub const fn vpn_to_va(vpn: usize) -> usize {
    vpn << PAGE_SIZE_BITS
}

/// Get page offset from address
#[inline]
pub const fn page_offset(addr: usize) -> usize {
    addr & (PAGE_SIZE - 1)
}

/// Align address up to page size
#[inline]
pub const fn align_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Align address down to page size
#[inline]
pub const fn align_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

/// Physical address type
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn page_number(&self) -> PhysPageNum {
        PhysPageNum(self.0 >> PAGE_SIZE_BITS)
    }

    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

/// Physical page number type
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct PhysPageNum(pub usize);

impl PhysPageNum {
    pub fn new(ppn: usize) -> Self {
        Self(ppn)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn addr(&self) -> PhysAddr {
        PhysAddr(self.0 << PAGE_SIZE_BITS)
    }

    pub fn as_ptr<T>(&self) -> *mut T {
        (self.0 << PAGE_SIZE_BITS) as *mut T
    }
}

/// Virtual address type
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn page_number(&self) -> VirtPageNum {
        VirtPageNum(self.0 >> PAGE_SIZE_BITS)
    }

    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}

/// Virtual page number type
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct VirtPageNum(pub usize);

impl VirtPageNum {
    pub fn new(vpn: usize) -> Self {
        Self(vpn)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn addr(&self) -> VirtAddr {
        VirtAddr(self.0 << PAGE_SIZE_BITS)
    }

    /// Get indexes for 3-level page table (SV39)
    pub fn indexes(&self) -> [usize; 3] {
        let vpn = self.0;
        [
            (vpn >> 18) & 0x1FF, // Level 2
            (vpn >> 9) & 0x1FF,  // Level 1
            vpn & 0x1FF,         // Level 0
        ]
    }
}
