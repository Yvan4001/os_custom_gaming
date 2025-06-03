ENTRY(_start)

SECTIONS {
    /* Start loading at 1MB - standard for most kernels */
    . = 1M;
    
    /* Define the kernel physical start symbol at the beginning */
    __kernel_physical_start = .;
    
    /* The multiboot header MUST come first */
    .multiboot_header ALIGN(8) : {
        KEEP(*(.multiboot_header))
    }
    
    .text ALIGN(4K) : {
        *(.text.boot)   /* Boot code first */
        *(.text .text.*)
    }
    
    /* Rest of sections... */
    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    }
    
    .data ALIGN(4K) : {
        *(.data .data.*)
    }
    
    .bss ALIGN(4K) : {
        *(.bss.stack)
        *(.bss .bss.*)
        *(COMMON)
    }
    
    /* Define the kernel physical end symbol after all sections */
    __kernel_physical_end = .;
    
    /* Discard unnecessary sections to make the kernel file smaller */
    /DISCARD/ : {
        *(.eh_frame*)
        *(.comment)
    }
}