; boot.asm - Minimal Bootloader for Long Mode Transition
; This bootloader transitions from 16-bit real mode to 64-bit long mode.
; It sets up a GDT and basic page tables for identity mapping.

BITS 16             ; Start in 16-bit real mode
ORG 0x7C00          ; BIOS loads bootloader here

ENTRY_POINT:
    cli             ; Disable interrupts
    xor ax, ax      ; AX = 0
    mov ds, ax      ; Set DS to 0
    mov es, ax      ; Set ES to 0
    mov ss, ax      ; Set SS to 0
    mov sp, 0x7C00  ; Stack pointer grows downwards from 0x7C00

    ; Check for CPUID support
    pushfd
    pop ax
    mov cx, ax
    xor ax, 0x8000      ; Try to flip VIF/VIP bits (CPUID support check)
    push ax
    popfd
    pushfd
    pop ax
    push cx
    popfd
    cmp ax, cx
    je .no_cpuid        ; If no change, CPUID not supported

    ; Check for Long Mode support (CPUID.80000001h:EDX[29])
    mov eax, 0x80000000 ; Get highest extended function
    cpuid
    cmp eax, 0x80000001 ; Check if extended function 1 is supported
    jb .no_long_mode    ; If not, no long mode

    mov eax, 0x80000001 ; Extended function 1
    cpuid
    test edx, 1 << 29   ; Test LM bit (bit 29)
    jz .no_long_mode    ; If not set, no long mode

    ; Long mode is supported, proceed with setup

    lgdt [gdt_descriptor] ; Load GDT

    ; Enable PAE (Physical Address Extension) in CR4
    mov eax, cr4
    or eax, 1 << 5      ; Set PAE bit
    mov cr4, eax

    ; Set up page tables (PML4, PDPT, PD)
    ; We will identity map the first 4MB using 2MB pages
    ; Tables are placed right after this code
    mov edi, PAGE_TABLE_BASE ; Base address for page tables
    
    ; PML4 Table (first entry points to PDPT)
    ; PML4[0] = address of PDPT | Present | Read/Write
    mov dword [edi], PAGE_TABLE_BASE + 0x1000 ; PDPT physical address
    or dword [edi], 0x003 ; Present, Read/Write
    add edi, 0x1000 ; Move to PDPT base

    ; PDPT Table (first entry points to Page Directory)
    ; PDPT[0] = address of Page Directory 0 | Present | Read/Write
    mov dword [edi], PAGE_TABLE_BASE + 0x2000 ; Page Directory 0 physical address
    or dword [edi], 0x003 ; Present, Read/Write
    add edi, 0x1000 ; Move to Page Directory 0 base

    ; Page Directory 0 (maps first 4MB using two 2MB pages)
    ; PD[0] = 0 (maps 0-2MB) | Present | Read/Write | Page Size (2MB)
    mov dword [edi], 0x00000000 | 0x083 ; Physical address 0, Present, R/W, Page Size
    ; PD[1] = 0x200000 (maps 2MB-4MB) | Present | Read/Write | Page Size (2MB)
    mov dword [edi + 8], 0x00200000 | 0x083 ; Physical address 0x200000, Present, R/W, Page Size

    ; Load PML4 address into CR3
    mov eax, PAGE_TABLE_BASE
    mov cr3, eax

    ; Enable Long Mode (set LME bit in MSR_EFER)
    mov ecx, 0xC0000080 ; EFER MSR
    rdmsr               ; Read EFER into EDX:EAX
    or eax, 1 << 8      ; Set LME bit (bit 8)
    wrmsr               ; Write back to EFER

    ; Enable Paging (set PG bit in CR0)
    mov eax, cr0
    or eax, 1 << 31     ; Set PG bit
    or eax, 1 << 0      ; Set PE bit (Protected Mode) - must be set before or with PG
    mov cr0, eax

    ; Far jump to 64-bit code segment
    ; This will load CS with the 64-bit code segment selector from GDT
    ; and transition to 64-bit mode.
    jmp CODE_SEG:start_64bit

.no_cpuid:
    mov si, msg_no_cpuid
    call print_string_bios
    jmp halt_system

.no_long_mode:
    mov si, msg_no_long_mode
    call print_string_bios
    jmp halt_system

; Simple string printing routine using BIOS (for error messages in 16-bit mode)
print_string_bios:
    mov ah, 0x0E        ; BIOS teletype output
    mov bh, 0x00        ; Page number
    mov bl, 0x07        ; White on black
.next_char:
    lodsb               ; Load byte from [DS:SI] into AL, increment SI
    or al, al           ; Check if AL is zero (end of string)
    jz .done
    int 0x10            ; Print character
    jmp .next_char
.done:
    ret

halt_system:
    cli
.hang_loop:
    hlt
    jmp .hang_loop

; --- GDT (Global Descriptor Table) ---
gdt_start:
    ; Null Descriptor (required)
    dq 0x0000000000000000

    ; 64-bit Code Segment Descriptor (CS)
    ; Base=0, Limit=0xFFFFF (4GB, ignored in long mode),
    ; Flags: Present, Ring 0, Code, Executable, Readable
    ; G (Granularity)=1 (limit in 4KB units), L (Long Mode)=1, D (Default Operand Size)=0 (for 64-bit)
    dq 0x00209A0000000000 ; P=1, DPL=00, S=1, Type=1010 (Execute/Read Code), G=1, L=1

    ; 64-bit Data Segment Descriptor (DS, ES, SS, etc.)
    ; Base=0, Limit=0xFFFFF (4GB, ignored in long mode),
    ; Flags: Present, Ring 0, Data, Writable
    ; G=1, L=0 (data segments don't use L bit), D/B=1 (32-bit stack, but ignored in long mode for DS/ES)
    dq 0x00C0920000000000 ; P=1, DPL=00, S=1, Type=0010 (Read/Write Data), G=1, DB=1

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1 ; GDT Limit (size - 1)
    dd gdt_start               ; GDT Base Address (linear address)

CODE_SEG equ gdt_start + 8 - gdt_start ; Offset of code segment (0x08)
DATA_SEG equ gdt_start + 16 - gdt_start ; Offset of data segment (0x10)

; --- Messages for 16-bit mode ---
msg_no_cpuid: db "CPUID not supported. Halting.", 0
msg_no_long_mode: db "Long Mode not supported. Halting.", 0

; --- Page Table Area ---
; These tables must be page-aligned (4KB).
; We place them immediately after the bootloader code.
; The ORG 0x7C00 means $$ starts at 0x7C00.
; We need to ensure PAGE_TABLE_BASE is a physical address.
; Since we are loaded at 0x7C00, and this code is small,
; placing tables at 0x8000 (next 4K boundary after 0x7C00 if code < 1KB)
; or higher is common. For simplicity, let's assume our code + GDT < 1KB.
; A more robust bootloader would calculate this.
; Let's place them starting at a known physical address, e.g., 0x10000 (64KB)
; This bootloader itself will be running from 0x7C00.
PAGE_TABLE_BASE equ 0x10000 ; Physical address for PML4
; PML4 table: 0x10000 - 0x10FFF
; PDPT table: 0x11000 - 0x11FFF
; PD0 table:  0x12000 - 0x12FFF

; --- 64-bit Code Section ---
BITS 64
start_64bit:
    ; We are now in 64-bit Long Mode
    ; CS is already set by the far jump
    ; Set up data segments (DS, ES, SS)
    mov ax, DATA_SEG    ; Load 64-bit data segment selector
    mov ds, ax
    mov es, ax
    mov ss, ax
    ; GS and FS can be zeroed or set up for specific purposes later
    xor rax, rax
    mov fs, ax
    mov gs, ax

    ; At this point, the first 4MB of physical memory is identity mapped.
    ; You can now, for example, clear the screen or print via direct framebuffer access if mapped.
    ; For this minimal example, just halt.
    ; A real bootloader would now load the kernel.

    ; Example: Write to VGA memory (assuming it's at 0xB8000 and identity mapped)
    ; This requires 0xB8000 to be within the mapped 4MB.
    ; mov edi, 0xB8000
    ; mov rax, 0x074B074F ; 'O' 'K' in white on black
    ; mov [edi], rax

final_hang:
    hlt
    jmp final_hang

; --- Padding and Boot Signature ---
; Ensure the total size is 512 bytes.
; The 'times' directive fills the remaining space up to 510 bytes with 0.
; The last two bytes are the boot signature.
    times 510 - ($-$$) db 0  ; Pad with 0s up to 510th byte
    dw 0xAA55                ; Boot signature
