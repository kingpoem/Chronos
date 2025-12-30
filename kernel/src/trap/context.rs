//! Trap context for user-kernel transitions

use riscv::register::sstatus::{Sstatus, SPP};

/// Trap context saved on trap entry
#[repr(C)]
pub struct TrapContext {
    /// General registers x0..x31
    pub x: [usize; 32],
    /// Supervisor status register (stored as usize for alignment)
    pub sstatus: usize,
    /// Supervisor exception program counter
    pub sepc: usize,
    /// Kernel satp (page table)
    pub kernel_satp: usize,
    /// Kernel stack pointer
    pub kernel_sp: usize,
    /// Trap handler entry point
    pub trap_handler: usize,
}

impl TrapContext {
    /// Initialize trap context for a new app
    pub fn app_init_context(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        // We need to manually construct an sstatus value with SPP=User
        // SPP is bit 8: 0 = User, 1 = Supervisor
        // SPIE is bit 5: 1 = enable interrupts when returning
        // SUM is bit 18: 0 = kernel cannot access user pages (correct for security)
        let sstatus_bits: usize = 1 << 5; // SPIE=1, SPP=0, SUM=0

        let mut cx = Self {
            x: [0; 32],
            sstatus: sstatus_bits,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            trap_handler,
        };
        cx.set_sp(sp);
        cx
    }

    /// Set stack pointer (x2)
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
}

/// Trap handler function (will be called from assembly)
fn trap_handler() -> ! {
    crate::trap::trap_from_user();
}
