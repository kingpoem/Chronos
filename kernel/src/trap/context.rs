use riscv::register::sstatus::{self, Sstatus};

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    // Store user satp for address space switching in __restore
    pub user_satp: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    
    pub fn user_init_context(entry: usize, sp: usize) -> Self {
        let sstatus = sstatus::read();
        unsafe {
            sstatus::set_spp(sstatus::SPP::User);
        }

        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            user_satp: 0,
        };
        cx.set_sp(sp);
        cx
    }
    
    /// Initialize trap context for app (with kernel token and stack)
    pub fn app_init_context(
        entry: usize,
        user_sp: usize,
        _kernel_satp: usize,
        _kernel_sp: usize,
        _trap_handler: usize,
    ) -> Self {
        // Read current sstatus and create a new one with SPP set to User and SIE enabled
        // This follows rCore's implementation: set SPP to User and enable interrupts (SIE bit)
        // so that when we restore it, we'll be in user mode with interrupts enabled
        let sstatus_val = unsafe {
            // Temporarily modify the register to get the correct value
            let original_bits: usize;
            core::arch::asm!("csrr {}, sstatus", out(reg) original_bits);
            
            // Set SPP to User (clear bit 8) and enable interrupts (set bit 1, SIE)
            // SPP bit (bit 8): 0 = User, 1 = Supervisor
            // SIE bit (bit 1): 0 = disabled, 1 = enabled
            let modified_bits = (original_bits & !(1 << 8)) | (1 << 1);
            core::arch::asm!("csrw sstatus, {}", in(reg) modified_bits);
            let result = sstatus::read();
            
            // Restore original value
            core::arch::asm!("csrw sstatus, {}", in(reg) original_bits);
            result
        };
        
        let mut cx = Self {
            x: [0; 32],
            sstatus: sstatus_val, // SPP is set to User, SIE is enabled
            sepc: entry,
            user_satp: 0,  // Will be set when task is created
        };
        cx.set_sp(user_sp);
        
        // NOTE: sscratch will be set when we actually switch to the task
        // Don't set it here to avoid side effects during task creation
        
        cx
    }
}
