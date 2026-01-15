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
        // According to rCore implementation: use set_spp and set_sie, then read
        // CRITICAL: This function assumes interrupts are disabled and timer is NOT set
        // The caller must ensure these conditions before calling this function
        let sstatus_val = unsafe {
            // Save original sstatus bits
            let original_bits: usize;
            core::arch::asm!("csrr {}, sstatus", out(reg) original_bits);
            
            // Set SPP to User and SIE to enabled (modifies actual register)
            // This is the rCore way: modify register to get the value we want
            // NOTE: This will temporarily enable interrupts, so timer must NOT be set
            sstatus::set_spp(sstatus::SPP::User);
            sstatus::set_sie();
            let result = sstatus::read(); // Read the modified value
            
            // CRITICAL: Restore original sstatus immediately
            // Use inline assembly for fastest possible restoration
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
