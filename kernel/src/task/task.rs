//! Task Control Block
//! 
//! Defines the structure and operations for tasks (processes)

use super::context::TaskContext;
use crate::mm::MemorySet;
use crate::mm::memory_layout::PhysPageNum;
use crate::config::memory_layout::{KERNEL_STACK_SIZE, PAGE_SIZE};
use crate::trap::TrapContext;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

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
        unsafe { &mut *(self.trap_cx_ppn.as_ptr::<TrapContext>()) }
    }
    
    /// Get user token (satp value)
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    
    /// Create a new task from ELF data
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        crate::println!("[Task] Creating task from ELF data (size: {} bytes)", elf_data.len());
        // Parse ELF - for now, we'll load it as a simple binary
        crate::println!("[Task] Parsing ELF...");
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        crate::println!("[Task] ELF parsed: entry_point={:#x}, user_sp={:#x}", entry_point, user_sp);
        // Trap context is stored in user address space at TRAP_CONTEXT
        let trap_cx_ppn = PhysPageNum::new(
            memory_set
                .translate(TRAP_CONTEXT)
                .expect("Failed to translate TRAP_CONTEXT address")
                >> PAGE_SIZE.trailing_zeros() as usize
        );
        let task_status = TaskStatus::Ready;
        
        // Allocate kernel stack
        let (_kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        let task_cx = TaskContext::goto_trap_return(kernel_stack_top);
        
        let task = Self {
            task_status,
            task_cx,
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
        };
        
        // Initialize trap context
        let user_token = task.memory_set.token();
        let trap_cx = task.get_trap_cx();
        let mut trap_context = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            kernel_stack_top,
            trap_handler as *const () as usize,
        );
        // Store user token in trap context for __restore to use
        trap_context.user_satp = user_token;
        *trap_cx = trap_context;
        task
    }
}

/// Get kernel stack position for app
/// Kernel stacks are allocated below TRAP_CONTEXT
/// TRAP_CONTEXT is at TRAMPOLINE - PAGE_SIZE
/// So we allocate stacks starting from TRAP_CONTEXT - PAGE_SIZE
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    // Start from TRAP_CONTEXT - PAGE_SIZE and go down
    // Each stack needs KERNEL_STACK_SIZE + PAGE_SIZE (guard page)
    // Note: We start from TRAP_CONTEXT - PAGE_SIZE (not TRAP_CONTEXT) to leave space for trap context
    let top = TRAP_CONTEXT.wrapping_sub(PAGE_SIZE).wrapping_sub(app_id * (KERNEL_STACK_SIZE + PAGE_SIZE));
    let bottom = top.wrapping_sub(KERNEL_STACK_SIZE);
    
    // Debug: print kernel stack position
    crate::sbi::console_putstr("[Kernel Stack] app_id=");
    crate::trap::print_hex_usize(app_id);
    crate::sbi::console_putstr(", bottom=0x");
    crate::trap::print_hex_usize(bottom);
    crate::sbi::console_putstr(", top=0x");
    crate::trap::print_hex_usize(top);
    crate::sbi::console_putstr("\n");
    
    (bottom, top)
}

const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

lazy_static! {
    // KERNEL_SPACE is initialized and activated in mm::init()
    // We need to ensure it's properly set up when accessed
    // The actual kernel space is stored in mm::KERNEL_SPACE_INTERNAL
    // We'll create a new one here for compatibility, but it should match the activated one
    pub static ref KERNEL_SPACE: spin::Mutex<MemorySet> = {
        // When first accessed, create a kernel space
        // Note: This should match the one created in mm::init()
        // The actual activated kernel space is in mm module
        spin::Mutex::new(MemorySet::new_kernel())
    };
}

extern "C" {
    fn trap_handler();
}

use lazy_static::*;
