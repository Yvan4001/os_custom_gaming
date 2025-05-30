ENTRY(_start)

SECTIONS {
    . = 1M;
    
    .multiboot_header : {
        KEEP(*(.multiboot_header))
    }
    
    . = ALIGN(4096);
    
    .text : {
        *(.text._start)  /* Force _start first */
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
        *(COMMON)
    }
    
    /DISCARD/ : {
        *(.eh_frame*)
        *(.debug*)
        *(.note*)
        *(.comment*)
    }
}