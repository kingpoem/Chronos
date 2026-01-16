//! File system related system calls
//!
//! Implements file system operations like read, write, etc.

use crate::task::TASK_MANAGER;

const FD_STDOUT: usize = 1;

/// Write to a file descriptor
/// 
/// # Arguments
/// * `fd` - File descriptor
/// * `buf` - Buffer pointer (user virtual address)
/// * `len` - Length of buffer in bytes
/// 
/// # Returns
/// * Number of bytes written, or -1 on error
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            // Get current task's page table to translate user virtual address
            let task_manager = TASK_MANAGER.lock();
            let current_pid = match task_manager.get_current_task() {
                Some(pid) => pid,
                None => {
                    return -1;
                }
            };

            let task = match task_manager.get_task(current_pid) {
                Some(task) => task,
                None => {
                    return -1;
                }
            };
            
            // Get user page table
            let user_page_table = &task.memory_set.page_table();
            
            // Translate user virtual address to kernel virtual address
            // This safely accesses user space data from kernel space
            let user_va = buf as usize;
            let buffers = user_page_table.translated_byte_buffer_readonly(user_va, len);
            
            drop(task_manager);
            
            // Write all buffers to console
            let mut total_written = 0;
            for buffer in buffers {
                // Convert bytes to string and print
                match core::str::from_utf8(buffer) {
                    Ok(s) => {
                        print!("{}", s);
                        total_written += buffer.len();
                    }
                    Err(_) => {
                        // If not valid UTF-8, print as raw bytes
                        for &byte in buffer {
                            crate::sbi::console_putchar(byte);
                            total_written += 1;
                        }
                    }
                }
            }
            
            total_written as isize
        }
        _ => -1,
    }
}
