//! Interrupt and trap handling module

use crate::syscall::syscall;
use crate::global_asm;
use riscv::register::{scause, sie, sstatus, stval};

pub mod context;
pub use context::TrapContext;

global_asm!(include_str!("trap.S"));

pub fn init() {
    extern "C" {
        fn __alltraps();
        fn __restore();
        fn allocate_trap_context();
        static mut KERNEL_SATP: usize;
        static mut TRAP_HANDLER_KERNEL_ADDR: usize;
        static mut RESTORE_TRAMPOLINE_ADDR: usize;
    }

    // CRITICAL: Set KERNEL_SATP so that __alltraps can switch back to kernel address space
    unsafe {
        let kernel_satp: usize;
        core::arch::asm!("csrr {}, satp", out(reg) kernel_satp, options(nomem, nostack));
        KERNEL_SATP = kernel_satp;

        // CRITICAL: Set TRAP_HANDLER_KERNEL_ADDR so trampoline can jump back to kernel
        TRAP_HANDLER_KERNEL_ADDR = allocate_trap_context as usize;

        // CRITICAL: Set RESTORE_TRAMPOLINE_ADDR so trap_handler can return to __restore in trampoline
        extern "C" {
            fn strampoline();
        }
        let restore_offset = __restore as usize - strampoline as usize;
        let restore_trampoline = crate::config::TRAMPOLINE + restore_offset;
        RESTORE_TRAMPOLINE_ADDR = restore_trampoline;
    }

    // CRITICAL: Set sscratch to current kernel stack pointer
    unsafe {
        let current_sp: usize;
        core::arch::asm!("mv {}, sp", out(reg) current_sp);
        core::arch::asm!("csrw sscratch, {}", in(reg) current_sp);
    }

    unsafe {
        extern "C" {
            fn strampoline();
        }
        let trap_offset = __alltraps as usize - strampoline as usize;
        let trap_vec = crate::config::TRAMPOLINE + trap_offset;

        // Set stvec to trampoline address
        core::arch::asm!("csrw stvec, {}", in(reg) trap_vec, options(nomem, nostack));
    }
}

/// Get the trampoline address of __restore
/// This is used by task switching code to jump to __restore in trampoline space
#[inline(never)]
pub fn get_restore_trampoline_addr() -> usize {
    extern "C" {
        fn __restore();
        fn strampoline();
    }
    let restore_addr = __restore as usize;
    let strampoline_addr = strampoline as usize;
    let restore_offset = restore_addr - strampoline_addr;
    crate::config::TRAMPOLINE + restore_offset
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    
    // Determine if we're in kernel mode or user mode
    // Check sstatus.SPP bit: 0 = User, 1 = Supervisor
    use riscv::register::sstatus;
    let is_user_mode = cx.sstatus.spp() == sstatus::SPP::User;

    
    // Store user token in trap context for __restore to use
    // Only do this if we have a current task and we're in user mode
    if is_user_mode {
        let task_manager = crate::task::TASK_MANAGER.lock();
        if let Some(current_pid) = task_manager.get_current_task() {
            if let Some(task) = task_manager.get_task(current_pid) {
                cx.user_satp = task.get_user_token();
            }
        }
        drop(task_manager);
    } else {
        // Kernel mode interrupt - set user_satp to 0 to indicate kernel mode
        cx.user_satp = 0;
    }
    
    match scause.cause() {
        scause::Trap::Interrupt(scause::Interrupt::SupervisorTimer) => {
            // Timer interrupt - trigger preemptive scheduling
            // CRITICAL: Only trigger task switching from user mode interrupts
            // Kernel mode interrupts should NOT trigger task switching
            // This is the rCore way: kernel mode interrupts are handled synchronously
            // and should not cause context switches
            
            set_next_timer();
            
            if is_user_mode {
                // User mode interrupt: can trigger preemptive scheduling
                let task_manager = crate::task::TASK_MANAGER.lock();
                let task_count = task_manager.task_count();
                drop(task_manager);
                
                if task_count > 0 {
                    let mut scheduler = crate::task::SCHEDULER.lock();
                    if scheduler.tick() {
                        // Time slice expired, switch task
                        drop(scheduler);
                        crate::task::switch_task();
                    } else {
                        drop(scheduler);
                    }
                }
            } else {
                // Kernel mode interrupt: do NOT trigger task switching
                // Just set the next timer and return
                // This prevents issues when kernel is executing and gets interrupted
            }
        }
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            cx.sepc += 4;
            // System call arguments: a0-a5 (x[10]-x[15]), syscall number in a7 (x[17])
            cx.x[10] = syscall(
                cx.x[17],
                [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]]
            ) as usize;
        }
        scause::Trap::Exception(scause::Exception::StoreFault)
        | scause::Trap::Exception(scause::Exception::StorePageFault) => {
            if is_user_mode {
                println!("[Trap] Store fault in user: stval=0x{:x}, sepc=0x{:x}", stval, cx.sepc);
                crate::task::exit_current_and_run_next(-1);
            } else {
                println!("[Trap] Store fault in kernel: stval=0x{:x}, sepc=0x{:x}", stval, cx.sepc);
                crate::sbi::shutdown();
            }
        }
        scause::Trap::Exception(scause::Exception::LoadFault)
        | scause::Trap::Exception(scause::Exception::LoadPageFault) => {
            if is_user_mode {
                println!("[Trap] Load fault in user: stval=0x{:x}, sepc=0x{:x}", stval, cx.sepc);
                crate::task::exit_current_and_run_next(-1);
            } else {
                println!("[Trap] Load fault in kernel: stval=0x{:x}, sepc=0x{:x}", stval, cx.sepc);
                crate::sbi::shutdown();
            }
        }
        scause::Trap::Exception(scause::Exception::InstructionFault)
        | scause::Trap::Exception(scause::Exception::InstructionPageFault) => {
            if is_user_mode {
                println!("[Trap] Instruction fault in user: stval=0x{:x}, sepc=0x{:x}", stval, cx.sepc);
                crate::task::exit_current_and_run_next(-1);
            } else {
                println!("[Trap] Instruction fault in kernel: stval=0x{:x}, sepc=0x{:x}", stval, cx.sepc);
                crate::sbi::shutdown();
            }
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            if is_user_mode {
                println!("[Trap] Illegal instruction in user: sepc=0x{:x}", cx.sepc);
                crate::task::exit_current_and_run_next(-1);
            } else {
                println!("[Trap] Illegal instruction in kernel: sepc=0x{:x}", cx.sepc);
                crate::sbi::shutdown();
            }
        }
        _ => {
            if is_user_mode {
                println!("[Trap] Unsupported trap in user: scause=0x{:x}", scause.bits());
                crate::task::exit_current_and_run_next(-1);
            } else {
                println!("[Trap] Unsupported trap in kernel: scause=0x{:x}", scause.bits());
                crate::sbi::shutdown();
            }
        }
    }
    cx
}

/// Enable timer interrupt (should be called after tasks are loaded)
/// NOTE: This function only enables timer interrupt in sie, but does NOT set the timer
/// The timer should be set in switch_task() right before switching to user mode
/// This prevents timer interrupts from triggering during kernel initialization
pub fn enable_timer_interrupt() {
    // Disable interrupts while setting up timer
    unsafe {
        sstatus::clear_sie();
    }

    unsafe {
        // Enable timer interrupt in sie (but interrupts are still disabled via sstatus::SIE)
        sie::set_stimer();
    }

    // DO NOT enable sstatus::SIE here - it will be enabled when we restore sstatus in __restore
    // The trap context's sstatus should have SIE bit set, so when we switch to user mode,
    // interrupts will be automatically enabled
}

/// Set next timer interrupt (10ms interval for more responsive scheduling)
/// This function should be called right before switching to user mode for the first time,
/// and also in trap_handler after handling timer interrupts
pub fn set_next_timer() {
    use crate::config::CLOCK_FREQ;
    use crate::sbi;

    let time = sbi::get_time();

    // Set timer to 10ms intervals (CLOCK_FREQ / 100 = 10ms)
    // This gives us 100 ticks per second, and with time_slice=10, each task gets 100ms
    let next = time + (CLOCK_FREQ / 100) as u64; // 10ms
    sbi::set_timer(next);
}

/// Print a usize as hexadecimal (helper for low-level debug)
pub fn print_hex_usize(n: usize) {
    let hex_digits = b"0123456789abcdef";
    let mut buffer = [0u8; 16];
    let mut num = n;
    let mut i = 0;

    if num == 0 {
        crate::sbi::console_putchar(b'0');
        return;
    }

    while num > 0 && i < 16 {
        buffer[i] = hex_digits[(num & 0xF) as usize];
        num >>= 4;
        i += 1;
    }

    for j in (0..i).rev() {
        crate::sbi::console_putchar(buffer[j]);
    }
}
