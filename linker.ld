/* linker.ld - A standard linker script for a 64-bit kernel */
OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

PHDRS {
    /* Defines the loadable segments of the kernel ELF */
    text_and_header PT_LOAD FLAGS(5);  /* R, X - For Multiboot header and code */
    rodata PT_LOAD FLAGS(4);          /* R      - For read-only data */
    data_and_bss PT_LOAD FLAGS(6);    /* R, W   - For read-write data and BSS */
}

SECTIONS {
    /* MODIFIED: Kernel's load address set to the standard 1M. */
    . = 1M;

    /* Start of kernel's virtual address space, for calculating offsets */
    PROVIDE(__kernel_physical_start = .);

    /* The .text section will contain the Multiboot2 header and all code. */
    .text : ALIGN(4K) {
        /* Place the Multiboot2 header first, ensuring 8-byte alignment. */
        KEEP(*(.multiboot_header))

        /* Place the _start symbol immediately after. */
        *(.text._start)
        *(.text .text.*)
    } :text_and_header

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    } :rodata

    .data : ALIGN(4K) {
        *(.data .data.*)
    } :data_and_bss

    /* .bss section for zero-initialized data, including the stack. */
    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(.bss.stack) /* Section for our bootstrap stack */
        *(COMMON)
    } :data_and_bss

    /* Define kernel end after all loaded sections. */
    PROVIDE(__kernel_physical_end = .);

    /* Discard unnecessary sections, but KEEP debug info. */
    /DISCARD/ : {
        *(.comment)
        *(.note*)
        /* We DO NOT discard .debug_* or .eh_frame, so GDB can work */
    }
}
