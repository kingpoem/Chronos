//! Task Context
//! 
//! Defines the context structure for task switching


/// Task context for context switching
#[repr(C)]
pub struct TaskContext {
    /// Return address (ra)
    pub ra: usize,
    /// Stack pointer (sp)
    pub sp: usize,
    /// Saved registers s0-s11
    pub s: [usize; 12],
}

impl TaskContext {
    /// Create a zero-initialized context
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    
    /// Create a context that will jump to trap_return via trampoline
    /// The trap context should be prepared on kernel stack before calling __restore
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        // Get __restore's trampoline address from trap module
        let restore_trampoline_addr = crate::trap::get_restore_trampoline_addr();
        
        Self {
            ra: restore_trampoline_addr,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
