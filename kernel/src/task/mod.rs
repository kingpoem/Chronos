//! Task (process) management module
<<<<<<< HEAD
=======

mod context;

pub use context::TaskContext;
use crate::{global_asm, println};

global_asm!(include_str!("switch.S"));

extern "C" {
    fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}

/// Simple task structure for testing
pub struct SimpleTask {
    id: usize,
    context: TaskContext,
}

static mut TASKS: [Option<SimpleTask>; 2] = [None, None];
static mut CURRENT_TASK: usize = 0;
>>>>>>> c32c54c (feat: task subsystem.)

pub fn init() {
    println!("[Task] Task management initialized");
}

/// Switch to next task (for testing)
pub fn switch_task() {
    unsafe {
        let current = CURRENT_TASK;
        let next = (current + 1) % 2;
        
        if let (Some(curr_task), Some(next_task)) = (&mut TASKS[current], &TASKS[next]) {
            CURRENT_TASK = next;
            __switch(
                &mut curr_task.context as *mut TaskContext,
                &next_task.context as *const TaskContext,
            );
        }
    }
}
