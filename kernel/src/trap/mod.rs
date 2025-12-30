//! Interrupt and trap handling module

use crate::syscall::syscall;
use crate::{global_asm, println};
use context::TrapContext;
use riscv::register::mtvec::TrapMode;
use riscv::register::{scause, stval, stvec};

mod context;

global_asm!(include_str!("trap.S"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            cx.spec += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        scause::Trap::Exception(scause::Exception::StoreFault)
        | scause::Trap::Exception(scause::Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, killed.");
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction, killed.");
        }
        _ => {
            panic!("Unsupport trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
    cx
}
