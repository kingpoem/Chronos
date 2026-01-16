OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
    /* QEMU virt machine: 128MB RAM starting at 0x80000000 */
    /* Bootloader is loaded at 0x80200000 by SBI */
    RAM : ORIGIN = 0x80200000, LENGTH = 128K
}

PHDRS
{
    text PT_LOAD;
    rodata PT_LOAD;
    data PT_LOAD;
    bss PT_LOAD;
}

SECTIONS
{
    . = 0x80200000;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    } >RAM AT>RAM :text

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } >RAM AT>RAM :rodata

    .data : {
        . = ALIGN(8);
        __global_pointer$ = . + 0x800;
        *(.data .data.*)
        *(.sdata .sdata.*)
    } >RAM AT>RAM :data

    .bss : {
        sbss = .;
        _bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        ebss = .;
        _bss_end = .;
    } >RAM AT>RAM :bss

    . = ALIGN(4K);
    .stack (NOLOAD) : {
        *(.stack)
    } >RAM AT>RAM :bss

    /* Stack symbols are provided by entry.S (.bss.stack) */
    _stack_bottom = boot_stack_lower_bound;
    _stack_top = boot_stack_top;

    /* Ensure DTB region is not overwritten */
    . = ALIGN(4K);
    _end = .;
}

