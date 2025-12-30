.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm

    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    csrrw sp, sscratch, sp
    # Now sp points to TrapContext in user space, sscratch points to user stack
    # Save general purpose registers
    sd x1, 1*8(sp)
    # Skip sp (x2), we'll save it later
    sd x3, 3*8(sp)
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # Save sstatus and sepc
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # Save user sp
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # Load kernel_satp into t0
    ld t0, 34*8(sp)
    # Load trap_handler into t1
    ld t1, 36*8(sp)
    # Load kernel sp into sp
    ld sp, 35*8(sp)
    # Switch to kernel page table
    csrw satp, t0
    sfence.vma
    # Jump to trap_handler
    jr t1

    .globl __restore
__restore:
    # a0: *TrapContext in user space (virtual address)
    # a1: user space token (satp value)
    
    # Save our arguments first
    mv s0, a0
    mv s1, a1
    
    # Switch to user page table
    csrw satp, s1
    sfence.vma
    
    # Restore sscratch and sp
    csrw sscratch, s0
    mv sp, s0
    
    # Restore sstatus and sepc from trap context
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    
    # Restore general purpose registers except sp and x0
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    ld x4, 4*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    
    # Restore sp last (x2)
    ld x2, 2*8(sp)
    
    # Ensure all memory operations complete
    fence
    fence.i
    
    sret
