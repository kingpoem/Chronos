use crate::println;

pub fn sys_exit(xstate: i32) -> ! {
    println!("[kernel] Application exited with code {}.", xstate);
    crate::sbi::shutdown()
}
