//! Task context switch
//!
//! Assembly implementation of context switching

use super::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

extern "C" {
    /// Switch from current task context to next task context
    ///
    /// # Arguments
    /// * `current_task_cx_ptr` - Pointer to current task's TaskContext
    /// * `next_task_cx_ptr` - Pointer to next task's TaskContext
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}
