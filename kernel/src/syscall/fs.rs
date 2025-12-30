use crate::print;

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            // Get current task to translate user space address
            let task = crate::task::current_task().expect("No current task in sys_write");
            let inner = task.inner_exclusive_access();
            
            // Translate the user buffer address to kernel accessible memory
            // We need to copy the data from user space to kernel space
            let mut kernel_buf = alloc::vec::Vec::with_capacity(len);
            for i in 0..len {
                let user_va = (buf as usize) + i;
                if let Some(phys_addr) = inner.memory_set.translate(user_va) {
                    let byte = unsafe { *(phys_addr as *const u8) };
                    kernel_buf.push(byte);
                } else {
                    crate::println!("[syscall] Failed to translate user address {:#x}", user_va);
                    return -1;
                }
            }
            drop(inner);
            
            let str = core::str::from_utf8(&kernel_buf).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            crate::println!("Unsupported fd in sys_write: {}", fd);
            -1
        }
    }
}
