; boot_header.s - Defines the Multiboot2 Header for the kernel ELF
; This file should be assembled and linked with your Rust kernel objects.

section .multiboot_header
align 8 ; Multiboot2 header must be 8-byte aligned

header_start:
    dd 0xe85250d6                ; Multiboot2 magic number (MB2_HEADER_MAGIC)
    dd 0                         ; Architecture: 0 for i386/x86_64
    dd header_end - header_start ; Header length
    ; Checksum: -(magic + architecture + header_length)
    dd -(0xe85250d6 + 0 + (header_end - header_start))

    ; End Tag (required)
    dw 0    ; Type: End tag
    dw 0    ; Flags
    dd 8    ; Size of end tag (type + flags + size = 2+2+4 = 8 bytes)
header_end:

; You could add other optional tags here if needed, e.g., for framebuffer request.
; Ensure they are correctly formatted and update header_length and checksum.
