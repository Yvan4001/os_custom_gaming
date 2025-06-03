ENTRY(_start)

SECTIONS {
    . = 0x101000;
    __kernel_physical_start = .;
    
    .multiboot_header ALIGN(8) : {
        KEEP(*(.multiboot_header))
    }
    
    .text.boot ALIGN(8) : {
        *(.text.boot)
    }
    
    /* Rest of code follows */
    .text : {
        *(.text .text.*)
    }
    
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
    
    /* Mark the end of the kernel image */
    __kernel_physical_end = .;
    
    /DISCARD/ : {
        *(.eh_frame*)
        *(.debug*)
        *(.note*)
        *(.comment*)
    }
}