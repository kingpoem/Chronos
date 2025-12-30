//! Task Control Block
//! 
//! Defines the structure and operations for tasks (processes)

use super::context::TaskContext;
use crate::config::*;
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, FRAME_ALLOCATOR};
use crate::trap::TrapContext;
use alloc::vec::Vec;

/// Task状态
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

/// Task Control Block
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
    pub heap_bottom: usize,
    pub program_brk: usize,
}

impl TaskControlBlock {
    /// Get trap context
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.as_ptr::<TrapContext>() as &'static mut TrapContext
    }
    
    /// Get user token (satp value)
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    
    /// Create a new task from ELF data
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // Parse ELF - for now, we'll load it as a simple binary
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(TRAP_CONTEXT.into())
            .unwrap()
            .into();
        let task_status = TaskStatus::Ready;
        
        // Allocate kernel stack
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        let task_cx = TaskContext::goto_trap_return(kernel_stack_top);
        
        let mut task = Self {
            task_status,
            task_cx,
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
        };
        
        // Initialize trap context
        let trap_cx = task.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task
    }
}

/// Get kernel stack position for app
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

lazy_static! {
    static ref KERNEL_SPACE: spin::Mutex<MemorySet> = spin::Mutex::new(MemorySet::new_kernel());
}

extern "C" {
    fn trap_handler();
}

use crate::config::PAGE_SIZE;
use lazy_static::*;
