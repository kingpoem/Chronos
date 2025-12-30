# Context switch
#
# void __switch(
#     TaskContext *current_task_cx_ptr,
#     TaskContext *next_task_cx_ptr
# )

    .section .text
    .globl __switch
__switch:
    # Save current task context
    # __switch(current_task_cx_ptr, next_task_cx_ptr)
    # a0 = current_task_cx_ptr
    # a1 = next_task_cx_ptr
    # Save current context to a0
    sd ra, 0*8(a0)
    sd sp, 1*8(a0)
    sd s0, 2*8(a0)
    sd s1, 3*8(a0)
    sd s2, 4*8(a0)
    sd s3, 5*8(a0)
    sd s4, 6*8(a0)
    sd s5, 7*8(a0)
    sd s6, 8*8(a0)
    sd s7, 9*8(a0)
    sd s8, 10*8(a0)
    sd s9, 11*8(a0)
    sd s10, 12*8(a0)
    sd s11, 13*8(a0)
    # Restore next context from a1
    ld ra, 0*8(a1)
    ld sp, 1*8(a1)
    ld s0, 2*8(a1)
    ld s1, 3*8(a1)
    ld s2, 4*8(a1)
    ld s3, 5*8(a1)
    ld s4, 6*8(a1)
    ld s5, 7*8(a1)
    ld s6, 8*8(a1)
    ld s7, 9*8(a1)
    ld s8, 10*8(a1)
    ld s9, 11*8(a1)
    ld s10, 12*8(a1)
    ld s11, 13*8(a1)
    ret
