use crate::{println, sbi, task};

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}.", exit_code);
    task::exit_current_and_run_next(exit_code);
    panic!("Unreachable after exit_current_and_run_next!");
}

pub fn sys_yield() -> isize {
    task::suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    // TODO: Implement get_time
    sbi::get_time() as isize
}
