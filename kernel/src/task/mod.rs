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
    crate::sbi::console_putstr("[switch_task] Called\n");
    
    // Debug: print current state before switching
    unsafe {
        use core::arch::asm;
        let current_sp: usize;
        let current_satp: usize;
        let current_sscratch: usize;
        asm!("mv {}, sp", out(reg) current_sp);
        asm!("csrr {}, satp", out(reg) current_satp);
        asm!("csrr {}, sscratch", out(reg) current_sscratch);
        crate::sbi::console_putstr("[switch_task] Before switch: sp=0x");
        crate::trap::print_hex_usize(current_sp);
        crate::sbi::console_putstr(", satp=0x");
        crate::trap::print_hex_usize(current_satp);
        crate::sbi::console_putstr(", sscratch=0x");
        crate::trap::print_hex_usize(current_sscratch);
        crate::sbi::console_putstr("\n");
    }
    
    crate::sbi::console_putstr("[switch_task] Called\n");
    
    // Debug: print current state before switching
    unsafe {
        use core::arch::asm;
        let current_sp: usize;
        let current_satp: usize;
        let current_sscratch: usize;
        asm!("mv {}, sp", out(reg) current_sp);
        asm!("csrr {}, satp", out(reg) current_satp);
        asm!("csrr {}, sscratch", out(reg) current_sscratch);
        crate::sbi::console_putstr("[switch_task] Before switch: sp=0x");
        crate::trap::print_hex_usize(current_sp);
        crate::sbi::console_putstr(", satp=0x");
        crate::trap::print_hex_usize(current_satp);
        crate::sbi::console_putstr(", sscratch=0x");
        crate::trap::print_hex_usize(current_sscratch);
        crate::sbi::console_putstr("\n");
    }
    
    println!("[Task] switch_task called");
    crate::sbi::console_putstr("[switch_task] Locking TASK_MANAGER...\n");
    let mut task_manager = TASK_MANAGER.lock();
    crate::sbi::console_putstr("[switch_task] TASK_MANAGER locked\n");
    crate::sbi::console_putstr("[switch_task] Locking SCHEDULER...\n");
    let mut scheduler = SCHEDULER.lock();
    crate::sbi::console_putstr("[switch_task] SCHEDULER locked\n");
    
    crate::sbi::console_putstr("[switch_task] Getting current task...\n");
    let current_pid = task_manager.get_current_task();
    crate::sbi::console_putstr("[switch_task] Current task: ");
    if let Some(pid) = current_pid {
        crate::trap::print_hex_usize(pid);
    } else {
        crate::sbi::console_putstr("None");
    }
    crate::sbi::console_putstr("\n");
    println!("[Task] Current task: {:?}", current_pid);
    crate::sbi::console_putstr("[switch_task] Total tasks: ");
    crate::trap::print_hex_usize(task_manager.task_count());
    crate::sbi::console_putstr("\n");
    println!("[Task] Total tasks: {}", task_manager.task_count());
    
    crate::sbi::console_putstr("[switch_task] Processing current task...\n");
    if let Some(current) = current_pid {
        crate::sbi::console_putstr("[switch_task] Marking current task as ready: pid=");
        crate::trap::print_hex_usize(current);
        crate::sbi::console_putstr("\n");
        // Mark current task as ready
        if let Some(task) = task_manager.get_task_mut(current) {
            crate::sbi::console_putstr("[switch_task] Current task found, marking as ready\n");
            task.task_status = TaskStatus::Ready;
        } else {
            crate::sbi::console_putstr("[switch_task] WARNING: Current task not found!\n");
        }
    } else {
        crate::sbi::console_putstr("[switch_task] No current task\n");
    }
    
    // Find next task
    crate::sbi::console_putstr("[switch_task] Finding next task...\n");
    let next_pid = if let Some(current) = current_pid {
        crate::sbi::console_putstr("[switch_task] Scheduling from current task: pid=");
        crate::trap::print_hex_usize(current);
        crate::sbi::console_putstr("\n");
        scheduler.schedule_next(current, &*task_manager)
    } else {
        crate::sbi::console_putstr("[switch_task] Scheduling from start (no current task)\n");
        scheduler.schedule_next(0, &*task_manager)
    };
    
    crate::sbi::console_putstr("[switch_task] Next task: ");
    if let Some(pid) = next_pid {
        crate::trap::print_hex_usize(pid);
    } else {
        crate::sbi::console_putstr("None");
    }
    crate::sbi::console_putstr("\n");
    
    if let Some(next) = next_pid {
        crate::sbi::console_putstr("[switch_task] Switching to task ");
        crate::trap::print_hex_usize(next);
        crate::sbi::console_putstr("\n");
        println!("[Task] Switching to task {}", next);
        
        // Mark next task as running
        crate::sbi::console_putstr("[switch_task] Marking next task as running...\n");
        if let Some(task) = task_manager.get_task_mut(next) {
            crate::sbi::console_putstr("[switch_task] Next task found, marking as running\n");
            task.task_status = TaskStatus::Running;
        } else {
            crate::sbi::console_putstr("[switch_task] ERROR: Next task not found!\n");
            panic!("Next task not found!");
        }
        
        // Don't switch address space here - it will be done in __restore
        // We need to stay in kernel space to access kernel stack during __switch
        crate::sbi::console_putstr("[switch_task] Setting current task...\n");
        task_manager.set_current_task(Some(next));
        crate::sbi::console_putstr("[switch_task] Current task set\n");
        
        crate::sbi::console_putstr("[switch_task] Getting task contexts...\n");
        let current_cx = if let Some(current) = current_pid {
            crate::sbi::console_putstr("[switch_task] Getting current task context: pid=");
            crate::trap::print_hex_usize(current);
            crate::sbi::console_putstr("\n");
            task_manager.get_task(current).map(|t| {
                crate::sbi::console_putstr("[switch_task] Current task found, getting task_cx\n");
                &t.task_cx as *const TaskContext
            })
        } else {
            crate::sbi::console_putstr("[switch_task] No current task, current_cx=None\n");
            None
        };
        
        crate::sbi::console_putstr("[switch_task] Getting next task context: pid=");
        crate::trap::print_hex_usize(next);
        crate::sbi::console_putstr("\n");
        let next_cx = task_manager.get_task(next).map(|t| {
            crate::sbi::console_putstr("[switch_task] Next task found, getting task_cx\n");
            &t.task_cx as *const TaskContext
        });
        
        crate::sbi::console_putstr("[switch_task] Task contexts obtained\n");
        
        if let (Some(curr_ptr), Some(next_ptr)) = (current_cx, next_cx) {
            // For task switch, we need to:
            // 1. Save current task's trap context (if it was running)
            // 2. Prepare next task's trap context on its kernel stack
            // 3. Switch context and jump to __restore
            
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
            
            // Ensure we're in kernel address space BEFORE accessing trap context
            // get_trap_cx() uses physical address as kernel virtual address (identity mapping)
            // So we must be in kernel address space when calling it
            let kernel_token = crate::task::task::KERNEL_SPACE.lock().token();
            
            // Switch to kernel address space before accessing trap context
            unsafe {
                use core::arch::asm;
                asm!("csrw satp, {}", in(reg) kernel_token);
                asm!("sfence.vma");
            }
            
            let next_trap_cx_data = {
                let trap_cx = next_task.get_trap_cx();
                crate::trap::TrapContext {
                    x: trap_cx.x,
                    sstatus: trap_cx.sstatus,
                    sepc: trap_cx.sepc,
                    user_satp: trap_cx.user_satp,
                }
            };
            
            drop(task_manager);
            drop(scheduler);
            
            unsafe {
                // Print register state before switch
                crate::trap::print_critical_registers("[Before __switch]");
                
                // Note: We already switched to kernel address space above
                // No need to switch again here
                
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
            
            // Debug: print kernel stack address
            crate::sbi::console_putstr("[Task] First task kernel stack: 0x");
            crate::trap::print_hex_usize(kernel_sp);
            crate::sbi::console_putstr("\n");
            
            // Get task information before dropping task_manager
            let kernel_token = crate::task::task::KERNEL_SPACE.lock().token();
            let (entry_point, user_sp, user_token, kernel_stack_top) = {
                let task = task_manager.get_task(next).unwrap();
                let entry_point = task.entry_point;
                let user_sp = task.user_sp;
                let user_token = task.get_user_token();
                let kernel_stack_top = task.task_cx.sp;
                (entry_point, user_sp, user_token, kernel_stack_top)
            };
            
            // Create trap context data
            extern "C" {
                fn trap_handler();
            }
            
            crate::sbi::console_putstr("[Task] Creating trap context: entry=0x");
            crate::trap::print_hex_usize(entry_point);
            crate::sbi::console_putstr(", user_sp=0x");
            crate::trap::print_hex_usize(user_sp);
            crate::sbi::console_putstr(", kernel_sp=0x");
            crate::trap::print_hex_usize(kernel_stack_top);
            crate::sbi::console_putstr("\n");
            
            // CRITICAL: Ensure interrupts are disabled and timer interrupt is cleared before calling app_init_context
            // app_init_context will temporarily modify sstatus (enable interrupts), so we must ensure:
            // 1. Interrupts are disabled (sstatus::SIE = 0)
            // 2. Timer interrupt is disabled in sie (sie::STIMER = 0) - temporarily disable it
            // 3. This ensures that even if interrupts are enabled during app_init_context, timer won't trigger
            unsafe {
                use riscv::register::{sstatus, sie};
                sstatus::clear_sie(); // Disable interrupts
                sie::clear_stimer();  // Temporarily disable timer interrupt in sie
                // We will re-enable it after app_init_context returns and before setting the timer
            }
            
            let mut trap_cx_data = crate::trap::TrapContext::app_init_context(
                entry_point,
                user_sp,
                kernel_token,
                kernel_stack_top,
                trap_handler as *const () as usize,
            );
            trap_cx_data.user_satp = user_token;
            
            // CRITICAL: Immediately disable interrupts after app_init_context returns
            // app_init_context modifies sstatus to get the correct value, which may enable interrupts
            // We must disable interrupts again to ensure they remain disabled until we switch to user mode
            unsafe {
                use riscv::register::sstatus;
                sstatus::clear_sie(); // Disable interrupts again
            }
            
            // Debug: Print trap context details
            crate::sbi::console_putstr("[Task] Trap context created:\n");
            crate::sbi::console_putstr("  entry_point=0x");
            crate::trap::print_hex_usize(entry_point);
            crate::sbi::console_putstr("\n  user_sp=0x");
            crate::trap::print_hex_usize(user_sp);
            crate::sbi::console_putstr("\n  user_satp=0x");
            crate::trap::print_hex_usize(user_token);
            crate::sbi::console_putstr("\n  sepc=0x");
            crate::trap::print_hex_usize(trap_cx_data.sepc);
            
            // Check sstatus SPP and SIE bits
            use riscv::register::sstatus;
            let sstatus_val = trap_cx_data.sstatus;
            
            // Get sstatus bits by temporarily writing to register
            unsafe {
                let saved_bits: usize;
                core::arch::asm!("csrr {}, sstatus", out(reg) saved_bits);
                
                // Construct sstatus bits from the Sstatus value
                // We need to write it to register to read it back
                // Let's use set_spp and set_sie to construct the value
                if sstatus_val.spp() == sstatus::SPP::User {
                    sstatus::set_spp(sstatus::SPP::User);
                } else {
                    sstatus::set_spp(sstatus::SPP::Supervisor);
                }
                if sstatus_val.sie() {
                    sstatus::set_sie();
                } else {
                    sstatus::clear_sie();
                }
                let sstatus_bits: usize;
                core::arch::asm!("csrr {}, sstatus", out(reg) sstatus_bits);
                
                // Restore original
                core::arch::asm!("csrw sstatus, {}", in(reg) saved_bits);
                
                crate::sbi::console_putstr("\n  sstatus bits=0x");
                crate::trap::print_hex_usize(sstatus_bits);
            }
            
            // Check SPP and SIE using the methods
            crate::sbi::console_putstr("\n  sstatus.SPP=");
            if sstatus_val.spp() == sstatus::SPP::User {
                crate::sbi::console_putstr("User");
            } else {
                crate::sbi::console_putstr("Supervisor");
            }
            crate::sbi::console_putstr("\n  sstatus.SIE=");
            if sstatus_val.sie() {
                crate::sbi::console_putstr("enabled");
            } else {
                crate::sbi::console_putstr("disabled");
            }
            crate::sbi::console_putstr("\n");
            
            // Ensure we're in kernel address space
            let kernel_token = crate::task::task::KERNEL_SPACE.lock().token();
            
            // Debug: Print the first instruction that will be executed (before dropping task_manager)
            // Read instruction through the task's memory set (translates user VA to PA)
            let task_ref = task_manager.get_task(next).unwrap();
            if let Some(pa) = task_ref.memory_set.translate(entry_point) {
                unsafe {
                    let instr_ptr = pa as *const u32;
                    let first_instr = *instr_ptr;
                    let first_instr_16bit = first_instr & 0xFFFF;  // Extract lower 16 bits
                    crate::sbi::console_putstr("[switch_task] First instruction at 0x");
                    crate::trap::print_hex_usize(entry_point);
                    crate::sbi::console_putstr(" (PA=0x");
                    crate::trap::print_hex_usize(pa);
                    crate::sbi::console_putstr("): 32-bit=0x");
                    crate::trap::print_hex_usize(first_instr as usize);
                    crate::sbi::console_putstr(", 16-bit=0x");
                    crate::trap::print_hex_usize(first_instr_16bit as usize);
                    crate::sbi::console_putstr(" (expected: 0x1141)\n");
                    if first_instr_16bit != 0x1141 {
                        crate::sbi::console_putstr("[switch_task] ERROR: Instruction mismatch!\n");
                    }
                }
            } else {
                crate::sbi::console_putstr("[switch_task] WARNING: Cannot translate entry point 0x");
                crate::trap::print_hex_usize(entry_point);
                crate::sbi::console_putstr("\n");
            }
            
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
                    
                    // Save sepc before moving trap_cx_data
                    let saved_sepc = trap_cx_data.sepc;
                
                // Write trap context to kernel stack
                *trap_cx_kernel = trap_cx_data;
                // Ensure user_satp is set
                (*trap_cx_kernel).user_satp = user_token;
                
                // Debug: print user_satp value
                crate::sbi::console_putstr("[switch_task] First task user_satp=0x");
                crate::trap::print_hex_usize(user_token);
                crate::sbi::console_putstr("\n");
                
                if user_token == 0 {
                    crate::sbi::console_putstr("[switch_task] ERROR: user_satp is 0! Cannot switch to user mode!\n");
                    crate::sbi::shutdown();
                }
                    
                    // Debug: Print sepc and user_satp values that will be restored
                    crate::sbi::console_putstr("[switch_task] Trap context sepc=0x");
                    crate::trap::print_hex_usize(saved_sepc);
                    crate::sbi::console_putstr(", user_satp=0x");
                    crate::trap::print_hex_usize(user_token);
                    crate::sbi::console_putstr("\n");
                
                // CRITICAL: Set the first timer right before switching to user mode
                // This ensures we have enough time to complete initialization and switch to user mode
                // before the first timer interrupt triggers
                // According to rCore: set timer right before jumping to __restore
                crate::trap::set_next_timer();
                crate::sbi::console_putstr("[switch_task] First timer set, ready to switch to user mode\n");
                
                // Now jump to __restore with trap context address in a0
                // __restore will restore sstatus which contains the correct interrupt state
                // No need to disable interrupts here - __restore handles the state transition
                extern "C" {
                    fn __restore();
                }
                crate::sbi::console_putstr("[switch_task] Jumping to __restore with trap_cx=0x");
                crate::trap::print_hex_usize(trap_cx_kernel as usize);
                crate::sbi::console_putstr("\n");
                asm!(
                    "mv a0, {}",
                    "jal {}",
                    in(reg) trap_cx_kernel as usize,
                    sym __restore,
                );
                // Should never reach here
                crate::sbi::console_putstr("[switch_task] ERROR: Returned from __restore!\n");
                crate::sbi::shutdown();
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
        
        // If no more tasks, shutdown (disable timer interrupt first)
        if task_manager.task_count() == 0 {
            drop(task_manager);
            println!("[Task] No more tasks, disabling timer interrupt and shutting down...");
            
            // Disable timer interrupt before shutdown
            unsafe {
                use riscv::register::{sie, sstatus};
                sstatus::clear_sie();  // Disable interrupts
                sie::clear_stimer();   // Disable timer interrupt
            }
            
            crate::sbi::shutdown();  // shutdown() returns !, so code after this is unreachable
        }
    }
    
    drop(task_manager);
    switch_task();
}
