    .section .text.entry
    .globl _start
_start:
    # a0 = hartid
    # a1 = dtb
    
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    .option push
    .option norelax
    la sp, boot_stack_top
    .option pop
    
    # Save initial register state before calling kernel_main
    # We'll print these in kernel_main
    # sp is already set to boot_stack_top
    # Save sp to a temporary register for later use
    mv t0, sp
    call kernel_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:
