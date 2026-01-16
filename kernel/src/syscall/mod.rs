//! System call handling module

mod fs;
mod process;
mod memory;

use fs::*;
use process::*;
use memory::*;

/// System call numbers
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;

/// System call dispatcher
/// 
/// # Arguments
/// * `syscall_id` - System call number (in a7)
/// * `args` - System call arguments (a0-a5, but we only use a0-a2 for most calls)
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2], args[3], args[4], args[5]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        _ => {
            println!("[syscall] Unsupported syscall_id: {}", syscall_id);
            -1
        }
    }
}
