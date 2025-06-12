; stage2.asm - Loaded by an MBR to physical 0x8000.
; Transitions from 16-bit real mode to 64-bit long mode.

BITS 16

; --- Constants and Selectors ---
PAGE_TABLE_BASE equ 0x1000 ; Place page tables at physical 4KB
KERNEL_ENTRY equ 0x100000  ; The physical address where the Rust kernel is loaded

CODE_SEG_SEL equ 0x08       ; Selector for our 32-bit/64-bit code segment
DATA_SEG_SEL equ 0x10       ; Selector for our 32-bit/64-bit data segment

stage2_start:
    ; We are in 16-bit real mode when Stage 1 jumps here.
    ; Set up segments and a safe stack.
    mov ax, cs
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0xFFFE ; Stack at the top of our current 64KB segment

    ; Print Stage 2 start message
    mov si, msg_stage2_start
    call print_string_16bit

    ; --- Check for Long Mode support ---
    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29
    jz no_long_mode

    ; --- Load GDT and transition to 32-bit Protected Mode ---
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1               ; Set PE bit to enter Protected Mode
    mov cr0, eax
    jmp CODE_SEG_SEL:protected_mode_entry

BITS 32
protected_mode_entry:
    ; Now in 32-bit Protected Mode
    mov ax, DATA_SEG_SEL
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x90000 ; Set up a safe stack pointer for 32-bit operations

    ; --- Setup Page Tables for Long Mode ---
    ; Clear page tables (PML4, PDPT, PD)
    mov edi, PAGE_TABLE_BASE
    xor eax, eax
    mov ecx, 4096 * 3 ; Zero out 12KB
    rep stosb

    ; PML4[0] points to PDPT (for identity mapping low memory)
    mov edi, PAGE_TABLE_BASE
    mov dword [edi], (PAGE_TABLE_BASE + 0x1000) | 0x003 ; Present, R/W
    
    ; PDPT[0] points to a Page Directory
    mov edi, PAGE_TABLE_BASE + 0x1000
    mov dword [edi], (PAGE_TABLE_BASE + 0x2000) | 0x003 ; Present, R/W

    ; Page Directory: Map first 2MB using one 2MB page
    mov edi, PAGE_TABLE_BASE + 0x2000
    mov dword [edi], 0x000000 | 0x083 ; Address=0, Flags: Present, R/W, PageSize

    ; --- Enable Long Mode ---
    ; 1. Enable PAE in CR4
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; 2. Load PML4 address into CR3 (CRITICAL STEP)
    mov eax, PAGE_TABLE_BASE
    mov cr3, eax

    ; 3. Enable Long Mode (LME) in EFER MSR
    mov ecx, 0xC0000080 ; EFER MSR
    rdmsr
    or eax, 1 << 8     ; Set LME bit
    wrmsr

    ; 4. Enable Paging
    mov eax, cr0
    or eax, 1 << 31    ; Set PG bit (PE is already set)
    mov cr0, eax

    ; 5. Far jump to 64-bit code segment
    jmp CODE_SEG_SEL:long_mode_entry ; We can reuse the same code selector

BITS 64
long_mode_entry:
    ; We are now in 64-bit Long Mode
    ; Set up data segments
    mov ax, DATA_SEG_SEL
    mov ds, ax
    mov es, ax
    mov ss, ax
    xor ax, ax ; Not strictly necessary, but good practice
    mov fs, ax
    mov gs, ax

    ; Print a success message directly to VGA memory
    mov rdi, 0xB8000 + (160 * 5) ; Line 6 on screen
    mov rsi, msg_lm
    call print_string_lm_64bit

    ; Here you would set EAX/EBX and jump to your Rust kernel
    ; mov eax, 0x36d76289
    ; mov ebx, MBI_PHYSICAL_ADDRESS
    ; jmp KERNEL_ENTRY

halt_loop:
    hlt
    jmp halt_loop

; --- Helper Functions and Data ---
print_string_16bit:
    mov ah, 0x0E
.loop16:
    lodsb; or al, al; jz .done16; int 0x10; jmp .loop16
.done16: ret

no_long_mode:
    mov si, msg_no_lm
    call print_string_16bit
    jmp halt_loop

print_string_lm_64bit:
.next_char_64:
    mov al, [rsi]
    inc rsi
    or al, al
    jz .done_64
    mov ah, 0x0B ; Light Cyan color
    mov [rdi], ax
    add rdi, 2
    jmp .next_char_64
.done_64:
    ret

msg_stage2_start: db "Stage 2 started...", 0Dh, 0Ah, 0
msg_no_lm: db "Long Mode Not Supported. Halting.", 0
msg_lm: db "Entered 64-bit long mode! System Halted.", 0

; --- GDT Definition ---
gdt_start:
    dq 0 ; Null Descriptor
    dq 0x00209A0000000000 ; 64-bit Code Segment (L=1, D/B=0)
    dq 0x00C092000000FFFF ; 32-bit Data Segment (D/B=1)
gdt_end:
gdt_descriptor: dw gdt_end - gdt_start - 1; dd gdt_start
