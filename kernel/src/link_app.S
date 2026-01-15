    .section .data
    .align 3
    .global _num_app
_num_app:
    .quad 1
    .quad app_0_start, app_0_end

    .global _app_names
_app_names:
    .string "01hello"

    .align 3
    .global app_0_start
    .global app_0_end
app_0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/01hello"
app_0_end:

