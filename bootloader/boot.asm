; boot.asm - Stage 1 (MBR) loads Stage 2.
; Stage 2 is currently minimal (transitions to long mode and halts or jumps to kernel).

BITS 16
ORG 0x7C00

STAGE2_LOAD_SEGMENT equ 0x0800  ; Load Stage 2 to 0x0800:0000 (physical 0x8000)
STAGE2_LOAD_OFFSET  equ 0x0000
STAGE2_START_SECTOR equ 1       ; LBA of first sector of Stage 2 (sector after MBR)
STAGE2_SECTORS_TO_LOAD equ 1    ; Load 1 sector (512 bytes) for Stage 2.
                                ; Ensure Stage 2 code + padding fits this.

KERNEL_LOAD_PHYS_ADDR equ 0x100000 ; Where Stage 2 might load the actual kernel
KERNEL_ENTRY_POINT  equ KERNEL_LOAD_PHYS_ADDR
MBI_PHYSICAL_ADDRESS equ 0x9000    ; Where Stage 2 might build MBI
PAGE_TABLE_BASE equ 0x1000         ; For Stage 2's paging setup
KERNEL_SECTORS equ 100
KERNEL_START_SECTOR equ 2

MBR_EntryPoint:
    cli
    ; initial segment and stack setup for MBR
    xor ax, ax      ; AX = 0
    mov ds, ax      ; DS = 0
    mov es, ax      ; ES = 0
    mov ss, ax      ; SS = 0
    mov sp, 0x7BE0  ; Stack pointer slightly below 0x7C00

    push dx

    ; Save boot drive from BIOS
    mov [boot_drive], dl

    mov ah, 0x0E; mov al, '1'; mov bh, 0x00; mov bl, 0x0F; int 0x10 ; Print '1'

    ; Skip disk reset entirely - go straight to loading
    mov ah, 0x0E; mov al, 'L'; mov bh, 0x00; mov bl, 0x0B; int 0x10 ; Print 'L' for Load

    ; Setup for int 0x13, ah=0x02 (Read Sectors)
    mov ah, 0x02
    mov al, STAGE2_SECTORS_TO_LOAD    ; Number of sectors to read
    mov ch, 0                         ; Cylinder 0
    mov cl, 2                         ; Sector 2 (sectors start at 1, so sector 2 = LBA 1)
    mov dh, 0                         ; Head 0
    mov dl, [boot_drive]              ; Drive number from BIOS
    
    mov bx, STAGE2_LOAD_SEGMENT
    mov es, bx                        ; ES = 0x0800
    mov bx, STAGE2_LOAD_OFFSET        ; BX = 0x0000 (so ES:BX = 0x0800:0000)

    int 0x13 ; Attempt to load Stage 2

    jc .disk_error_stage2_load        ; Jump if carry flag set (error)

    ; Success - print 'J' and jump to Stage 2
    mov ah, 0x0E; mov al, 'J'; mov bh, 0; mov bl, 0x0B; int 0x10
    jmp STAGE2_LOAD_SEGMENT:STAGE2_LOAD_OFFSET

.disk_error_stage2_load:
    mov ah, 0x0E; mov al, 'E'; mov bh, 0x00; mov bl, 0x4F; int 0x10 ; Print 'E' for Error
    ; AH contains the error code from int 0x13
    push ax         ; Save the error code
    mov al, ah      ; Move error code to AL for printing
    call PrintHexByte_MBR ; Print the error code as hex
    pop ax          ; Restore original AX
    jmp .halt_mbr

.halt_mbr:
    cli; hlt; jmp .halt_mbr

; Add storage for boot drive
boot_drive: db 0

; Helper to print byte in AL as two hex characters (for MBR)
PrintHexByte_MBR:
    push ax
    push bx
    push cx
    mov bh, 0x0E    ; BIOS teletype  
    mov bl, 0x4F    ; Error color (White on Red)
    mov ch, al      ; Save al
    shr al, 4       ; Get upper nibble
    call .PrintNibble_MBR
    mov al, ch      ; Restore al
    and al, 0x0F    ; Get lower nibble
    call .PrintNibble_MBR
    pop cx
    pop bx
    pop ax
    ret
.PrintNibble_MBR:
    cmp al, 9
    jle .IsDigit_MBR
    add al, 'A' - 10
    jmp .DoPrint_MBR
.IsDigit_MBR:
    add al, '0'
.DoPrint_MBR:
    mov ah, 0x0E
    int 0x10
    ret

    times 510 - ($ - MBR_EntryPoint) db 0
    dw 0xAA55

; =============================================================================
; Stage 2 - Minimal (Loaded by MBR to physical 0x8000)
; =============================================================================
CODE_SEG_32_S2 equ 0x08 ; Renamed to avoid conflict with MBR defines if any
DATA_SEG_32_S2 equ 0x10
CODE_SEG_64_S2 equ 0x08
DATA_SEG_64_S2 equ 0x10

Stage2_EntryPoint:
    mov ah, 0x0E
    mov al, 'S'
    mov bh, 0
    mov bl, 0x0A
    int 0x10

    ; Add debug print
    mov ah, 0x0E
    mov al, '2'
    int 0x10

    cli
    mov ax, cs
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7000  ; Safer stack location

    ; Add another debug print
    mov ah, 0x0E
    mov al, '3'
    int 0x10

    mov si, msg_stage2_active
    call print_string_stage2

    ; Add debug print after string
    mov ah, 0x0E
    mov al, '4'
    int 0x10

    jmp proceed_to_mode_switch_s2

print_string_stage2:
    pusha
    mov ah, 0x0E
    mov bh, 0x00
    mov bl, 0x0A
.loop_print_s2:
    lodsb
    or al, al
    jz .done_print_s2
    int 0x10
    jmp .loop_print_s2
.done_print_s2:
    popa
    ret

msg_stage2_active : db "Stage 2 Active", 0Dh, 0Ah, 0
msg_no_cpuid_s2: db "No CPUID", 0Dh, 0Ah, 0
msg_no_long_mode_s2: db "No Long Mode", 0Dh, 0Ah, 0

proceed_to_mode_switch_s2:
    ; Add debug print before GDT load
    mov ah, 0x0E
    mov al, '5'
    int 0x10

    lgdt [gdt_descriptor_stage2]

    ; Add debug print after GDT load  
    mov ah, 0x0E
    mov al, '6'
    int 0x10

    ; Print success message BEFORE switching to protected mode
    mov ah, 0x0E
    mov al, 'P'    ; P for Protected mode about to start
    mov bl, 0x0E   ; Yellow
    int 0x10

    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp CODE_SEG_32_S2:protected_mode_entry_s2

gdt_start_s2: 
    dq 0                                    ; Null descriptor
    dw 0xFFFF, 0x0000                       ; Code segment: limit 0-15, base 0-15
    db 0x00, 0b10011010, 0b11001111, 0x00   ; Code segment: base 16-23, access, flags+limit 16-19, base 24-31
    dw 0xFFFF, 0x0000                       ; Data segment: limit 0-15, base 0-15  
    db 0x00, 0b10010010, 0b11001111, 0x00   ; Data segment: base 16-23, access, flags+limit 16-19, base 24-31
gdt_end_s2:

gdt_descriptor_stage2: 
    dw gdt_end_s2 - gdt_start_s2 - 1        ; GDT size
    dd gdt_start_s2                          ; GDT address

times 512 - ($ - Stage2_EntryPoint) db 0


BITS 32
protected_mode_entry_s2:
    mov ax, DATA_SEG_32_S2
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax ; PAE

    ; Print success before going to long mode
    mov byte [0xB8000], 'O'      ; Print 'O' at top-left of screen
    mov byte [0xB8001], 0x0A     ; Green on black
    mov byte [0xB8002], 'K'      ; Print 'K'
    mov byte [0xB8003], 0x0A     ; Green on black

    mov edi, PAGE_TABLE_BASE
    xor eax, eax; mov ecx, 3 * 512 * 2; rep stosd ; Clear page tables

    mov edi, PAGE_TABLE_BASE ; PML4
    mov dword [edi], PAGE_TABLE_BASE + 0x1000; or dword [edi], 0x003
    mov dword [edi + 256 * 8], PAGE_TABLE_BASE + 0x1000; or dword [edi + 256 * 8], 0x003

    mov edi, PAGE_TABLE_BASE + 0x1000 ; PDPT
    mov dword [edi], PAGE_TABLE_BASE + 0x2000; or dword [edi], 0x003

    mov edi, PAGE_TABLE_BASE + 0x2000 ; PD0
    mov dword [edi],     0x000000 | 0x083
    mov dword [edi + 8], 0x200000 | 0x083

    mov eax, PAGE_TABLE_BASE; mov cr3, eax
    mov ecx, 0xC0000080; rdmsr; or eax, 1 << 8; wrmsr ; LME
    mov eax, cr0; or eax, (1 << 31); mov cr0, eax ; PG

    push ds; xor ax, ax; mov ds, ax; mov es, ax
    
    mov edi, MBI_PHYSICAL_ADDRESS
    mov dword [es:edi + 0], 80; mov dword [es:edi + 4], 0
    mov dword [es:edi + 8], 6; mov dword [es:edi + 12], (16 + 2 * 24)
    mov dword [es:edi + 16], 24; mov dword [es:edi + 20], 0
    mov dword [es:edi + 24], 0x0; mov dword [es:edi + 28], 0x0
    mov dword [es:edi + 32], 0x9FC00; mov dword [es:edi + 36], 0x0
    mov dword [es:edi + 40], 1; mov dword [es:edi + 44], 0
    mov dword [es:edi + 48], KERNEL_LOAD_PHYS_ADDR; mov dword [es:edi + 52], 0x0
    mov dword [es:edi + 56], 0x01000000; mov dword [es:edi + 60], 0x0
    mov dword [es:edi + 64], 1; mov dword [es:edi + 68], 0
    mov word [es:edi + 72], 0; mov word [es:edi + 74], 0; mov dword [es:edi + 76], 8
    pop ds

    mov eax, 0x36d76289; mov ebx, MBI_PHYSICAL_ADDRESS
    jmp CODE_SEG_64_S2:start_64bit_s2

.no_cpuid_s2: 
    mov si, msg_no_cpuid_s2
    call print_string_stage2
    ; Print error code for no CPUID (custom code 0xC1)
    mov al, 0xC1
    call print_hex_byte_stage2
    jmp .halt_stage2_final

.no_long_mode_s2: 
    mov si, msg_no_long_mode_s2
    call print_string_stage2
    ; Print error code for no long mode (custom code 0xC2)
    mov al, 0xC2
    call print_hex_byte_stage2
    jmp .halt_stage2_final

.halt_stage2_final: 
    cli; hlt; jmp .halt_stage2_final

print_hex_byte_stage2:
    push ax
    push bx
    push cx
    mov bh, 0x00    ; Video page
    mov bl, 0x4F    ; Error color (White on Red)
    mov ch, al      ; Save original value
    
    ; Print upper nibble
    shr al, 4
    call .print_nibble_s2
    
    ; Print lower nibble
    mov al, ch
    and al, 0x0F
    call .print_nibble_s2
    
    pop cx
    pop bx
    pop ax
    ret

.print_nibble_s2:
    cmp al, 9
    jle .is_digit_s2
    add al, 'A' - 10
    jmp .do_print_s2
.is_digit_s2:
    add al, '0'
.do_print_s2:
    mov ah, 0x0E
    int 0x10
    ret

print_string_stage2_and_halt_s2: call print_string_stage2; jmp .halt_stage2_final

.halt_stage2_final: cli; hlt; jmp .halt_stage2_final

msg_stage2_booting: db "S2 Boot...", 0Dh, 0Ah, 0


BITS 64
start_64bit_s2:
    mov ax, DATA_SEG_64_S2
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov rsp, 0x70000
    
    ; Jump directly to Rust kernel 
    jmp 0x101000  ; Your kernel's entry point
