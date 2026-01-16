use riscv::register::sstatus::{self, Sstatus};

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    // Store user satp for address space switching in __restore
    pub user_satp: usize,
    // Store kernel stack pointer for next trap entry
    // This will be loaded into sscratch before sret to user mode
    pub kernel_sp: usize,
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
            kernel_sp: 0,  // Will be set when needed
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
        // Create sstatus value for returning to user mode:
        // - SPP = User (bit 8 = 0): sret will return to user mode
        // - SPIE = 1 (bit 5): sret will copy SPIE to SIE, enabling interrupts
        // - SIE = 0 (bit 1): doesn't matter, will be overwritten by sret
        //
        // The sret instruction does:
        // 1. SIE = SPIE (enable interrupts if SPIE was set)
        // 2. SPIE = 1
        // 3. SPP = 0 (user mode)
        // 4. PC = sepc (jump to user code)
        let sstatus_val = unsafe {
            // Save original sstatus bits
            let original_bits: usize;
            core::arch::asm!("csrr {}, sstatus", out(reg) original_bits);
            
            // Modify sstatus register to get the value we want
            sstatus::set_spp(sstatus::SPP::User);  // SPP = User
            sstatus::set_spie();  // SPIE = 1 (CRITICAL: this is what enables interrupts after sret!)
            let result = sstatus::read(); // Read the modified value
            
            // CRITICAL: Restore original sstatus immediately
            core::arch::asm!("csrw sstatus, {}", in(reg) original_bits);
            
            result
        };
        
        let mut cx = Self {
            x: [0; 32],
            sstatus: sstatus_val, // SPP is set to User, SPIE is set (enables interrupts after sret)
            sepc: entry,
            user_satp: 0,  // Will be set when task is created
            kernel_sp: 0,  // Will be set when task is created
        };
        cx.set_sp(user_sp);
        
        // NOTE: sscratch will be set when we actually switch to the task
        // Don't set it here to avoid side effects during task creation
        
        cx
    }
}
