use crate::sbi;
use crate::task::exit_current_and_run_next;

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    // Should never reach here
    loop {}
}

pub fn sys_yield() -> isize {
    crate::task::switch_task();
    0
}

pub fn sys_get_time() -> isize {
    sbi::get_time() as isize
}
