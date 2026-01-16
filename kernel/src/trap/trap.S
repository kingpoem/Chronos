    .section .text.trampoline
.globl __alltraps
.globl __restore
.align 2

    .section .text.trampoline

    .globl KERNEL_SATP
    .align 3
KERNEL_SATP:
    .quad 0

    # Address of kernel trap handler continuation (set during initialization)
    .globl TRAP_HANDLER_KERNEL_ADDR
    .align 3
TRAP_HANDLER_KERNEL_ADDR:
    .quad 0

    # Address of __restore in trampoline VA space (set during initialization)
    # This is used to jump back to trampoline after trap_handler returns
    .globl RESTORE_TRAMPOLINE_ADDR
    .align 3
RESTORE_TRAMPOLINE_ADDR:
    .quad 0

    .section .bss
    .globl TRAP_DEBUG_ONCE
TRAP_DEBUG_ONCE:
    .quad 0

    .section .text.trampoline

# Helper function to read instruction at address
# Input: a0 = address
# Output: a0 = instruction (lower 32 bits)
.globl debug_read_instruction
debug_read_instruction:
    lw a0, 0(a0)
    ret

debug_put_hex:
    mv t5, a0
    li t0, 16
    li t1, 60
1:
    srl t2, t5, t1
    andi t2, t2, 0xf
    li t3, 10
    blt t2, t3, 2f
    addi t2, t2, 87
    j 3f
2:
    addi t2, t2, 48
3:
    mv a0, t2
    li a7, 1
    ecall
    addi t1, t1, -4
    addi t0, t0, -1
    bnez t0, 1b
    ret

    .align 2  # CRITICAL: stvec requires 4-byte alignment
__alltraps:
    # CRITICAL: Do NOT use ecall for debug printing here!
    # ecall clobbers t0-t6 (caller-saved registers) and we need t2 to hold user_satp
    
    # Check if we're coming from user mode or kernel mode
    # sstatus.SPP bit: 0 = User, 1 = Supervisor
    csrr t0, sstatus
    andi t0, t0, 0x100  # Mask to get SPP bit (bit 8)
    beqz t0, from_user_mode
    
    # From kernel/supervisor mode
    # CRITICAL: Even though we came from S-mode, we might be in user page table
    # (e.g., during __restore after csrw satp but before sret)
    # We must ALWAYS switch to kernel page table before accessing kernel code
    
    # Save user/current satp to t2 (will be stored in TrapContext to restore later)
    csrr t2, satp
    
    # Load kernel satp and switch page tables
    la t1, KERNEL_SATP
    ld t1, 0(t1)
    
    # Check if we're already in kernel page table
    beq t2, t1, skip_kernel_satp_switch
    
    # Switch to kernel page table
    csrw satp, t1
    sfence.vma zero, zero
    
skip_kernel_satp_switch:
    # Save sp to sscratch for next trap
    csrw sscratch, sp
    
    # Jump to kernel address of allocate_trap_context
    la t0, TRAP_HANDLER_KERNEL_ADDR
    ld t0, 0(t0)
    jr t0

from_user_mode:
    # From user mode: record user satp, then switch to kernel satp before using kernel stack
    # CRITICAL: t2 holds user_satp and must NOT be clobbered until saved to TrapContext!
    csrr t2, satp  # Save user satp to t2
    
    # Load kernel satp and switch page tables
    la t1, KERNEL_SATP
    ld t1, 0(t1)
    csrw satp, t1
    sfence.vma zero, zero
    
    # Switch to kernel stack (sscratch holds kernel_sp, sp holds user_sp)
    csrrw sp, sscratch, sp
    # now sp->kernel stack, sscratch->user stack
    
    # CRITICAL: Jump back to kernel address space!
    # We cannot use PC-relative addressing here (la, call, etc.) because
    # PC is in trampoline space (0xfffffffffffff...) but we want to jump
    # to kernel address space (0x8020....). 
    # Load the absolute kernel address from TRAP_HANDLER_KERNEL_ADDR and jump.
    la t0, TRAP_HANDLER_KERNEL_ADDR
    ld t0, 0(t0)
    jr t0

    .globl allocate_trap_context
allocate_trap_context:
    # allocate a TrapContext on kernel stack
    # TrapContext layout: x[32] (32*8) + sstatus (1*8) + sepc (1*8) + user_satp (1*8) + kernel_sp (1*8) = 36*8
    addi sp, sp, -36*8
    # save general-purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    sd x5, 5*8(sp)
    sd x6, 6*8(sp)
    sd x7, 7*8(sp)
    sd x8, 8*8(sp)
    sd x9, 9*8(sp)
    sd x10, 10*8(sp)
    sd x11, 11*8(sp)
    sd x12, 12*8(sp)
    sd x13, 13*8(sp)
    sd x14, 14*8(sp)
    sd x15, 15*8(sp)
    sd x16, 16*8(sp)
    sd x17, 17*8(sp)
    sd x18, 18*8(sp)
    sd x19, 19*8(sp)
    sd x20, 20*8(sp)
    sd x21, 21*8(sp)
    sd x22, 22*8(sp)
    sd x23, 23*8(sp)
    sd x24, 24*8(sp)
    sd x25, 25*8(sp)
    sd x26, 26*8(sp)
    sd x27, 27*8(sp)
    sd x28, 28*8(sp)
    sd x29, 29*8(sp)
    sd x30, 30*8(sp)
    sd x31, 31*8(sp)
    # we can use t0/t1/t2 freely, because they were saved on kernel stack
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # Save user satp captured before switching to kernel satp
    sd t2, 34*8(sp)
    # read original stack pointer from sscratch and save it on the kernel stack
    # After csrrw, sscratch contains the original sp (user stack if from user, kernel stack if from kernel)
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # Save kernel_sp for __restore to use when setting sscratch before sret
    # kernel_sp = top of kernel stack = sp after allocating TrapContext + sizeof(TrapContext)
    # This is the value that sscratch should have for the next trap
    addi t2, sp, 36*8
    sd t2, 35*8(sp)
    # Restore sscratch to kernel stack for next trap
    # This ensures sscratch always points to kernel stack after trap entry
    csrw sscratch, sp
    
    # NOTE: Do NOT use ecall here for debug printing!
    # We're in kernel mode now, so ecall would be a supervisor ecall, not user ecall.
    # This causes issues. Use SBI direct calls instead if debugging is needed.
    
    # set input argument of trap_handler(cx: &mut TrapContext)
    mv a0, sp
    call trap_handler
    
    # CRITICAL: After trap_handler returns, we must jump to __restore in TRAMPOLINE space!
    # trap_handler returns TrapContext* in a0
    # We cannot fall through to __restore at kernel address because after switching
    # to user page table, kernel addresses become invalid.
    # Load the trampoline address of __restore and jump there.
    la t0, RESTORE_TRAMPOLINE_ADDR
    ld t0, 0(t0)
    jr t0
    # Note: Never reaches here - jr t0 jumps to __restore in trampoline

__restore:
    # case1: start running app by __restore
    # case2: back to U after handling trap
    # a0: *TrapContext in kernel stack (where it was saved by __alltraps)
    
    # DEBUG: Print 'R' to show we entered __restore
    # Save a0 since we need it, use stack temporarily
    # addi sp, sp, -16
    # sd a0, 0(sp)
    # sd ra, 8(sp)
    # li a0, 'R'
    # li a7, 1
    # ecall
    # ld ra, 8(sp)
    # ld a0, 0(sp)
    # addi sp, sp, 16
    
    mv sp, a0
    # now sp->kernel stack(after allocated), sscratch->user stack (or kernel stack if from kernel)
    
    # DEBUG: Print '1' after setting sp
    # addi sp, sp, -16
    # sd ra, 0(sp)
    # li a0, '1'
    # li a7, 1
    # ecall
    # ld ra, 0(sp)
    # addi sp, sp, 16
    
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    
    # DEBUG: Print '2' after restoring sstatus/sepc
    # addi sp, sp, -16
    # sd ra, 0(sp)
    # li a0, '2'
    # li a7, 1
    # ecall
    # ld ra, 0(sp)
    # addi sp, sp, 16
    
    # Check if we're returning to kernel mode (user_satp == 0) or user mode (user_satp != 0)
    ld t2, 34*8(sp)  # Load user_satp to check
    beqz t2, restore_to_kernel
    
    # Restore to user mode with page table switch
    # Challenge: After switching page tables, kernel memory becomes inaccessible
    # Solution: Load everything we need BEFORE the switch
    # Key insight: sscratch must hold kernel_sp for next trap, not user_sp!
    
    # DEBUG: Print '3' - about to load values for user restore
    # addi sp, sp, -16
    # sd ra, 0(sp)
    # li a0, '3'
    # li a7, 1
    # ecall
    # ld ra, 0(sp)
    # addi sp, sp, 16
    
    # Step 1: Load user_sp for restoration (field 2)
    ld t2, 2*8(sp)
    
    # Step 2: Load kernel_sp for next trap (field 35)
    ld t1, 35*8(sp)
    
    # Step 3: Load user_satp into t0
    ld t0, 34*8(sp)
    
    # DEBUG: Print '4' - loaded all values, about to restore GPRs
    # addi sp, sp, -16
    # sd ra, 0(sp)
    # li a0, '4'
    # li a7, 1
    # ecall
    # ld ra, 0(sp)
    # addi sp, sp, 16
    
    # Step 4: Restore all general-purpose registers except t0, t1, t2, and sp
    # t0 holds user_satp (will be clobbered)
    # t1 holds kernel_sp (will be loaded into sscratch)
    # t2 holds user_sp (will be loaded into sp)
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    # Skip x5 (t0) - contains user_satp for page table switch
    # Skip x6 (t1) - contains kernel_sp for sscratch
    # Skip x7 (t2) - contains user_sp for sp restoration
    ld x8, 8*8(sp)
    ld x9, 9*8(sp)
    ld x10, 10*8(sp)
    ld x11, 11*8(sp)
    ld x12, 12*8(sp)
    ld x13, 13*8(sp)
    ld x14, 14*8(sp)
    ld x15, 15*8(sp)
    ld x16, 16*8(sp)
    ld x17, 17*8(sp)
    ld x18, 18*8(sp)
    ld x19, 19*8(sp)
    ld x20, 20*8(sp)
    ld x21, 21*8(sp)
    ld x22, 22*8(sp)
    ld x23, 23*8(sp)
    ld x24, 24*8(sp)
    ld x25, 25*8(sp)
    ld x26, 26*8(sp)
    ld x27, 27*8(sp)
    ld x28, 28*8(sp)
    ld x29, 29*8(sp)
    ld x30, 30*8(sp)
    ld x31, 31*8(sp)
    
    # DEBUG: Print '5' - about to switch page table
    # NOTE: We cannot use ecall here because t0/t1/t2 hold critical values
    # We need to save them temporarily
    # addi sp, sp, -32
    # sd t0, 0(sp)
    # sd t1, 8(sp)
    # sd t2, 16(sp)
    # sd ra, 24(sp)
    # li a0, '5'
    # li a7, 1
    # ecall
    # ld ra, 24(sp)
    # ld t2, 16(sp)
    # ld t1, 8(sp)
    # ld t0, 0(sp)
    # addi sp, sp, 32
    
    # Step 5: Switch to user page table
    # DEBUG: Let's try a marker that doesn't use ecall
    # We'll use the SBI legacy putchar directly inline
    # But actually, let's split the debug around the csrw satp
    
    # DEBUG: Save t0/t1/t2/ra and print '6' BEFORE csrw satp
    # addi sp, sp, -32
    # sd t0, 0(sp)
    # sd t1, 8(sp)
    # d t2, 16(sp)
    # sd ra, 24(sp)
    # li a0, '6'
    # li a7, 1
    # ecall
    # ld ra, 24(sp)
    # ld t2, 16(sp)
    # ld t1, 8(sp)
    # ld t0, 0(sp)
    # addi sp, sp, 32
    
    csrw satp, t0
    sfence.vma zero, zero
    
    # DEBUG: After page table switch - we're in user page table now
    # The trampoline is mapped in both, so we should still be here
    # But neither SBI ecall nor UART will work since we're in user page table
    # (UART is not mapped in user space)
    # 
    # Let's just continue and see if sret works
    # If we get here, the page table switch succeeded
    
    # Step 6: Set sscratch to kernel_sp for next trap entry
    csrw sscratch, t1
    
    # Step 7: Restore user sp
    mv sp, t2
    
    # NOTE: Cannot use ecall for debugging here - we're in user page table
    # but still in S-mode, so ecall would be unexpected
    
    # Step 8: Return to user mode
    # sp = user_sp, sscratch = kernel_sp (correct for next trap!)
    sret

    # If we ever return here, print marker and spin
    li a0, 'X'
    li a7, 1
    ecall
1:  j 1b

    # If we ever return here, print marker and spin
    li a0, 'X'
    li a7, 1
    ecall
1:  j 1b
restore_to_kernel:
    # Restore to kernel mode (from kernel interrupt)
    # restore general-purpuse registers except sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    ld x5, 5*8(sp)
    ld x6, 6*8(sp)
    ld x7, 7*8(sp)
    ld x8, 8*8(sp)
    ld x9, 9*8(sp)
    ld x10, 10*8(sp)
    ld x11, 11*8(sp)
    ld x12, 12*8(sp)
    ld x13, 13*8(sp)
    ld x14, 14*8(sp)
    ld x15, 15*8(sp)
    ld x16, 16*8(sp)
    ld x17, 17*8(sp)
    ld x18, 18*8(sp)
    ld x19, 19*8(sp)
    ld x20, 20*8(sp)
    ld x21, 21*8(sp)
    ld x22, 22*8(sp)
    ld x23, 23*8(sp)
    ld x24, 24*8(sp)
    ld x25, 25*8(sp)
    ld x26, 26*8(sp)
    ld x27, 27*8(sp)
    ld x28, 28*8(sp)
    ld x29, 29*8(sp)
    ld x30, 30*8(sp)
    ld x31, 31*8(sp)
    # Restore kernel stack pointer from saved value in TrapContext
    # For kernel mode, the saved sp (x[2]) is the original kernel stack before trap
    ld sp, 2*8(sp)
    # Restore sscratch to kernel stack for next trap
    csrw sscratch, sp
    # Keep kernel address space (don't change satp)
    sret
