//! Trap handling module

mod context;

pub use context::TrapContext;
use core::arch::global_asm;
use riscv::register::{scause, sepc, sscratch, sstatus, stval, stvec};

global_asm!(include_str!("trap.S"));

/// Initialize trap handling
pub fn init() {
    unsafe {
        // Set stvec to the TRAMPOLINE address of __alltraps
        stvec::write(crate::config::TRAMPOLINE, stvec::TrapMode::Direct);
    }
}

/// Trap handler called from assembly
#[no_mangle]
pub fn trap_handler() -> ! {
    trap_from_user()
}

/// Trap handler called from assembly
#[no_mangle]
pub fn trap_from_user() -> ! {
    let task = crate::task::current_task().expect("No current task");
    let mut inner = task.inner_exclusive_access();
    let trap_cx = inner.get_trap_cx();
    let scause = scause::read();
    let stval = stval::read();
    
    // Save sepc for later use in error messages
    let trap_sepc = trap_cx.sepc;
    
    // Drop inner before processing heavy logic that might switch tasks
    drop(inner);

    match scause.cause() {
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
             // System call
             // Re-acquire to get arguments
             let mut inner = task.inner_exclusive_access();
             let trap_cx = inner.get_trap_cx();
             let syscall_num = trap_cx.x[17];
             let args = [trap_cx.x[10], trap_cx.x[11], trap_cx.x[12]];
             trap_cx.sepc += 4;
             drop(inner);
             
             let result = crate::syscall::syscall(syscall_num, args) as usize;
             
             // Re-acquire to set return value
             // Note: current_task() might have changed if we yielded! 
             // But trap_from_user is running on the kernel stack of the *current* task.
             // If we switched tasks, we wouldn't be here until we switched back.
             let task = crate::task::current_task().expect("No current task");
             let mut inner = task.inner_exclusive_access();
             let trap_cx = inner.get_trap_cx();
             trap_cx.x[10] = result;
        }
        scause::Trap::Exception(scause::Exception::StoreFault)
        | scause::Trap::Exception(scause::Exception::StorePageFault)
        | scause::Trap::Exception(scause::Exception::LoadFault)
        | scause::Trap::Exception(scause::Exception::LoadPageFault) => {
            crate::println!(
                "[Kernel] Page fault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                stval,
                trap_sepc
            );
            crate::task::exit_current_and_run_next(-2);
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            crate::println!("[Kernel] IllegalInstruction in application, kernel killed it.");
            crate::task::exit_current_and_run_next(-3);
        }
        scause::Trap::Interrupt(scause::Interrupt::SupervisorTimer) => {
            set_next_trigger();
            crate::task::suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }

    trap_return();
}


/// Get current trap context
// fn current_trap_cx() -> &'static mut TrapContext {
//    crate::task::current_task()
//        .expect("No current task")
//        .get_trap_cx()
// }


/// Set next timer interrupt
fn set_next_trigger() {
    const TICKS_PER_SEC: usize = 100;
    crate::sbi::set_timer(
        (crate::sbi::get_time() as usize + crate::config::CLOCK_FREQ / TICKS_PER_SEC) as u64,
    );
}

/// Return to user space
#[no_mangle]
pub fn trap_return() -> ! {
    extern "C" {
        fn __alltraps();
        fn __restore();
    }

    let trap_cx_ptr = crate::config::TRAP_CONTEXT;
    let task = crate::task::current_task().expect("No current task");
    let inner = task.inner_exclusive_access();
    let user_satp = inner.get_user_token();
    drop(inner); // drop borrow before entering assembly loop

    let restore_va = crate::config::TRAMPOLINE + (__restore as usize - __alltraps as usize);

    unsafe {
        core::arch::asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}
