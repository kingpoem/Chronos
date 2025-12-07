OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
    /* QEMU virt machine: 128MB RAM starting at 0x80000000 */
    RAM : ORIGIN = 0x80000000, LENGTH = 128M
    /* DTB is typically placed at 0x82000000 */
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
    . = 0x80000000;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    } >RAM AT>RAM :text

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } >RAM AT>RAM :rodata

    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } >RAM AT>RAM :data

    .bss : {
        _bss_start = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        _bss_end = .;
    } >RAM AT>RAM :bss

    /* Stack grows downward from 0x80020000 */
    _stack_top = 0x80020000;
    _stack_bottom = _stack_top - 0x10000; /* 64KB stack */

    /* Ensure DTB region is not overwritten */
    . = ALIGN(4K);
    _end = .;
}

