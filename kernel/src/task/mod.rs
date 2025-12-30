//! Task (process) management module

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

pub use context::TaskContext;
pub use task::{TaskControlBlock, TaskStatus};

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::cell::RefCell;
use lazy_static::lazy_static;

/// Safe wrapper for static mut
pub struct UPSafeCell<T> {
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    pub fn exclusive_access(&self) -> core::cell::RefMut<'_, T> {
        self.inner.borrow_mut()
    }

    pub fn shared_access(&self) -> core::cell::Ref<'_, T> {
        self.inner.borrow()
    }
}

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
    static ref CURRENT_TASK: UPSafeCell<Option<Arc<TaskControlBlock>>> =
        unsafe { UPSafeCell::new(None) };
}

pub fn init() {
    // TODO: Load and initialize initial tasks
}

/// Add a task to the ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

/// Get current running task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    CURRENT_TASK.shared_access().clone()
}

/// Set current running task
pub fn set_current_task(task: Option<Arc<TaskControlBlock>>) {
    *CURRENT_TASK.exclusive_access() = task;
}

/// Suspend current task and run next task
pub fn suspend_current_and_run_next() {
    let current_task_ctx = current_task();

    if let Some(task) = current_task_ctx {
        let mut task_inner = task.inner_exclusive_access();
        task_inner.task_status = TaskStatus::Ready;
        drop(task_inner);
        add_task(task);
    }

    run_next_task();
}

/// Exit current task and run next task
pub fn exit_current_and_run_next(exit_code: i32) {
    {
        let current = current_task();
        if let Some(task) = current {
            let mut task_inner = task.inner_exclusive_access();
            task_inner.task_status = TaskStatus::Zombie;
            task_inner.exit_code = exit_code;
        }
    }

    set_current_task(None);
    run_next_task();
}

/// Run the next task from ready queue
fn run_next_task() {
    let mut task = None;
    {
        let mut task_manager = TASK_MANAGER.exclusive_access();
        task = task_manager.fetch();
    }

    if let Some(next) = task {
        let mut task_inner = next.inner_exclusive_access();
        let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
        task_inner.task_status = TaskStatus::Running;
        drop(task_inner);

        set_current_task(Some(next));

        let mut unused = TaskContext::zero_init();
        unsafe {
            switch::__switch(&mut unused as *mut TaskContext, next_task_cx_ptr);
        }
    } else {
        crate::println!("[Task] No more tasks to run, shutting down");
        crate::sbi::shutdown();
    }
}

/// Run the first task
pub fn run_first_task() -> ! {
    run_next_task();
    panic!("run_first_task should never return!");
}
