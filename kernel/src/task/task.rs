//! Task Control Block
//!
//! Defines the structure and operations for tasks (processes)

use super::context::TaskContext;
use crate::config::memory_layout::{KERNEL_STACK_SIZE, PAGE_SIZE};
use crate::mm::memory_layout::PhysPageNum;
use crate::mm::MemorySet;
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

        // Safety check: ensure we're not accessing kernel code section
        extern "C" {
            fn stext();
            fn ekernel();
        }
        let stext_addr = stext as *const () as usize;
        let ekernel_addr = ekernel as *const () as usize;

        if kernel_va >= stext_addr && kernel_va < ekernel_addr {
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

        // Verify entry point page is mapped
        use crate::mm::memory_layout::VirtAddr;
        let entry_vpn = VirtAddr::new(entry_point).page_number();
        if let Some((_ppn, flags)) = memory_set.page_table().translate(entry_vpn) {
            if !flags.contains(crate::mm::page_table::PTEFlags::X)
                || !flags.contains(crate::mm::page_table::PTEFlags::U)
            {
                panic!("Entry page missing required permissions");
            }
        } else {
            panic!("Entry page not mapped!");
        }

        // Trap context is stored in user address space at TRAP_CONTEXT
        let trap_cx_pa = memory_set
            .translate(TRAP_CONTEXT)
            .expect("Failed to translate TRAP_CONTEXT address");

        let trap_cx_ppn = PhysPageNum::new(trap_cx_pa >> PAGE_SIZE.trailing_zeros() as usize);

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

        let tcb = Self {
            task_status,
            task_cx,
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
            entry_point,
            user_sp,
        };

        // Initialize trap context with user_satp
        let trap_cx = tcb.get_trap_cx();
        let user_token = tcb.get_user_token();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            0, // kernel_satp - not used
            kernel_stack_top,
            trap_handler as usize,
        );
        // Set user_satp to enable virtual memory for user program
        trap_cx.user_satp = user_token;
        // Set kernel_sp for next trap entry
        trap_cx.kernel_sp = kernel_stack_top;


        tcb
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
    let top = TRAP_CONTEXT
        .wrapping_sub(PAGE_SIZE)
        .wrapping_sub(app_id * (KERNEL_STACK_SIZE + PAGE_SIZE));
    let bottom = top.wrapping_sub(KERNEL_STACK_SIZE);

    (bottom, top)
}

const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

lazy_static! {
    // Legacy placeholder; kernel token comes from mm::get_kernel_token().
    pub static ref KERNEL_SPACE: spin::Mutex<MemorySet> = spin::Mutex::new(MemorySet::new_kernel());
}

extern "C" {
    fn trap_handler();
}

use lazy_static::*;
