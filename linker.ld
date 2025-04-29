OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

SECTIONS {
    . = 1M;

    /* Section .boot supprimée */

    .text : ALIGN(4K) :
    {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) :
    {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) :
    {
        *(.data .data.*)
    }

    .bss : ALIGN(4K) :
    {
        *(.bss .bss.*)
    }
}