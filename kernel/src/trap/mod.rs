//! Interrupt and trap handling module

use crate::syscall::syscall;
use crate::global_asm;
use riscv::register::mtvec::TrapMode;
use riscv::register::{scause, sie, sstatus, stval, stvec};

pub mod context;
pub use context::TrapContext;

global_asm!(include_str!("trap.S"));

pub fn init() {
    crate::println!("[Trap] Starting trap handler initialization...");
    extern "C" {
        fn __alltraps();
    }
    
    // CRITICAL: Set sscratch to current kernel stack pointer
    // This is required because __alltraps uses csrrw to swap sp and sscratch
    // If sscratch is not set, the kernel stack pointer will be lost
    unsafe {
        let current_sp: usize;
        core::arch::asm!("mv {}, sp", out(reg) current_sp);
        core::arch::asm!("csrw sscratch, {}", in(reg) current_sp);
    }
    crate::println!("[Trap] sscratch initialized to kernel stack");
    
    crate::println!("[Trap] Setting trap vector...");
    unsafe {
        stvec::write(__alltraps as *const () as usize, TrapMode::Direct);
    }
    
    // NOTE: Do NOT enable timer interrupt here!
    // Timer interrupt should be enabled AFTER tasks are loaded.
    // Enabling it too early can cause issues during initialization.
    // Timer interrupt will be enabled in task::switch_task() or after task::load_apps()
    
    crate::sbi::console_putstr("[Trap] Trap handler initialized (timer interrupt will be enabled later)\n");
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    
    // Determine if we're in kernel mode or user mode
    // Check sstatus.SPP bit: 0 = User, 1 = Supervisor
    use riscv::register::sstatus;
    let is_user_mode = cx.sstatus.spp() == sstatus::SPP::User;
    
    // Debug: log trap type and mode (use simple string matching to avoid format! allocation)
    crate::sbi::console_putstr("[Trap] Trap: ");
    match scause.cause() {
        scause::Trap::Interrupt(scause::Interrupt::SupervisorTimer) => {
            crate::sbi::console_putstr("Timer");
        }
        scause::Trap::Exception(scause::Exception::StorePageFault) => {
            crate::sbi::console_putstr("StorePageFault");
        }
        scause::Trap::Exception(scause::Exception::LoadPageFault) => {
            crate::sbi::console_putstr("LoadPageFault");
        }
        scause::Trap::Exception(scause::Exception::InstructionPageFault) => {
            crate::sbi::console_putstr("InstructionPageFault");
        }
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            crate::sbi::console_putstr("UserEnvCall");
        }
        _ => {
            crate::sbi::console_putstr("Other");
        }
    }
    if is_user_mode {
        crate::sbi::console_putstr(" (User)");
    } else {
        crate::sbi::console_putstr(" (Kernel)");
    }
    crate::sbi::console_putstr(", stval=0x");
    print_hex_usize(stval);
    crate::sbi::console_putstr(", sepc=0x");
    print_hex_usize(cx.sepc);
    crate::sbi::console_putstr("\n");
    
    // Store user token in trap context for __restore to use
    // Only do this if we have a current task and we're in user mode
    if is_user_mode {
        let task_manager = crate::task::TASK_MANAGER.lock();
        if let Some(current_pid) = task_manager.get_current_task() {
            if let Some(task) = task_manager.get_task(current_pid) {
                cx.user_satp = task.get_user_token();
            }
        }
        drop(task_manager);
    } else {
        // Kernel mode interrupt - set user_satp to 0 to indicate kernel mode
        cx.user_satp = 0;
    }
    
    match scause.cause() {
        scause::Trap::Interrupt(scause::Interrupt::SupervisorTimer) => {
            // Timer interrupt - trigger preemptive scheduling
            crate::sbi::console_putstr("[Timer] Timer interrupt\n");
            set_next_timer();
            let task_manager = crate::task::TASK_MANAGER.lock();
            let task_count = task_manager.task_count();
            drop(task_manager);
            
            if task_count > 0 {
                let mut scheduler = crate::task::SCHEDULER.lock();
                if scheduler.tick() {
                    // Time slice expired, switch task
                    drop(scheduler);
                    crate::sbi::console_putstr("[Timer] Time slice expired, switching task\n");
                    crate::task::switch_task();
                } else {
                    drop(scheduler);
                }
            } else {
                // No tasks yet, just clear the interrupt
                crate::sbi::console_putstr("[Timer] No tasks, ignoring\n");
            }
        }
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        scause::Trap::Exception(scause::Exception::StoreFault)
        | scause::Trap::Exception(scause::Exception::StorePageFault) => {
            crate::sbi::console_putstr("[Trap] StorePageFault: ");
            if is_user_mode {
                crate::sbi::console_putstr("User mode, killing task\n");
                crate::task::exit_current_and_run_next(-1);
            } else {
                // Kernel mode page fault - this is a serious error
                // Add detailed debugging information
                crate::sbi::console_putstr("Kernel mode PANIC!\n");
                crate::sbi::console_putstr("[Debug] Fault address (stval): 0x");
                print_hex_usize(stval);
                crate::sbi::console_putstr("\n");
                crate::sbi::console_putstr("[Debug] Fault instruction (sepc): 0x");
                print_hex_usize(cx.sepc);
                crate::sbi::console_putstr("\n");
                
                // Check if address is in kernel space
                extern "C" {
                    fn stext();
                    fn ekernel();
                }
                let kernel_start = stext as *const () as usize;
                let kernel_end = ekernel as *const () as usize;
                crate::sbi::console_putstr("[Debug] Kernel range: 0x");
                print_hex_usize(kernel_start);
                crate::sbi::console_putstr(" - 0x");
                print_hex_usize(kernel_end);
                crate::sbi::console_putstr("\n");
                
                // Check if address is in MMIO region
                const MMIO_START: usize = 0x2000000;
                const MMIO_END: usize = 0x10000000;
                if stval >= MMIO_START && stval < MMIO_END {
                    crate::sbi::console_putstr("[Debug] Address is in MMIO region\n");
                }
                
                // Check if address is in physical memory region
                use crate::config::memory_layout::MEMORY_END;
                if stval >= kernel_end && stval < MEMORY_END {
                    crate::sbi::console_putstr("[Debug] Address is in physical memory region\n");
                }
                
                // Try to translate the address
                let kernel_space = crate::mm::KERNEL_SPACE_INTERNAL.lock();
                if let Some(ref ks) = *kernel_space {
                    if let Some(pa) = ks.translate(stval) {
                        crate::sbi::console_putstr("[Debug] Translation: VA 0x");
                        print_hex_usize(stval);
                        crate::sbi::console_putstr(" -> PA 0x");
                        print_hex_usize(pa);
                        crate::sbi::console_putstr("\n");
                    } else {
                        crate::sbi::console_putstr("[Debug] Translation failed: address not mapped!\n");
                    }
                }
                drop(kernel_space);
                
                // Print register values for debugging
                crate::sbi::console_putstr("[Debug] Registers:\n");
                crate::sbi::console_putstr("[Debug]   sp=0x");
                print_hex_usize(cx.x[2]);
                crate::sbi::console_putstr(", ra=0x");
                print_hex_usize(cx.x[1]);
                crate::sbi::console_putstr("\n");
                
                crate::sbi::shutdown();
            }
        }
        scause::Trap::Exception(scause::Exception::LoadFault)
        | scause::Trap::Exception(scause::Exception::LoadPageFault) => {
            crate::sbi::console_putstr("[Trap] LoadPageFault: ");
            if is_user_mode {
                crate::sbi::console_putstr("User mode, killing task\n");
                crate::task::exit_current_and_run_next(-1);
            } else {
                // Kernel mode page fault - this is a serious error
                crate::sbi::console_putstr("Kernel mode PANIC!\n");
                crate::sbi::console_putstr("stval=0x");
                print_hex_usize(stval);
                crate::sbi::console_putstr(", sepc=0x");
                print_hex_usize(cx.sepc);
                crate::sbi::console_putstr("\n");
                crate::sbi::shutdown();
            }
        }
        scause::Trap::Exception(scause::Exception::InstructionFault)
        | scause::Trap::Exception(scause::Exception::InstructionPageFault) => {
            crate::sbi::console_putstr("[Trap] InstructionPageFault: ");
            if is_user_mode {
                crate::sbi::console_putstr("User mode, killing task\n");
                crate::task::exit_current_and_run_next(-1);
            } else {
                // Kernel mode page fault - this is a serious error
                crate::sbi::console_putstr("Kernel mode PANIC!\n");
                crate::sbi::console_putstr("stval=0x");
                print_hex_usize(stval);
                crate::sbi::console_putstr(", sepc=0x");
                print_hex_usize(cx.sepc);
                crate::sbi::console_putstr("\n");
                crate::sbi::shutdown();
            }
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            crate::sbi::console_putstr("[Trap] IllegalInstruction\n");
            if is_user_mode {
                crate::task::exit_current_and_run_next(-1);
            } else {
                crate::sbi::console_putstr("[Trap] PANIC: IllegalInstruction in kernel!\n");
                crate::sbi::shutdown();
            }
        }
        _ => {
            crate::sbi::console_putstr("[Trap] Unsupported trap\n");
            if is_user_mode {
                crate::task::exit_current_and_run_next(-1);
            } else {
                crate::sbi::console_putstr("[Trap] PANIC: Unsupported trap in kernel!\n");
                crate::sbi::shutdown();
            }
        }
    }
    cx
}

/// Enable timer interrupt (should be called after tasks are loaded)
pub fn enable_timer_interrupt() {
    crate::sbi::console_putstr("[Trap] Enabling timer interrupt...\n");
    unsafe {
        // Enable timer interrupt
        sie::set_stimer();
    }
    crate::sbi::console_putstr("[Trap] Enabling supervisor interrupts...\n");
    unsafe {
        // Enable supervisor interrupts
        sstatus::set_sie();
    }
    crate::sbi::console_putstr("[Trap] Supervisor interrupts enabled\n");
    
    // Set initial timer
    set_next_timer();
    crate::sbi::console_putstr("[Trap] Initial timer set\n");
}

/// Set next timer interrupt (10ms interval for more responsive scheduling)
fn set_next_timer() {
    use crate::config::CLOCK_FREQ;
    use crate::sbi;
    
    let time = sbi::get_time();
    // Set timer to 10ms intervals (CLOCK_FREQ / 100 = 10ms)
    // This gives us 100 ticks per second, and with time_slice=10, each task gets 100ms
    let next = time + (CLOCK_FREQ / 100) as u64; // 10ms
    sbi::set_timer(next);
}

/// Print a usize as hexadecimal (helper for trap handler)
pub fn print_hex_usize(n: usize) {
    let hex_digits = b"0123456789abcdef";
    let mut buffer = [0u8; 16];
    let mut num = n;
    let mut i = 0;

    if num == 0 {
        crate::sbi::console_putchar(b'0');
        return;
    }

    while num > 0 && i < 16 {
        buffer[i] = hex_digits[(num & 0xF) as usize];
        num >>= 4;
        i += 1;
    }

    for j in (0..i).rev() {
        crate::sbi::console_putchar(buffer[j]);
    }
}

/// Print critical register state for debugging
pub fn print_critical_registers(label: &str) {
    use riscv::register::{satp, sstatus, sscratch};
    
    crate::sbi::console_putstr(label);
    crate::sbi::console_putstr(" Critical Registers:\n");
    
    // Print sp (stack pointer)
    unsafe {
        let sp: usize;
        core::arch::asm!("mv {}, sp", out(reg) sp);
        crate::sbi::console_putstr("  sp=0x");
        print_hex_usize(sp);
        crate::sbi::console_putstr("\n");
    }
    
    // Print satp (page table register)
    let satp_val = satp::read().bits();
    crate::sbi::console_putstr("  satp=0x");
    print_hex_usize(satp_val);
    if satp_val >> 60 == 8 {
        crate::sbi::console_putstr(" (SV39 enabled)");
    } else {
        crate::sbi::console_putstr(" (paging disabled)");
    }
    crate::sbi::console_putstr("\n");
    
    // Print sstatus
    unsafe {
        let sstatus_reg = sstatus::read();
        crate::sbi::console_putstr("  sstatus=0x");
        // Read sstatus using inline assembly
        let sstatus_val: usize;
        core::arch::asm!("csrr {}, sstatus", out(reg) sstatus_val);
        print_hex_usize(sstatus_val);
        if sstatus_reg.sie() {
            crate::sbi::console_putstr(" (SIE enabled)");
        } else {
            crate::sbi::console_putstr(" (SIE disabled)");
        }
        crate::sbi::console_putstr("\n");
    }
    
    // Print sscratch
    let sscratch_val = sscratch::read();
    crate::sbi::console_putstr("  sscratch=0x");
    print_hex_usize(sscratch_val);
    crate::sbi::console_putstr("\n");
    
    // Print stvec (trap vector)
    let stvec_val = stvec::read().bits();
    crate::sbi::console_putstr("  stvec=0x");
    print_hex_usize(stvec_val);
    crate::sbi::console_putstr("\n");
}

