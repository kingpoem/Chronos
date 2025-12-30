//! Task Control Block
//!
//! Defines the structure and operations for tasks (processes)

use super::context::TaskContext;
use super::UPSafeCell;
use crate::config::memory_layout::{KERNEL_STACK_SIZE, PAGE_SIZE};
use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::mm::memory_layout::{PhysPageNum, PhysAddr};
use crate::mm::MemorySet;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;
use crate::println;

/// Task State
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

pub struct TaskControlBlock {
    // Immutable
    pub pid: usize,
    pub kernel_stack: usize,
    // Mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
    pub heap_bottom: usize,
    pub program_brk: usize,
    pub exit_code: i32,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        unsafe { &mut *(self.trap_cx_ppn.as_ptr::<TrapContext>()) }
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn get_status(&self) -> TaskStatus {
        self.task_status
    }

    pub fn is_zombie(&self) -> bool {
        self.task_status == TaskStatus::Zombie
    }
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> core::cell::RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn getpid(&self) -> usize {
        self.pid
    }

    /// Create a new task from ELF data
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // Parse ELF
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        let trap_cx_ppn = PhysAddr::new(
            memory_set
                .translate(TRAP_CONTEXT)
                .unwrap(),
        )
        .page_number();
        let task_status = TaskStatus::Ready;

        // Allocate kernel stack
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        
        // Map kernel stack in the KERNEL_SPACE
        use crate::mm::memory_set::{MapArea, MapPermission, MapType};
        let mut kernel_space = crate::mm::KERNEL_SPACE.lock();
        kernel_space.push(
            MapArea::new(
                kernel_stack_bottom,
                kernel_stack_top,
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        drop(kernel_space);
        
        let task_cx = TaskContext::goto_trap_return(kernel_stack_top);

        let task_control_block = Self {
            pid: app_id,
            kernel_stack: kernel_stack_top,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    task_status,
                    task_cx,
                    memory_set,
                    trap_cx_ppn,
                    base_size: user_sp,
                    heap_bottom: user_sp,
                    program_brk: user_sp,
                    exit_code: 0,
                })
            },
        };

        // Initialize trap context
        let mut inner = task_control_block.inner_exclusive_access();
        let trap_cx = inner.get_trap_cx();
        let kernel_token = crate::mm::KERNEL_SPACE.lock().token();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            kernel_token,
            kernel_stack_top,
            trap_handler as usize,
        );
        drop(inner);
        task_control_block
    }

}

/// Get kernel stack position for app
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - (app_id + 1) * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

extern "C" {
    fn trap_handler();
}
