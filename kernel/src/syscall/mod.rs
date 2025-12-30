//! System call handling module

<<<<<<< HEAD
pub fn syscall(_syscall_id: usize, _args: [usize; 3]) -> isize {
    // TODO: Implement syscall handling
    -1
=======
mod fs;
mod process;

use fs::*;
use process::*;

/// System call numbers
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;

/// System call dispatcher
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(),
        _ => {
            crate::println!("[kernel] Unsupported syscall_id: {}", syscall_id);
            -1
        }
    }
>>>>>>> c32c54c (feat: task subsystem.)
}
