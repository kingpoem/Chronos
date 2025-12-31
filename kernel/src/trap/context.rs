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
        kernel_sp: usize,
        _trap_handler: usize,
    ) -> Self {
        let sstatus = sstatus::read();
        let mut new_sstatus = sstatus;
        unsafe {
            sstatus::set_spp(sstatus::SPP::User);
        }
        
        let mut cx = Self {
            x: [0; 32],
            sstatus: new_sstatus,
            sepc: entry,
            user_satp: 0,  // Will be set when task is created
        };
        cx.set_sp(user_sp);
        
        // Set sscratch to kernel stack for trap handling
        unsafe {
            core::arch::asm!("csrw sscratch, {}", in(reg) kernel_sp);
        }
        
        cx
    }
}
