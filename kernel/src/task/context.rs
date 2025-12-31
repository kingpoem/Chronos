//! Task Context
//! 
//! Defines the context structure for task switching

use crate::trap::TrapContext;

/// Task context for context switching
#[repr(C)]
pub struct TaskContext {
    /// Return address (ra)
    ra: usize,
    /// Stack pointer (sp)
    sp: usize,
    /// Saved registers s0-s11
    s: [usize; 12],
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
    
    /// Create a context that will jump to trap_return
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        extern "C" {
            fn __restore();
        }
        Self {
            ra: __restore as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
