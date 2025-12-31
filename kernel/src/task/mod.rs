//! Task (process) management module

mod context;
mod task;
mod manager;
mod scheduler;
mod loader;

pub use context::TaskContext;
pub use task::TaskStatus;
pub use manager::TaskManager;
pub use scheduler::Scheduler;
pub use loader::load_apps;

use crate::global_asm;
use lazy_static::*;
use spin::Mutex;

global_asm!(include_str!("switch.S"));

extern "C" {
    fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}

lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
    pub static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

pub fn init() {
    println!("[Task] Starting task management initialization...");
    // Kernel address space should already be activated in mm::init()
    // Verify that we can access it
    println!("[Task] Getting kernel token...");
    let kernel_token = crate::mm::get_kernel_token();
    println!("[Task] Kernel token obtained: {:#x}", kernel_token);
    
    // Update KERNEL_SPACE to match the activated one
    // Instead of recreating the entire memory set (which requires heap allocation),
    // we just need to ensure the token is correct
    // The actual kernel space is already created and activated in mm::init()
    println!("[Task] Locking KERNEL_SPACE...");
    let mut ks = task::KERNEL_SPACE.lock();
    println!("[Task] Creating new kernel memory set (this may allocate memory)...");
    // Recreate kernel space to match the activated one
    // Note: This will allocate Vec and BTreeMap, but heap allocator is already initialized
    *ks = crate::mm::MemorySet::new_kernel();
    println!("[Task] Kernel memory set created successfully");
    // Verify token matches (but don't panic if it doesn't - just warn)
    let ks_token = ks.token();
    println!("[Task] Verifying token match...");
    if ks_token != kernel_token {
        println!("[Task] WARNING: Kernel space token mismatch: expected {:#x}, got {:#x}", kernel_token, ks_token);
    } else {
        println!("[Task] Token match verified");
    }
    
    println!("[Task] Task management initialized");
    println!("[Task] Kernel address space token: {:#x}", kernel_token);
}

/// Switch to next task
pub fn switch_task() {
    println!("[Task] switch_task called");
    let mut task_manager = TASK_MANAGER.lock();
    let mut scheduler = SCHEDULER.lock();
    
    let current_pid = task_manager.get_current_task();
    println!("[Task] Current task: {:?}", current_pid);
    println!("[Task] Total tasks: {}", task_manager.task_count());
    
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
        println!("[Task] Switching to task {}", next);
        // Mark next task as running
        if let Some(task) = task_manager.get_task_mut(next) {
            task.task_status = TaskStatus::Running;
        }
        
        // Don't switch address space here - it will be done in __restore
        // We need to stay in kernel space to access kernel stack during __switch
        task_manager.set_current_task(Some(next));
        
        let current_cx = if let Some(current) = current_pid {
            task_manager.get_task(current).map(|t| &t.task_cx as *const TaskContext)
        } else {
            None
        };
        
        let next_cx = task_manager.get_task(next).map(|t| &t.task_cx as *const TaskContext);
        
        if let (Some(curr_ptr), Some(next_ptr)) = (current_cx, next_cx) {
            // For task switch, we need to:
            // 1. Save current task's trap context (if it was running)
            // 2. Prepare next task's trap context on its kernel stack
            // 3. Switch context and jump to __restore
            
            let current = current_pid.unwrap();
            let current_task = task_manager.get_task(current).unwrap();
            let next_task = task_manager.get_task(next).unwrap();
            
            // Save current task's trap context if it was running
            // The trap context is already on the kernel stack from __alltraps
            // trap_handler has already updated it in user space
            
            // Prepare next task's trap context on its kernel stack
            let next_kernel_sp = next_task.task_cx.sp;
            let next_user_token = next_task.get_user_token();
            
            // Debug: print kernel stack address
            crate::sbi::console_putstr("[Task] Next kernel stack: 0x");
            crate::trap::print_hex_usize(next_kernel_sp);
            crate::sbi::console_putstr("\n");
            
            let next_trap_cx_data = {
                let trap_cx = next_task.get_trap_cx();
                crate::trap::TrapContext {
                    x: trap_cx.x,
                    sstatus: trap_cx.sstatus,
                    sepc: trap_cx.sepc,
                    user_satp: trap_cx.user_satp,
                }
            };
            
            // Ensure we're in kernel address space
            let kernel_token = crate::task::task::KERNEL_SPACE.lock().token();
            drop(task_manager);
            drop(scheduler);
            
                    unsafe {
                        use core::arch::asm;
                        // Print register state before switch
                        crate::trap::print_critical_registers("[Before __switch]");
                        
                        // Switch to kernel address space
                        asm!("csrw satp, {}", in(reg) kernel_token);
                        asm!("sfence.vma");
                        
                        // Print register state after address space switch
                        crate::sbi::console_putstr("[After satp switch] satp=0x");
                        crate::trap::print_hex_usize(kernel_token);
                        crate::sbi::console_putstr("\n");
                        
                        // Prepare trap context on next task's kernel stack
                        let next_trap_cx_kernel = (next_kernel_sp - 34 * 8) as *mut crate::trap::TrapContext;
                        
                        // Debug: print trap context address
                        crate::sbi::console_putstr("[Task] Next kernel stack: 0x");
                        crate::trap::print_hex_usize(next_kernel_sp);
                        crate::sbi::console_putstr(", Trap context: 0x");
                        crate::trap::print_hex_usize(next_trap_cx_kernel as usize);
                        crate::sbi::console_putstr("\n");
                        
                        // Check if the address is mapped
                        let kernel_space = crate::mm::KERNEL_SPACE_INTERNAL.lock();
                        if let Some(ref ks) = *kernel_space {
                            if let Some(pa) = ks.translate(next_trap_cx_kernel as usize) {
                                crate::sbi::console_putstr("[Task] Trap context mapped: VA 0x");
                                crate::trap::print_hex_usize(next_trap_cx_kernel as usize);
                                crate::sbi::console_putstr(" -> PA 0x");
                                crate::trap::print_hex_usize(pa);
                                crate::sbi::console_putstr("\n");
                            } else {
                                crate::sbi::console_putstr("[Task] ERROR: Trap context NOT MAPPED!\n");
                                crate::sbi::console_putstr("[Task] Kernel stack region check:\n");
                                // Check kernel stack region bounds
                                use crate::config::memory_layout::{KERNEL_STACK_SIZE, PAGE_SIZE};
                                const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
                                const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;
                                const MAX_KERNEL_STACKS: usize = 16;
                                let kernel_stack_region_size = MAX_KERNEL_STACKS * (KERNEL_STACK_SIZE + PAGE_SIZE);
                                let kernel_stack_region_start = TRAP_CONTEXT.wrapping_sub(kernel_stack_region_size);
                                let kernel_stack_region_start_aligned = kernel_stack_region_start & !(PAGE_SIZE - 1);
                                crate::sbi::console_putstr("[Task] Expected region: 0x");
                                crate::trap::print_hex_usize(kernel_stack_region_start_aligned);
                                crate::sbi::console_putstr(" - 0x");
                                crate::trap::print_hex_usize(TRAMPOLINE.wrapping_add(PAGE_SIZE));
                                crate::sbi::console_putstr("\n");
                                crate::sbi::shutdown();
                            }
                        }
                        drop(kernel_space);
                        
                        *next_trap_cx_kernel = next_trap_cx_data;
                        (*next_trap_cx_kernel).user_satp = next_user_token;
                
                // Call __switch with current and next task context pointers
                // __switch will:
                // 1. Save current task context
                // 2. Restore next task context (including sp)
                // 3. Set a0 to trap context address (sp - 34*8)
                // 4. Return to __restore (via ra)
                __switch(
                    curr_ptr as *mut TaskContext,  // Current task context pointer
                    next_ptr,  // Next task context pointer
                );
                // After __switch, we're in __restore
                // __restore will restore registers and switch to user space
            }
        } else {
            // No current task, need to set up first task
            // For first task, we need to prepare trap context on kernel stack
            // and then jump to __restore
            let kernel_sp = task_manager.get_task(next).unwrap().task_cx.sp;
            let user_token = task_manager.get_task(next).unwrap().get_user_token();
            
            // Debug: print kernel stack address
            crate::sbi::console_putstr("[Task] First task kernel stack: 0x");
            crate::trap::print_hex_usize(kernel_sp);
            crate::sbi::console_putstr("\n");
            
            // Get trap context data before dropping task_manager
            let trap_cx_data = {
                let task = task_manager.get_task(next).unwrap();
                let trap_cx = task.get_trap_cx();
                crate::trap::TrapContext {
                    x: trap_cx.x,
                    sstatus: trap_cx.sstatus,
                    sepc: trap_cx.sepc,
                    user_satp: trap_cx.user_satp,
                }
            };
            
            // Ensure we're in kernel address space
            let kernel_token = crate::task::task::KERNEL_SPACE.lock().token();
            drop(task_manager);
            drop(scheduler);
            
            unsafe {
                use core::arch::asm;
                // Print register state before first task
                crate::trap::print_critical_registers("[Before first task]");
                
                // Switch to kernel address space
                asm!("csrw satp, {}", in(reg) kernel_token);
                asm!("sfence.vma");
                
                // Prepare trap context on kernel stack
                // Allocate space for TrapContext (34 * 8 bytes)
                let trap_cx_kernel = (kernel_sp - 34 * 8) as *mut crate::trap::TrapContext;
                
                // Debug: print trap context address
                crate::sbi::console_putstr("[Task] First task trap context: 0x");
                crate::trap::print_hex_usize(trap_cx_kernel as usize);
                crate::sbi::console_putstr("\n");
                
                // Check if the address is mapped
                let kernel_space = crate::mm::KERNEL_SPACE_INTERNAL.lock();
                if let Some(ref ks) = *kernel_space {
                    if let Some(pa) = ks.translate(trap_cx_kernel as usize) {
                        crate::sbi::console_putstr("[Task] Trap context mapped: VA 0x");
                        crate::trap::print_hex_usize(trap_cx_kernel as usize);
                        crate::sbi::console_putstr(" -> PA 0x");
                        crate::trap::print_hex_usize(pa);
                        crate::sbi::console_putstr("\n");
                    } else {
                        crate::sbi::console_putstr("[Task] ERROR: Trap context NOT MAPPED!\n");
                        crate::sbi::console_putstr("[Task] Kernel stack: 0x");
                        crate::trap::print_hex_usize(kernel_sp);
                        crate::sbi::console_putstr(", Trap context: 0x");
                        crate::trap::print_hex_usize(trap_cx_kernel as usize);
                        crate::sbi::console_putstr("\n");
                        crate::sbi::shutdown();
                    }
                }
                drop(kernel_space);
                
                // Write trap context to kernel stack
                *trap_cx_kernel = trap_cx_data;
                // Ensure user_satp is set
                (*trap_cx_kernel).user_satp = user_token;
                
                // Now jump to __restore with trap context address in a0
                extern "C" {
                    fn __restore();
                }
                asm!(
                    "mv a0, {}",
                    "jal {}",
                    in(reg) trap_cx_kernel as usize,
                    sym __restore,
                );
            }
        }
    } else {
        // No tasks to run, switch back to kernel space
        println!("[Task] No tasks to run, shutting down");
        let kernel_token = crate::task::task::KERNEL_SPACE.lock().token();
        unsafe {
            use core::arch::asm;
            asm!("csrw satp, {}", in(reg) kernel_token);
            asm!("sfence.vma");
        }
    }
}

/// Exit current task and run next
pub fn exit_current_and_run_next(exit_code: i32) {
    let mut task_manager = TASK_MANAGER.lock();
    let current_pid = task_manager.get_current_task();
    
    if let Some(pid) = current_pid {
        println!("[Task] Task {} exited with code {}", pid, exit_code);
        task_manager.mark_zombie(pid);
        task_manager.remove_task(pid);
        task_manager.set_current_task(None);
        
        // If no more tasks, shutdown
        if task_manager.task_count() == 0 {
            drop(task_manager);
            println!("[Task] No more tasks, shutting down...");
            crate::sbi::shutdown();
        }
    }
    
    drop(task_manager);
    switch_task();
}
