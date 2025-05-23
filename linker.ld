/* linker.ld - Linker script for FluxGridOS */
OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

PHDRS {
    /* Program header for the Multiboot2 header itself. Read-only. */
    multiboot_hdr_segment PT_LOAD FLAGS(4);  /* R */

    /* Program header for executable code. Read-only, Executable. */
    text_segment PT_LOAD FLAGS(5);           /* R, X */

    /* Program header for read-only data. */
    rodata_segment PT_LOAD FLAGS(4);          /* R */

    /* Program header for read-write data and BSS. */
    data_segment PT_LOAD FLAGS(6);            /* R, W */
}

SECTIONS {
    /* Kernel's load address in physical memory. */
    . = 1M;

    /* Section for the Multiboot2 header.
       It must be 8-byte aligned and within the first part of the image.
       It gets its own small program header. */
    .multiboot_header : ALIGN(8) {
        KEEP(*(.multiboot_header))
    } :multiboot_hdr_segment /* Assign to its own PHDR */

    /* The .text section follows the header.
       __kernel_physical_start now marks the beginning of the actual kernel code/data,
       which comes *after* the Multiboot2 header section. */
    .text : ALIGN(4K) {
        PROVIDE(__kernel_physical_start = .); /* Start of actual kernel code/data sections */
        *(.text._start)         /* Ensure _start symbol from main.rs is first */
        *(.text .text.*)        /* All other text sections */
    } :text_segment             /* Assign to the text_segment PHDR */

    /* Read-only data section. */
    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    } :rodata_segment           /* Assign to the rodata_segment PHDR */

    /* Read-write data section. */
    .data : ALIGN(4K) {
        *(.data .data.*)
    } :data_segment             /* Assign to the data_segment PHDR */

    /* BSS section (uninitialized data).
       It occupies space in memory but not in the file.
       It's part of the read-write data segment in memory. */
    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(COMMON)
    } :data_segment             /* Also part of the data_segment PHDR */
                                /* The data_segment PHDR will have a p_filesz covering .data
                                   and a p_memsz covering .data + .bss */

    /* Define __kernel_physical_end after all loaded sections. */
    PROVIDE(__kernel_physical_end = .);

    /* Discard unnecessary sections to reduce binary size. */
    /DISCARD/ : {
        *(.comment)
        *(.note*)
    }
}
