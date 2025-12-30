use crate::{println, sbi};

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}.", exit_code);
    // TODO: Call exit_current_and_run_next when task manager is ready
    sbi::shutdown()
}

pub fn sys_yield() -> isize {
    // TODO: Implement yield when scheduler is ready
    println!("[kernel] sys_yield called");
    0
}

pub fn sys_get_time() -> isize {
    // TODO: Implement get_time
    sbi::get_time() as isize
}
