//! Task (process) management module

mod context;
mod loader;
mod manager;
mod scheduler;
mod task;

pub use context::TaskContext;
pub use loader::load_apps;
pub use manager::TaskManager;
pub use scheduler::Scheduler;
pub use task::TaskStatus;

use crate::global_asm;
use lazy_static::*;
use spin::Mutex;

global_asm!(include_str!("switch.S"));

extern "C" {
    fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}

/// Helper function to jump to __restore with proper register assignment
/// Using a separate function prevents the compiler from optimizing incorrectly
#[inline(never)]
unsafe fn jump_to_restore(trap_cx: usize, restore_addr: usize) -> ! {
    use core::arch::asm;
    asm!(
        "mv a0, {trap_cx}",
        "jr {restore}",
        trap_cx = in(reg) trap_cx,
        restore = in(reg) restore_addr,
        options(noreturn)
    );
}

lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
    pub static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

pub fn init() {
    // Kernel address space should already be activated in mm::init()
    let _kernel_token = crate::mm::get_kernel_token();

    // Update KERNEL_SPACE to match the activated one
    // The actual kernel space is already created and activated in mm::init()
}

/// Switch to next task
pub fn switch_task() {
    let mut task_manager = TASK_MANAGER.lock();
    let mut scheduler = SCHEDULER.lock();

    let current_pid = task_manager.get_current_task();

    if let Some(current) = current_pid {
        // Mark current task as ready
        if let Some(task) = task_manager.get_task_mut(current) {
            task.task_status = TaskStatus::Ready;
        }
    }

    // Find next task
    let next_pid = if let Some(current) = current_pid {
        scheduler.schedule_next(current, &*task_manager)
    } else {
        scheduler.schedule_next(0, &*task_manager)
    };

    if let Some(next) = next_pid {
        // Mark next task as running
        if let Some(task) = task_manager.get_task_mut(next) {
            task.task_status = TaskStatus::Running;
        } else {
            panic!("Next task not found!");
        }

        task_manager.set_current_task(Some(next));

        let current_cx = if let Some(current) = current_pid {
            task_manager.get_task(current).map(|t| &t.task_cx as *const TaskContext)
        } else {
            None
        };

        let next_cx = task_manager.get_task(next).map(|t| &t.task_cx as *const TaskContext);

        if let (Some(curr_ptr), Some(next_ptr)) = (current_cx, next_cx) {
            // Task switch between running tasks
            let next_task = task_manager.get_task(next).unwrap();
            let next_kernel_sp = next_task.task_cx.sp;
            let next_user_token = next_task.get_user_token();

            let kernel_token = crate::mm::get_kernel_token();

            // Switch to kernel address space before accessing trap context
            unsafe {
                use core::arch::asm;
                asm!("csrw satp, {}", in(reg) kernel_token);
                asm!("sfence.vma");
            }

            // Save current task's trap context from kernel stack to its trap_cx page
            if let Some(current) = current_pid {
                if let Some(current_task) = task_manager.get_task(current) {
                    let current_trap_cx_kernel = (current_task.task_cx.sp
                        - core::mem::size_of::<crate::trap::TrapContext>())
                        as *mut crate::trap::TrapContext;
                    let current_trap_cx_data = unsafe { &*current_trap_cx_kernel };
                    let current_trap_cx_page = current_task.get_trap_cx();
                    *current_trap_cx_page = crate::trap::TrapContext {
                        x: current_trap_cx_data.x,
                        sstatus: current_trap_cx_data.sstatus,
                        sepc: current_trap_cx_data.sepc,
                        user_satp: current_trap_cx_data.user_satp,
                        kernel_sp: current_trap_cx_data.kernel_sp,
                    };
                }
            }

            let next_trap_cx_data = {
                let trap_cx = next_task.get_trap_cx();
                crate::trap::TrapContext {
                    x: trap_cx.x,
                    sstatus: trap_cx.sstatus,
                    sepc: trap_cx.sepc,
                    user_satp: trap_cx.user_satp,
                    kernel_sp: trap_cx.kernel_sp,
                }
            };

            drop(task_manager);
            drop(scheduler);

            unsafe {
                let next_trap_cx_kernel = (next_kernel_sp
                    - core::mem::size_of::<crate::trap::TrapContext>())
                    as *mut crate::trap::TrapContext;

                *next_trap_cx_kernel = next_trap_cx_data;
                (*next_trap_cx_kernel).user_satp = next_user_token;

                __switch(
                    curr_ptr as *mut TaskContext,
                    next_ptr,
                );
            }
        } else {
            // First task - need to set up trap context and jump to __restore
            let kernel_sp = task_manager.get_task(next).unwrap().task_cx.sp;
            let kernel_token = crate::mm::get_kernel_token();
            
            let (entry_point, user_sp, user_token, kernel_stack_top) = {
                let task = task_manager.get_task(next).unwrap();
                (task.entry_point, task.user_sp, task.get_user_token(), task.task_cx.sp)
            };

            extern "C" {
                fn trap_handler();
            }

            // Disable interrupts during context setup
            unsafe {
                use riscv::register::{sie, sstatus};
                sstatus::clear_sie();
                sie::clear_stimer();
            }

            let mut trap_cx_data = crate::trap::TrapContext::app_init_context(
                entry_point,
                user_sp,
                kernel_token,
                kernel_stack_top,
                trap_handler as *const () as usize,
            );
            trap_cx_data.user_satp = user_token;
            trap_cx_data.kernel_sp = kernel_stack_top;

            // Re-enable timer interrupt
            unsafe {
                use riscv::register::{sie, sstatus};
                sstatus::clear_sie();
                sie::set_stimer();
            }

            drop(task_manager);
            drop(scheduler);

            unsafe {
                use core::arch::asm;

                // Switch to kernel address space
                asm!("csrw satp, {}", in(reg) kernel_token);
                asm!("sfence.vma");

                // Prepare trap context on kernel stack
                let trap_cx_size = core::mem::size_of::<crate::trap::TrapContext>();
                let trap_cx_kernel = (kernel_sp - trap_cx_size) as *mut crate::trap::TrapContext;

                // Write trap context to kernel stack
                *trap_cx_kernel = trap_cx_data;
                (*trap_cx_kernel).user_satp = user_token;

                // Set the first timer before switching to user mode
                crate::trap::set_next_timer();

                // Get __restore's trampoline address from trap module
                let __restore_trampoline_addr = crate::trap::get_restore_trampoline_addr();

                // Jump to __restore at trampoline address
                // Use a helper function to force correct register assignment
                jump_to_restore(trap_cx_kernel as usize, __restore_trampoline_addr);
            }
        }
    } else {
        // No tasks to run
        let kernel_token = crate::mm::get_kernel_token();
        unsafe {
            use core::arch::asm;
            asm!("csrw satp, {}", in(reg) kernel_token);
            asm!("sfence.vma");
        }
    }
}

/// Exit current task and run next
pub fn exit_current_and_run_next(_exit_code: i32) {
    let mut task_manager = TASK_MANAGER.lock();
    let current_pid = task_manager.get_current_task();

    if let Some(pid) = current_pid {
        task_manager.mark_zombie(pid);
        task_manager.remove_task(pid);
        task_manager.set_current_task(None);

        // If no more tasks, shutdown (disable timer interrupt first)
        if task_manager.task_count() == 0 {
            drop(task_manager);

            // Disable timer interrupt before shutdown
            unsafe {
                use riscv::register::{sie, sstatus};
                sstatus::clear_sie(); // Disable interrupts
                sie::clear_stimer(); // Disable timer interrupt
            }

            crate::sbi::shutdown(); // shutdown() returns !, so code after this is unreachable
        }
    }

    drop(task_manager);
    switch_task();
}
