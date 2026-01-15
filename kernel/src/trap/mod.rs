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
    // Print all S-mode registers for debugging
    print_all_smode_registers("[Trap Entry]");
    
    let scause = scause::read();
    let stval = stval::read();
    
    // Determine if we're in kernel mode or user mode
    // Check sstatus.SPP bit: 0 = User, 1 = Supervisor
    use riscv::register::sstatus;
    let is_user_mode = cx.sstatus.spp() == sstatus::SPP::User;
    
    // Debug: log trap type and mode (use simple string matching to avoid format! allocation)
    let scause_code = scause.bits();
    crate::sbi::console_putstr("[Trap] Trap: scause=0x");
    print_hex_usize(scause_code);
    crate::sbi::console_putstr(" (");
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
        scause::Trap::Exception(scause::Exception::InstructionFault) => {
            crate::sbi::console_putstr("InstructionFault");
        }
        scause::Trap::Exception(scause::Exception::LoadFault) => {
            crate::sbi::console_putstr("LoadFault");
        }
        scause::Trap::Exception(scause::Exception::StoreFault) => {
            crate::sbi::console_putstr("StoreFault");
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            crate::sbi::console_putstr("IllegalInstruction");
        }
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            crate::sbi::console_putstr("UserEnvCall");
        }
        _ => {
            crate::sbi::console_putstr("Unknown");
        }
    }
    if is_user_mode {
        crate::sbi::console_putstr(" User");
    } else {
        crate::sbi::console_putstr(" Kernel");
    }
    crate::sbi::console_putstr("), stval=0x");
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
            // CRITICAL: Only trigger task switching from user mode interrupts
            // Kernel mode interrupts should NOT trigger task switching
            // This is the rCore way: kernel mode interrupts are handled synchronously
            // and should not cause context switches
            
            set_next_timer();
            
            if is_user_mode {
                // User mode interrupt: can trigger preemptive scheduling
                let task_manager = crate::task::TASK_MANAGER.lock();
                let task_count = task_manager.task_count();
                drop(task_manager);
                
                if task_count > 0 {
                    let mut scheduler = crate::task::SCHEDULER.lock();
                    if scheduler.tick() {
                        // Time slice expired, switch task
                        drop(scheduler);
                        crate::task::switch_task();
                    } else {
                        drop(scheduler);
                    }
                }
            } else {
                // Kernel mode interrupt: do NOT trigger task switching
                // Just set the next timer and return
                // This prevents issues when kernel is executing and gets interrupted
            }
        }
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            cx.sepc += 4;
            // System call arguments: a0-a5 (x[10]-x[15]), syscall number in a7 (x[17])
            cx.x[10] = syscall(
                cx.x[17],
                [cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]]
            ) as usize;
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
        scause::Trap::Exception(scause::Exception::InstructionFault) => {
            crate::sbi::console_putstr("[Trap] InstructionFault: ");
            if is_user_mode {
                crate::sbi::console_putstr("User mode\n");
                
                // Read and print the instruction that caused the fault
                // We need to translate user VA to PA first, then read from kernel address space
                let task_manager = crate::task::TASK_MANAGER.lock();
                if let Some(current_pid) = task_manager.get_current_task() {
                    if let Some(task) = task_manager.get_task(current_pid) {
                        if let Some(pa) = task.memory_set.translate(cx.sepc) {
                            extern "C" {
                                fn debug_read_instruction(addr: usize) -> u32;
                            }
                            let instr = unsafe { debug_read_instruction(pa) };
                            crate::sbi::console_putstr("[Debug] Instruction at 0x");
                            crate::trap::print_hex_usize(cx.sepc);
                            crate::sbi::console_putstr(" (PA=0x");
                            crate::trap::print_hex_usize(pa);
                            crate::sbi::console_putstr("): 0x");
                            crate::trap::print_hex_usize(instr as usize);
                            crate::sbi::console_putstr("\n");
                        }
                    }
                }
                drop(task_manager);
                
                // Detailed debugging information
                let task_manager = crate::task::TASK_MANAGER.lock();
                if let Some(current_pid) = task_manager.get_current_task() {
                    crate::sbi::console_putstr("[Debug] Task PID: ");
                    crate::trap::print_hex_usize(current_pid);
                    crate::sbi::console_putstr("\n");
                    crate::sbi::console_putstr("[Debug] Fault address (stval/sepc): 0x");
                    crate::trap::print_hex_usize(cx.sepc);
                    crate::sbi::console_putstr("\n");
                    
                    if let Some(task) = task_manager.get_task(current_pid) {
                        crate::sbi::console_putstr("[Debug] Task entry point: 0x");
                        crate::trap::print_hex_usize(task.entry_point);
                        crate::sbi::console_putstr("\n");
                        crate::sbi::console_putstr("[Debug] User stack: 0x");
                        crate::trap::print_hex_usize(task.user_sp);
                        crate::sbi::console_putstr("\n");
                        
                        // Check if the page is mapped and what permissions it has
                        use crate::mm::memory_layout::VirtAddr;
                        let va = VirtAddr::new(cx.sepc);
                        let vpn = va.page_number();
                        
                        if let Some((ppn, flags)) = task.memory_set.page_table().translate(vpn) {
                            crate::sbi::console_putstr("[Debug] Page IS mapped: VPN=0x");
                            crate::trap::print_hex_usize(vpn.0);
                            crate::sbi::console_putstr(" -> PPN=0x");
                            crate::trap::print_hex_usize(ppn.0);
                            crate::sbi::console_putstr(", flags=0x");
                            crate::trap::print_hex_usize(flags.bits() as usize);
                            crate::sbi::console_putstr(" (");
                            if flags.contains(crate::mm::page_table::PTEFlags::R) { crate::sbi::console_putstr("R"); }
                            if flags.contains(crate::mm::page_table::PTEFlags::W) { crate::sbi::console_putstr("W"); }
                            if flags.contains(crate::mm::page_table::PTEFlags::X) { crate::sbi::console_putstr("X"); }
                            if flags.contains(crate::mm::page_table::PTEFlags::U) { crate::sbi::console_putstr("U"); }
                            crate::sbi::console_putstr(")\n");
                            
                            // If X or U is missing, that's the problem
                            if !flags.contains(crate::mm::page_table::PTEFlags::X) {
                                crate::sbi::console_putstr("[Debug] ERROR: Page is missing X (Execute) permission!\n");
                            }
                            if !flags.contains(crate::mm::page_table::PTEFlags::U) {
                                crate::sbi::console_putstr("[Debug] ERROR: Page is missing U (User) permission!\n");
                            }
                        } else {
                            crate::sbi::console_putstr("[Debug] Page NOT mapped: VPN=0x");
                            crate::trap::print_hex_usize(vpn.0);
                            crate::sbi::console_putstr(", VA=0x");
                            crate::trap::print_hex_usize(cx.sepc);
                            crate::sbi::console_putstr("\n");
                        }
                        
                        // Print register values for debugging
                        crate::sbi::console_putstr("[Debug] Registers: sp=0x");
                        crate::trap::print_hex_usize(cx.x[2]);
                        crate::sbi::console_putstr(", ra=0x");
                        crate::trap::print_hex_usize(cx.x[1]);
                        crate::sbi::console_putstr("\n");
                    }
                }
                drop(task_manager);
                
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
        scause::Trap::Exception(scause::Exception::InstructionPageFault) => {
            crate::sbi::console_putstr("[Trap] InstructionPageFault: ");
            if is_user_mode {
                crate::sbi::console_putstr("User mode\n");
                
                // Detailed debugging information
                let task_manager = crate::task::TASK_MANAGER.lock();
                if let Some(current_pid) = task_manager.get_current_task() {
                    crate::sbi::console_putstr("[Debug] Task PID: ");
                    crate::trap::print_hex_usize(current_pid);
                    crate::sbi::console_putstr("\n");
                    
                    if let Some(task) = task_manager.get_task(current_pid) {
                        crate::sbi::console_putstr("[Debug] Fault address (stval/sepc): 0x");
                        crate::trap::print_hex_usize(cx.sepc);
                        crate::sbi::console_putstr("\n");
                        crate::sbi::console_putstr("[Debug] Task entry point: 0x");
                        crate::trap::print_hex_usize(task.entry_point);
                        crate::sbi::console_putstr("\n");
                        crate::sbi::console_putstr("[Debug] User stack: 0x");
                        crate::trap::print_hex_usize(task.user_sp);
                        crate::sbi::console_putstr("\n");
                        
                        // Check page mapping
                        use crate::mm::memory_layout::VirtAddr;
                        let va = VirtAddr::new(cx.sepc);
                        let vpn = va.page_number();
                        
                        if let Some((ppn, flags)) = task.memory_set.page_table().translate(vpn) {
                            crate::sbi::console_putstr("[Debug] Page IS mapped: VPN=0x");
                            crate::trap::print_hex_usize(vpn.0);
                            crate::sbi::console_putstr(" -> PPN=0x");
                            crate::trap::print_hex_usize(ppn.0);
                            crate::sbi::console_putstr(", flags=0x");
                            crate::trap::print_hex_usize(flags.bits() as usize);
                            crate::sbi::console_putstr(" (");
                            if flags.contains(crate::mm::page_table::PTEFlags::R) { crate::sbi::console_putstr("R"); }
                            if flags.contains(crate::mm::page_table::PTEFlags::W) { crate::sbi::console_putstr("W"); }
                            if flags.contains(crate::mm::page_table::PTEFlags::X) { crate::sbi::console_putstr("X"); }
                            if flags.contains(crate::mm::page_table::PTEFlags::U) { crate::sbi::console_putstr("U"); }
                            crate::sbi::console_putstr(")\n");
                            
                            if !flags.contains(crate::mm::page_table::PTEFlags::X) {
                                crate::sbi::console_putstr("[Debug] ERROR: Page missing X (Execute) permission!\n");
                            }
                        } else {
                            crate::sbi::console_putstr("[Debug] Page NOT mapped: VPN=0x");
                            crate::trap::print_hex_usize(vpn.0);
                            crate::sbi::console_putstr(", VA=0x");
                            crate::trap::print_hex_usize(cx.sepc);
                            crate::sbi::console_putstr("\n");
                        }
                        
                        // Print register values for debugging
                        crate::sbi::console_putstr("[Debug] Registers: sp=0x");
                        crate::trap::print_hex_usize(cx.x[2]);
                        crate::sbi::console_putstr(", ra=0x");
                        crate::trap::print_hex_usize(cx.x[1]);
                        crate::sbi::console_putstr("\n");
                    }
                }
                drop(task_manager);
                
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
/// NOTE: This function only enables timer interrupt in sie, but does NOT set the timer
/// The timer should be set in switch_task() right before switching to user mode
/// This prevents timer interrupts from triggering during kernel initialization
pub fn enable_timer_interrupt() {
    crate::sbi::console_putstr("[Trap] Setting up timer interrupt...\n");
    
    // Disable interrupts while setting up timer
    unsafe {
        sstatus::clear_sie();
    }
    
    unsafe {
        // Enable timer interrupt in sie (but interrupts are still disabled via sstatus::SIE)
        sie::set_stimer();
    }
    crate::sbi::console_putstr("[Trap] Timer interrupt enabled in sie (interrupts still disabled in sstatus)\n");
    
    // CRITICAL: Do NOT set timer here!
    // The timer should be set in switch_task() right before jumping to __restore
    // This ensures we have enough time to complete initialization and switch to user mode
    // before the first timer interrupt triggers
    crate::sbi::console_putstr("[Trap] Timer will be set in switch_task() before switching to user mode\n");
    
    // DO NOT enable sstatus::SIE here - it will be enabled when we restore sstatus in __restore
    // The trap context's sstatus should have SIE bit set, so when we switch to user mode,
    // interrupts will be automatically enabled
}

/// Set next timer interrupt (10ms interval for more responsive scheduling)
/// This function should be called right before switching to user mode for the first time,
/// and also in trap_handler after handling timer interrupts
pub fn set_next_timer() {
    use crate::config::CLOCK_FREQ;
    use crate::sbi;
    
    let time = sbi::get_time();
    // Set timer to 10ms intervals (CLOCK_FREQ / 100 = 10ms)
    // This gives us 100 ticks per second, and with time_slice=10, each task gets 100ms
    let next = time + (CLOCK_FREQ / 100) as u64; // 10ms
    sbi::set_timer(next);
}

/// Print all S-mode (Supervisor mode) system registers
/// This is useful for debugging trap handling
pub fn print_all_smode_registers(label: &str) {
    use riscv::register::{satp, sstatus, sscratch, stvec, sie, sip, scause, stval};
    
    crate::sbi::console_putstr("\n");
    crate::sbi::console_putstr(label);
    crate::sbi::console_putstr(" S-mode Registers:\n");
    
    // sstatus - Supervisor status register
    unsafe {
        let sstatus_val: usize;
        core::arch::asm!("csrr {}, sstatus", out(reg) sstatus_val);
        crate::sbi::console_putstr("  sstatus=0x");
        print_hex_usize(sstatus_val);
        let sstatus_reg = sstatus::read();
        crate::sbi::console_putstr(" (SPP=");
        match sstatus_reg.spp() {
            sstatus::SPP::User => crate::sbi::console_putstr("User"),
            sstatus::SPP::Supervisor => crate::sbi::console_putstr("Supervisor"),
        }
        crate::sbi::console_putstr(", SIE=");
        if sstatus_reg.sie() {
            crate::sbi::console_putstr("1");
        } else {
            crate::sbi::console_putstr("0");
        }
        crate::sbi::console_putstr(", SPIE=");
        if sstatus_reg.spie() {
            crate::sbi::console_putstr("1");
        } else {
            crate::sbi::console_putstr("0");
        }
        crate::sbi::console_putstr(")\n");
    }
    
    // sie - Supervisor interrupt enable register
    let sie_val = sie::read().bits();
    crate::sbi::console_putstr("  sie=0x");
    print_hex_usize(sie_val);
    crate::sbi::console_putstr(" (");
    let sie_reg = sie::read();
    if sie_reg.stimer() { crate::sbi::console_putstr("STimer "); }
    if sie_reg.sext() { crate::sbi::console_putstr("SExt "); }
    if sie_reg.ssoft() { crate::sbi::console_putstr("SSoft "); }
    crate::sbi::console_putstr(")\n");
    
    // sip - Supervisor interrupt pending register
    let sip_val = sip::read().bits();
    crate::sbi::console_putstr("  sip=0x");
    print_hex_usize(sip_val);
    crate::sbi::console_putstr(" (");
    let sip_reg = sip::read();
    if sip_reg.stimer() { crate::sbi::console_putstr("STimer "); }
    if sip_reg.sext() { crate::sbi::console_putstr("SExt "); }
    if sip_reg.ssoft() { crate::sbi::console_putstr("SSoft "); }
    crate::sbi::console_putstr(")\n");
    
    // stvec - Supervisor trap vector base address register
    let stvec_val = stvec::read().bits();
    crate::sbi::console_putstr("  stvec=0x");
    print_hex_usize(stvec_val);
    match stvec::read().trap_mode() {
        Some(riscv::register::mtvec::TrapMode::Direct) => crate::sbi::console_putstr(" (Direct)\n"),
        Some(riscv::register::mtvec::TrapMode::Vectored) => crate::sbi::console_putstr(" (Vectored)\n"),
        None => crate::sbi::console_putstr(" (Unknown)\n"),
    }
    
    // scounteren - Supervisor counter enable register
    unsafe {
        let scounteren_val: usize;
        core::arch::asm!("csrr {}, scounteren", out(reg) scounteren_val);
        crate::sbi::console_putstr("  scounteren=0x");
        print_hex_usize(scounteren_val);
        crate::sbi::console_putstr("\n");
    }
    
    // sscratch - Supervisor scratch register
    let sscratch_val = sscratch::read();
    crate::sbi::console_putstr("  sscratch=0x");
    print_hex_usize(sscratch_val);
    crate::sbi::console_putstr("\n");
    
    // sepc - Supervisor exception program counter
    unsafe {
        let sepc_val: usize;
        core::arch::asm!("csrr {}, sepc", out(reg) sepc_val);
        crate::sbi::console_putstr("  sepc=0x");
        print_hex_usize(sepc_val);
        crate::sbi::console_putstr("\n");
    }
    
    // scause - Supervisor cause register
    let scause_val = scause::read().bits();
    crate::sbi::console_putstr("  scause=0x");
    print_hex_usize(scause_val);
    let scause_reg = scause::read();
    match scause_reg.cause() {
        scause::Trap::Interrupt(interrupt) => {
            crate::sbi::console_putstr(" (Interrupt: ");
            match interrupt {
                scause::Interrupt::SupervisorTimer => crate::sbi::console_putstr("SupervisorTimer"),
                scause::Interrupt::SupervisorExternal => crate::sbi::console_putstr("SupervisorExternal"),
                scause::Interrupt::SupervisorSoft => crate::sbi::console_putstr("SupervisorSoft"),
                _ => crate::sbi::console_putstr("Other"),
            }
            crate::sbi::console_putstr(")\n");
        }
        scause::Trap::Exception(exception) => {
            crate::sbi::console_putstr(" (Exception: ");
            match exception {
                scause::Exception::InstructionFault => crate::sbi::console_putstr("InstructionFault"),
                scause::Exception::IllegalInstruction => crate::sbi::console_putstr("IllegalInstruction"),
                scause::Exception::Breakpoint => crate::sbi::console_putstr("Breakpoint"),
                scause::Exception::LoadFault => crate::sbi::console_putstr("LoadFault"),
                scause::Exception::StoreFault => crate::sbi::console_putstr("StoreFault"),
                scause::Exception::UserEnvCall => crate::sbi::console_putstr("UserEnvCall"),
                scause::Exception::InstructionPageFault => crate::sbi::console_putstr("InstructionPageFault"),
                scause::Exception::LoadPageFault => crate::sbi::console_putstr("LoadPageFault"),
                scause::Exception::StorePageFault => crate::sbi::console_putstr("StorePageFault"),
                _ => crate::sbi::console_putstr("Other"),
            }
            crate::sbi::console_putstr(")\n");
        }
    }
    
    // stval - Supervisor trap value register
    let stval_val = stval::read();
    crate::sbi::console_putstr("  stval=0x");
    print_hex_usize(stval_val);
    crate::sbi::console_putstr("\n");
    
    // satp - Supervisor address translation and protection register
    let satp_val = satp::read().bits();
    crate::sbi::console_putstr("  satp=0x");
    print_hex_usize(satp_val);
    let mode = (satp_val >> 60) & 0xF;
    if mode == 8 {
        crate::sbi::console_putstr(" (SV39, PPN=0x");
        print_hex_usize(satp_val & 0x0FFF_FFFF_FFFF);
        crate::sbi::console_putstr(")\n");
    } else if mode == 0 {
        crate::sbi::console_putstr(" (Bare, no paging)\n");
    } else {
        crate::sbi::console_putstr(" (Mode=");
        print_hex_usize(mode);
        crate::sbi::console_putstr(")\n");
    }
    
    crate::sbi::console_putstr("\n");
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

