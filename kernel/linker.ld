OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        . = ALIGN(8);
        __global_pointer$ = . + 0x800;
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    sbss = .;
    .bss : {
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    
    .bss.stack : {
        *(.bss.stack)
    }
    . = ALIGN(4K);
    ekernel = .;

    .data.apps : ALIGN(4K) {
        *(.data.apps)
        *(.data.app_*)
    }

    /DISCARD/ : {
        *(.eh_frame)
    }
}
