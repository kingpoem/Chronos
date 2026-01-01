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
    // Store entry point and user stack for trap context initialization
    pub entry_point: usize,
    pub user_sp: usize,
}

impl TaskControlBlock {
    /// Get trap context
    /// 
    /// # Safety
    /// This function assumes:
    /// 1. We're in kernel address space (satp points to kernel page table)
    /// 2. Kernel uses identity mapping (PA == VA)
    /// 3. The physical page is mapped in kernel address space
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        // Convert physical page number to kernel virtual address (identity mapping)
        // Since kernel uses identity mapping, physical address = virtual address
        let kernel_va = self.trap_cx_ppn.addr().0;
        
        // Debug: print trap context address
        crate::sbi::console_putstr("[get_trap_cx] PPN=0x");
        crate::trap::print_hex_usize(self.trap_cx_ppn.0);
        crate::sbi::console_putstr(" -> kernel_va=0x");
        crate::trap::print_hex_usize(kernel_va);
        crate::sbi::console_putstr("\n");
        
        // Safety check: ensure we're not accessing kernel code section
        extern "C" {
            fn stext();
            fn ekernel();
        }
        let stext_addr = stext as *const () as usize;
        let ekernel_addr = ekernel as *const () as usize;
        
        if kernel_va >= stext_addr && kernel_va < ekernel_addr {
            crate::sbi::console_putstr("[get_trap_cx] ERROR: Attempting to access kernel code section!\n");
            crate::sbi::console_putstr("[get_trap_cx] ERROR: PPN=0x");
            crate::trap::print_hex_usize(self.trap_cx_ppn.0);
            crate::sbi::console_putstr(", kernel_va=0x");
            crate::trap::print_hex_usize(kernel_va);
            crate::sbi::console_putstr(", stext=0x");
            crate::trap::print_hex_usize(stext_addr);
            crate::sbi::console_putstr("\n");
            panic!("get_trap_cx: Attempting to access kernel code section! This indicates a bug in trap_cx_ppn calculation.");
        }
        
        unsafe { &mut *(kernel_va as *mut TrapContext) }
    }
    
    /// Get user token (satp value)
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    
    /// Create a new task from ELF data
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        
        // Trap context is stored in user address space at TRAP_CONTEXT
        let trap_cx_pa = memory_set
            .translate(TRAP_CONTEXT)
            .expect("Failed to translate TRAP_CONTEXT address");
        
        let trap_cx_ppn = PhysPageNum::new(
            trap_cx_pa >> PAGE_SIZE.trailing_zeros() as usize
        );
        
        // Safety check: ensure trap_cx_ppn doesn't point to kernel code section
        extern "C" {
            fn stext();
            fn ekernel();
        }
        let stext_addr = stext as *const () as usize;
        let ekernel_addr = ekernel as *const () as usize;
        let kernel_va = trap_cx_ppn.addr().0;
        
        if kernel_va >= stext_addr && kernel_va < ekernel_addr {
            panic!("trap_cx_ppn calculation error: points to kernel code section! PPN=0x{:x}, kernel_va=0x{:x}", trap_cx_ppn.0, kernel_va);
        }
        
        let task_status = TaskStatus::Ready;
        let (_kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        let task_cx = TaskContext::goto_trap_return(kernel_stack_top);
        
        Self {
            task_status,
            task_cx,
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
            entry_point,
            user_sp,
        }
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
