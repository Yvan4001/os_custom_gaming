ENTRY(_start)

SECTIONS {
    /* Mark the beginning of the kernel image */
    __kernel_physical_start = .;

    .multiboot_header : {
        KEEP(*(.multiboot_header))
    }
    
    . = ALIGN(4096);
    
    .text : {
        *(.text._start)  /* Force _start to be first */
        *(.text*)
    }
    
    .rodata : {
        *(.rodata*)
    }
    
    .data : {
        *(.data*)
    }
    
    .bss : {
        *(.bss*)
        *(.bss.stack)
        *(COMMON)
    }
    
    /* Mark the end of the kernel image */
    __kernel_physical_end = .;
    
    /DISCARD/ : {
        *(.eh_frame*)
        *(.debug*)
        *(.note*)
        *(.comment*)
    }
}