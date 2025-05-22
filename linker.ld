/* linker.ld - Linker script for FluxGridOS */
OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

PHDRS {
    text PT_LOAD FLAGS(5); /* R, X */
    rodata PT_LOAD FLAGS(4); /* R */
    data PT_LOAD FLAGS(6); /* R, W */
    bss PT_LOAD FLAGS(6); /* R, W */
}

SECTIONS {
    /* Kernel starts at 1MB physical address */
    . = 1M;
    .multiboot_header : ALIGN(4K) {
    KEEP(*(.multiboot_header))
    }

    /* Define __kernel_physical_start at the beginning of the .text section */
    PROVIDE(__kernel_physical_start = .);

    /* MODIFIED: Removed extra colon after ALIGN(4K) */
    .text : ALIGN(4K) {
        *(.text._start) /* Ensure _start comes first if it's in its own section */
        *(.text .text.*)
    } :text

    /* MODIFIED: Removed extra colon after ALIGN(4K) */
    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    } :rodata

    /* MODIFIED: Removed extra colon after ALIGN(4K) */
    .data : ALIGN(4K) {
        *(.data .data.*)
    } :data

    /* MODIFIED: Removed extra colon after ALIGN(4K) */
    .bss : ALIGN(4K) {
        *(.bss .bss.*)
        *(COMMON)
    } :bss

    /* Define __kernel_physical_end after the .bss section */
    /* This symbol will mark the end of the statically allocated kernel image. */
    PROVIDE(__kernel_physical_end = .);

    /* Discard notes and comments to reduce binary size */
    /DISCARD/ : {
        *(.comment)
        *(.note*)
    }
}
