    .section .data
    .align 3
    .global _num_app
_num_app:
    .quad 3
    .quad app_0_start
    .quad app_1_start
    .quad app_2_start
    .quad app_2_end

    .global _app_names
_app_names:
    .string "power_3"
    .string "power_5"
    .string "power_7"

    .align 3
    .global app_0_start
app_0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/power_3"

    .align 3
    .global app_1_start
app_1_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/power_5"

    .align 3
    .global app_2_start
app_2_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/power_7"

    .global app_2_end
app_2_end:

