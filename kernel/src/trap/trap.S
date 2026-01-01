    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    # Check if we're coming from user mode or kernel mode
    # sstatus.SPP bit: 0 = User, 1 = Supervisor
    # We need to read sstatus BEFORE swapping, so save it to a temporary register
    csrr t0, sstatus
    # Check SPP bit (bit 8): if 0, we're from user mode; if 1, we're from kernel mode
    andi t0, t0, 0x100  # Mask to get SPP bit (bit 8)
    beqz t0, from_user_mode
    # From kernel mode: don't swap, sscratch and sp both point to kernel stack
    # But we need to ensure sscratch points to the current kernel stack (sp)
    # Update sscratch to current sp for consistency
    csrw sscratch, sp
    j allocate_trap_context
from_user_mode:
    # From user mode: swap sp and sscratch
    # sscratch should point to kernel stack, sp points to user stack
    csrrw sp, sscratch, sp
    # now sp->kernel stack, sscratch->user stack
allocate_trap_context:
    # allocate a TrapContext on kernel stack
    addi sp, sp, -34*8
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
    # read original stack pointer from sscratch and save it on the kernel stack
    # After csrrw, sscratch contains the original sp (user stack if from user, kernel stack if from kernel)
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # Restore sscratch to kernel stack for next trap
    # This ensures sscratch always points to kernel stack after trap entry
    csrw sscratch, sp
    # set input argument of trap_handler(cx: &mut TrapContext)
    mv a0, sp
    call trap_handler

__restore:
    # case1: start running app by __restore
    # case2: back to U after handling trap
    # a0: *TrapContext in kernel stack (where it was saved by __alltraps)
    mv sp, a0
    # now sp->kernel stack(after allocated), sscratch->user stack (or kernel stack if from kernel)
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # Get user token from trap context (stored at offset 34*8, after sstatus and sepc)
    # TrapContext layout: x[32] (32*8) + sstatus (1*8) + sepc (1*8) + user_satp (1*8)
    ld t2, 34*8(sp)  # user_satp
    # Debug: We can't easily print in assembly, but we can check the value
    # If t2 is 0, we go to kernel mode; if non-zero, we go to user mode
    # Check if we're returning to kernel mode (user_satp == 0 means kernel mode)
    beqz t2, restore_to_kernel
    # Restore to user mode
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
    # Load user stack pointer
    ld sp, 2*8(sp)
    # Switch to user address space (user token is in t2)
    csrw satp, t2
    sfence.vma
    # sstatus has already been restored above (line 84)
    # It contains the correct interrupt enable state (SIE bit)
    # No need to modify it again
    sret
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
