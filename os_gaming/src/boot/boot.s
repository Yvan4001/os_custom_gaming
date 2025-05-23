.section .multiboot_header
.align 8

# Multiboot2 header
header_start:
    .long 0xe85250d6                # Multiboot2 magic
    .long 0                         # Architecture (i386)
    .long (header_end - header_start) # Header length
    # Checksum
    .long -(0xe85250d6 + 0 + (header_end - header_start))
    # End tag
    .short 0                        # Type
    .short 0                        # Flags
    .long 8                         # Size
header_end:

.section .text
.global _start
.code32                            # Start in 32-bit mode

_start:
    # Save multiboot info immediately
    movl %eax, multiboot_magic
    movl %ebx, multiboot_ptr
    
    # Call Rust code
    jmp _rust_start

.section .data
.global multiboot_magic
.global multiboot_ptr
multiboot_magic: .long 0
multiboot_ptr: .long 0