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
    call kernel_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:
