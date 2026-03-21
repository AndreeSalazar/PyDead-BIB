// ============================================================
// ADead-BIB — OS-Level Code Generation (Phase 6)
// ============================================================
// Soporte completo para:
//   - 16-bit real mode codegen
//   - 32-bit protected mode codegen
//   - @interrupt / @exception handler attributes
//   - @packed struct support
//   - GDT/IDT structure generation
//   - Paging setup helpers
//   - Rust kernel integration bridge
//
// Pipeline: AST → ADeadIR → OsCodegen → Encoder → Flat Binary
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use std::collections::HashMap;

// ============================================================
// CPU Mode — Multi-mode code generation
// ============================================================

/// Modo de CPU para generación de código multi-mode.
///
/// ADead-BIB necesita generar código para los tres modos de CPU x86:
/// - Real16: Boot sector, BIOS calls (modo real 16-bit)
/// - Protected32: Transición, drivers legacy (modo protegido 32-bit)
/// - Long64: Kernel, aplicaciones (modo largo 64-bit) — YA EXISTENTE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuMode {
    /// Modo real 16-bit — Boot sector, BIOS interrupts
    /// Registros: AX, BX, CX, DX, SI, DI, SP, BP
    /// Segmentos: CS, DS, ES, SS
    /// Direccionamiento: 20-bit (1MB)
    Real16,

    /// Modo protegido 32-bit — Transición, drivers
    /// Registros: EAX, EBX, ECX, EDX, ESI, EDI, ESP, EBP
    /// GDT requerida, paginación opcional
    /// Direccionamiento: 32-bit (4GB)
    Protected32,

    /// Modo largo 64-bit — Kernel, aplicaciones (default)
    /// Registros: RAX-R15, XMM0-XMM15
    /// GDT + paginación requeridas
    /// Direccionamiento: 64-bit (virtual)
    Long64,
}

impl CpuMode {
    /// Retorna el prefijo de operand size override para este modo.
    /// En modo real, 0x66 cambia a operandos de 32-bit.
    /// En modo protegido, 0x66 cambia a operandos de 16-bit.
    pub fn operand_size_prefix(&self) -> Option<u8> {
        match self {
            CpuMode::Real16 => Some(0x66),      // Override a 32-bit en real mode
            CpuMode::Protected32 => Some(0x66), // Override a 16-bit en protected mode
            CpuMode::Long64 => None,            // REX prefix maneja todo
        }
    }

    /// Retorna el prefijo de address size override para este modo.
    pub fn address_size_prefix(&self) -> Option<u8> {
        match self {
            CpuMode::Real16 => Some(0x67),      // Override a 32-bit addressing
            CpuMode::Protected32 => Some(0x67), // Override a 16-bit addressing
            CpuMode::Long64 => None,
        }
    }

    /// Retorna el tamaño default de operandos en bytes.
    pub fn default_operand_size(&self) -> u8 {
        match self {
            CpuMode::Real16 => 2,      // 16-bit
            CpuMode::Protected32 => 4, // 32-bit
            CpuMode::Long64 => 8,      // 64-bit (con REX.W)
        }
    }
}

impl std::fmt::Display for CpuMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CpuMode::Real16 => write!(f, "real16"),
            CpuMode::Protected32 => write!(f, "protected32"),
            CpuMode::Long64 => write!(f, "long64"),
        }
    }
}

// ============================================================
// 16-bit Real Mode Code Generator
// ============================================================

/// Generador de código para modo real 16-bit.
///
/// Genera instrucciones x86 de 16 bits para boot sectors y
/// código que se ejecuta antes de la transición a modo protegido.
pub struct RealModeCodegen {
    code: Vec<u8>,
    labels: HashMap<String, usize>,
    pending_jumps: Vec<(usize, String)>,
}

impl RealModeCodegen {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            labels: HashMap::new(),
            pending_jumps: Vec::new(),
        }
    }

    /// Emite bytes crudos al buffer de código.
    pub fn emit(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    /// Registra un label en la posición actual.
    pub fn label(&mut self, name: &str) {
        self.labels.insert(name.to_string(), self.code.len());
    }

    /// Posición actual en el buffer de código.
    pub fn pos(&self) -> usize {
        self.code.len()
    }

    // ---- Instrucciones de 16-bit ----

    /// CLI — Desactivar interrupciones
    pub fn cli(&mut self) {
        self.emit(&[0xFA]);
    }

    /// STI — Activar interrupciones
    pub fn sti(&mut self) {
        self.emit(&[0xFB]);
    }

    /// HLT — Detener CPU
    pub fn hlt(&mut self) {
        self.emit(&[0xF4]);
    }

    /// XOR reg16, reg16 (zeroing)
    pub fn xor_reg16(&mut self, reg: u8) {
        // xor ax, ax = 0x31 0xC0 (reg=0)
        // xor bx, bx = 0x31 0xDB (reg=3)
        let modrm = 0xC0 | (reg << 3) | reg;
        self.emit(&[0x31, modrm]);
    }

    /// MOV segment_reg, reg16
    /// seg: 0=ES, 1=CS, 2=SS, 3=DS, 4=FS, 5=GS
    pub fn mov_seg_reg16(&mut self, seg: u8, src_reg: u8) {
        let modrm = 0xC0 | ((seg & 0x07) << 3) | (src_reg & 0x07);
        self.emit(&[0x8E, modrm]);
    }

    /// MOV reg16, imm16
    pub fn mov_reg16_imm16(&mut self, reg: u8, value: u16) {
        self.emit(&[0xB8 + (reg & 0x07)]);
        self.emit(&value.to_le_bytes());
    }

    /// MOV SP, imm16
    pub fn mov_sp_imm16(&mut self, value: u16) {
        self.emit(&[0xBC]); // mov sp, imm16
        self.emit(&value.to_le_bytes());
    }

    /// INT n — Software interrupt (BIOS call)
    pub fn int(&mut self, vector: u8) {
        self.emit(&[0xCD, vector]);
    }

    /// MOV AH, imm8
    pub fn mov_ah_imm8(&mut self, value: u8) {
        self.emit(&[0xB4, value]);
    }

    /// MOV AL, imm8
    pub fn mov_al_imm8(&mut self, value: u8) {
        self.emit(&[0xB0, value]);
    }

    /// LODSB — Load byte from [SI] into AL, increment SI
    pub fn lodsb(&mut self) {
        self.emit(&[0xAC]);
    }

    /// OR AL, AL — Test if AL is zero
    pub fn or_al_al(&mut self) {
        self.emit(&[0x08, 0xC0]);
    }

    /// JZ rel8 — Jump if zero (short)
    pub fn jz_short(&mut self, offset: i8) {
        self.emit(&[0x74, offset as u8]);
    }

    /// JMP short rel8 — Salto corto incondicional
    pub fn jmp_short(&mut self, offset: i8) {
        self.emit(&[0xEB, offset as u8]);
    }

    /// JMP near rel16 — Salto largo en modo real
    pub fn jmp_near(&mut self, offset: i16) {
        self.emit(&[0xE9]);
        self.emit(&offset.to_le_bytes());
    }

    /// Infinite loop: JMP $ (EB FE)
    pub fn infinite_loop(&mut self) {
        self.emit(&[0xEB, 0xFE]);
    }

    /// OUT imm8, AL — Escribir byte a puerto I/O
    pub fn out_imm8_al(&mut self, port: u8) {
        self.emit(&[0xE6, port]);
    }

    /// IN AL, imm8 — Leer byte de puerto I/O
    pub fn in_al_imm8(&mut self, port: u8) {
        self.emit(&[0xE4, port]);
    }

    /// LGDT [mem] — Cargar GDT (con prefijo de operand size para 32-bit en real mode)
    pub fn lgdt_mem16(&mut self, addr: u16) {
        // En modo real, lgdt carga un puntero de 6 bytes (2 limit + 4 base)
        // 0x0F 0x01 /2 con dirección absoluta
        self.emit(&[0x0F, 0x01, 0x16]); // lgdt [imm16]
        self.emit(&addr.to_le_bytes());
    }

    /// MOV CR0, EAX (requiere prefijo 0x66 en modo real para acceder a EAX)
    pub fn mov_cr0_eax(&mut self) {
        self.emit(&[0x0F, 0x22, 0xC0]);
    }

    /// MOV EAX, CR0 (requiere prefijo 0x66 en modo real)
    pub fn mov_eax_cr0(&mut self) {
        self.emit(&[0x0F, 0x20, 0xC0]);
    }

    /// OR EAX, 1 — Set PE bit (Protected Mode Enable)
    pub fn or_eax_1(&mut self) {
        self.emit(&[0x66, 0x83, 0xC8, 0x01]); // or eax, 1 (con prefijo 0x66 para 32-bit)
    }

    /// Far JMP ptr16:32 — Salto far para cambio de modo
    pub fn far_jmp(&mut self, selector: u16, offset: u32) {
        self.emit(&[0x66, 0xEA]); // Far jmp con prefijo 0x66 para offset de 32-bit
        self.emit(&offset.to_le_bytes());
        self.emit(&selector.to_le_bytes());
    }

    /// Genera rutina de impresión BIOS (INT 10h, AH=0x0E)
    /// Imprime string null-terminated apuntada por SI
    pub fn bios_print_routine(&mut self) {
        let start = self.pos();
        self.lodsb(); // al = [si++]
        self.or_al_al(); // test al, al
        self.jz_short(4); // jz done (skip 4 bytes: mov ah + int + jmp)
        self.mov_ah_imm8(0x0E); // ah = 0x0E (teletype)
        self.int(0x10); // BIOS video interrupt
        self.jmp_short(-((self.pos() - start + 2) as i8)); // loop back
                                                           // done: (falls through here)
    }

    /// Genera secuencia de habilitación de A20 gate via puerto 0x92
    pub fn enable_a20_fast(&mut self) {
        self.in_al_imm8(0x92); // in al, 0x92
        self.emit(&[0x0C, 0x02]); // or al, 2
        self.emit(&[0x24, 0xFE]); // and al, 0xFE (no reset)
        self.out_imm8_al(0x92); // out 0x92, al
    }

    /// Resuelve saltos pendientes y retorna el código final.
    pub fn finalize(&mut self) -> Vec<u8> {
        // Resolver saltos pendientes
        for (offset, label_name) in &self.pending_jumps {
            if let Some(&target) = self.labels.get(label_name) {
                let rel = (target as i32 - (*offset as i32 + 2)) as i16;
                self.code[*offset] = (rel & 0xFF) as u8;
                if self.code.len() > *offset + 1 {
                    self.code[*offset + 1] = ((rel >> 8) & 0xFF) as u8;
                }
            }
        }
        self.code.clone()
    }
}

// ============================================================
// 32-bit Protected Mode Code Generator
// ============================================================

/// Generador de código para modo protegido 32-bit.
pub struct ProtectedModeCodegen {
    code: Vec<u8>,
}

impl ProtectedModeCodegen {
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    pub fn emit(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    /// MOV EAX, imm32
    pub fn mov_eax_imm32(&mut self, value: u32) {
        self.emit(&[0xB8]);
        self.emit(&value.to_le_bytes());
    }

    /// MOV reg32, imm32
    pub fn mov_reg32_imm32(&mut self, reg: u8, value: u32) {
        self.emit(&[0xB8 + (reg & 0x07)]);
        self.emit(&value.to_le_bytes());
    }

    /// MOV segment, reg32 (reload segments after mode switch)
    pub fn mov_seg_reg32(&mut self, seg: u8, src_reg: u8) {
        let modrm = 0xC0 | ((seg & 0x07) << 3) | (src_reg & 0x07);
        self.emit(&[0x8E, modrm]);
    }

    /// MOV ESP, imm32 — Setup stack
    pub fn mov_esp_imm32(&mut self, value: u32) {
        self.emit(&[0xBC]);
        self.emit(&value.to_le_bytes());
    }

    /// MOV [mem32], reg32
    pub fn mov_mem32_reg32(&mut self, addr: u32, reg: u8) {
        let modrm = 0x05 | ((reg & 0x07) << 3); // mod=00, r/m=101 (disp32)
        self.emit(&[0x89, modrm]);
        self.emit(&addr.to_le_bytes());
    }

    /// CLI
    pub fn cli(&mut self) {
        self.emit(&[0xFA]);
    }

    /// STI
    pub fn sti(&mut self) {
        self.emit(&[0xFB]);
    }

    /// HLT
    pub fn hlt(&mut self) {
        self.emit(&[0xF4]);
    }

    /// LGDT [mem32]
    pub fn lgdt(&mut self, addr: u32) {
        self.emit(&[0x0F, 0x01, 0x15]); // lgdt [disp32]
        self.emit(&addr.to_le_bytes());
    }

    /// LIDT [mem32]
    pub fn lidt(&mut self, addr: u32) {
        self.emit(&[0x0F, 0x01, 0x1D]); // lidt [disp32]
        self.emit(&addr.to_le_bytes());
    }

    /// MOV CR0, EAX
    pub fn mov_cr0_eax(&mut self) {
        self.emit(&[0x0F, 0x22, 0xC0]);
    }

    /// MOV EAX, CR0
    pub fn mov_eax_cr0(&mut self) {
        self.emit(&[0x0F, 0x20, 0xC0]);
    }

    /// MOV CR3, EAX — Load page directory
    pub fn mov_cr3_eax(&mut self) {
        self.emit(&[0x0F, 0x22, 0xD8]);
    }

    /// MOV CR4, EAX
    pub fn mov_cr4_eax(&mut self) {
        self.emit(&[0x0F, 0x22, 0xE0]);
    }

    /// MOV EAX, CR4
    pub fn mov_eax_cr4(&mut self) {
        self.emit(&[0x0F, 0x20, 0xE0]);
    }

    /// Far JMP ptr16:32 — For long mode transition
    pub fn far_jmp(&mut self, selector: u16, offset: u32) {
        self.emit(&[0xEA]);
        self.emit(&offset.to_le_bytes());
        self.emit(&selector.to_le_bytes());
    }

    /// Reload all data segment registers after GDT load
    pub fn reload_segments(&mut self, data_selector: u16) {
        // mov ax, data_selector
        self.emit(&[0x66, 0xB8]);
        self.emit(&data_selector.to_le_bytes());
        // mov ds, ax
        self.emit(&[0x8E, 0xD8]);
        // mov es, ax
        self.emit(&[0x8E, 0xC0]);
        // mov fs, ax
        self.emit(&[0x8E, 0xE0]);
        // mov gs, ax
        self.emit(&[0x8E, 0xE8]);
        // mov ss, ax
        self.emit(&[0x8E, 0xD0]);
    }

    pub fn finalize(&self) -> Vec<u8> {
        self.code.clone()
    }
}

// ============================================================
// @interrupt Handler — Auto push/pop + IRETQ
// ============================================================

/// Genera el wrapper de un interrupt handler.
///
/// El compilador auto-genera:
/// 1. Push de todos los registros de propósito general
/// 2. El código del handler del usuario
/// 3. Pop de todos los registros
/// 4. IRETQ (retorno de interrupción)
///
/// Esto elimina la necesidad de escribir el boilerplate manualmente.
pub struct InterruptHandlerGen {
    mode: CpuMode,
}

impl InterruptHandlerGen {
    pub fn new(mode: CpuMode) -> Self {
        Self { mode }
    }

    /// Genera el prólogo del interrupt handler (push all registers).
    pub fn generate_prologue(&self) -> Vec<u8> {
        match self.mode {
            CpuMode::Long64 => {
                let mut code = Vec::new();
                // Push all general purpose registers (64-bit)
                code.push(0x50); // push rax
                code.push(0x53); // push rbx
                code.push(0x51); // push rcx
                code.push(0x52); // push rdx
                code.push(0x56); // push rsi
                code.push(0x57); // push rdi
                code.push(0x55); // push rbp
                code.extend_from_slice(&[0x41, 0x50]); // push r8
                code.extend_from_slice(&[0x41, 0x51]); // push r9
                code.extend_from_slice(&[0x41, 0x52]); // push r10
                code.extend_from_slice(&[0x41, 0x53]); // push r11
                code.extend_from_slice(&[0x41, 0x54]); // push r12
                code.extend_from_slice(&[0x41, 0x55]); // push r13
                code.extend_from_slice(&[0x41, 0x56]); // push r14
                code.extend_from_slice(&[0x41, 0x57]); // push r15
                code
            }
            CpuMode::Protected32 => {
                vec![
                    0x60, // pushad (push all 32-bit registers)
                ]
            }
            CpuMode::Real16 => {
                vec![
                    0x60, // pusha (push all 16-bit registers)
                ]
            }
        }
    }

    /// Genera el epílogo del interrupt handler (pop all registers + iret).
    pub fn generate_epilogue(&self) -> Vec<u8> {
        match self.mode {
            CpuMode::Long64 => {
                let mut code = Vec::new();
                // Pop all general purpose registers (reverse order)
                code.extend_from_slice(&[0x41, 0x5F]); // pop r15
                code.extend_from_slice(&[0x41, 0x5E]); // pop r14
                code.extend_from_slice(&[0x41, 0x5D]); // pop r13
                code.extend_from_slice(&[0x41, 0x5C]); // pop r12
                code.extend_from_slice(&[0x41, 0x5B]); // pop r11
                code.extend_from_slice(&[0x41, 0x5A]); // pop r10
                code.extend_from_slice(&[0x41, 0x59]); // pop r9
                code.extend_from_slice(&[0x41, 0x58]); // pop r8
                code.push(0x5D); // pop rbp
                code.push(0x5F); // pop rdi
                code.push(0x5E); // pop rsi
                code.push(0x5A); // pop rdx
                code.push(0x59); // pop rcx
                code.push(0x5B); // pop rbx
                code.push(0x58); // pop rax
                                 // IRETQ (REX.W + IRET)
                code.extend_from_slice(&[0x48, 0xCF]);
                code
            }
            CpuMode::Protected32 => {
                vec![
                    0x61, // popad
                    0xCF, // iret (32-bit)
                ]
            }
            CpuMode::Real16 => {
                vec![
                    0x61, // popa
                    0xCF, // iret (16-bit)
                ]
            }
        }
    }

    /// Genera un interrupt handler completo envolviendo el código del usuario.
    pub fn wrap_handler(&self, user_code: &[u8]) -> Vec<u8> {
        let mut handler = Vec::new();
        handler.extend_from_slice(&self.generate_prologue());
        handler.extend_from_slice(user_code);
        handler.extend_from_slice(&self.generate_epilogue());
        handler
    }
}

// ============================================================
// @packed Struct — Exact memory layout
// ============================================================

/// Representa un campo de un struct packed.
#[derive(Debug, Clone)]
pub struct PackedField {
    pub name: String,
    pub size: usize,   // Tamaño en bytes
    pub offset: usize, // Offset desde el inicio del struct
}

/// Struct con layout exacto en memoria (sin padding).
///
/// Esencial para estructuras de hardware como GDT entries,
/// IDT entries, page table entries, etc.
#[derive(Debug, Clone)]
pub struct PackedStruct {
    pub name: String,
    pub fields: Vec<PackedField>,
    pub total_size: usize,
}

impl PackedStruct {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
            total_size: 0,
        }
    }

    /// Agrega un campo al struct packed.
    pub fn add_field(&mut self, name: &str, size: usize) {
        let offset = self.total_size;
        self.fields.push(PackedField {
            name: name.to_string(),
            size,
            offset,
        });
        self.total_size += size;
    }

    /// Genera los bytes del struct con los valores dados.
    pub fn generate_bytes(&self, values: &[u64]) -> Vec<u8> {
        let mut bytes = vec![0u8; self.total_size];
        for (i, field) in self.fields.iter().enumerate() {
            if i < values.len() {
                let val = values[i];
                let val_bytes = val.to_le_bytes();
                let copy_len = field.size.min(8);
                bytes[field.offset..field.offset + copy_len]
                    .copy_from_slice(&val_bytes[..copy_len]);
            }
        }
        bytes
    }
}

// ============================================================
// GDT (Global Descriptor Table) Generation
// ============================================================

/// Tipo de segmento GDT.
#[derive(Debug, Clone, Copy)]
pub enum GdtSegmentType {
    Null,
    KernelCode64,
    KernelData,
    UserCode64,
    UserData,
    KernelCode32,
    Tss,
}

/// Entrada de la GDT (8 bytes).
#[derive(Debug, Clone, Copy)]
pub struct GdtEntry {
    pub limit_low: u16,
    pub base_low: u16,
    pub base_mid: u8,
    pub access: u8,
    pub flags_limit: u8,
    pub base_high: u8,
}

impl GdtEntry {
    /// Crea una entrada null (requerida como primera entrada).
    pub fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0,
            flags_limit: 0,
            base_high: 0,
        }
    }

    /// Crea una entrada de código kernel 64-bit.
    pub fn kernel_code_64() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0x0000,
            base_mid: 0x00,
            access: 0x9A,      // Present, Ring 0, Code, Execute/Read
            flags_limit: 0xAF, // Long mode, 4KB granularity, limit[19:16]=0xF
            base_high: 0x00,
        }
    }

    /// Crea una entrada de datos kernel.
    pub fn kernel_data() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0x0000,
            base_mid: 0x00,
            access: 0x92,      // Present, Ring 0, Data, Read/Write
            flags_limit: 0xCF, // 32-bit, 4KB granularity
            base_high: 0x00,
        }
    }

    /// Crea una entrada de código usuario 64-bit (Ring 3).
    pub fn user_code_64() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0x0000,
            base_mid: 0x00,
            access: 0xFA,      // Present, Ring 3, Code, Execute/Read
            flags_limit: 0xAF, // Long mode
            base_high: 0x00,
        }
    }

    /// Crea una entrada de datos usuario (Ring 3).
    pub fn user_data() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0x0000,
            base_mid: 0x00,
            access: 0xF2, // Present, Ring 3, Data, Read/Write
            flags_limit: 0xCF,
            base_high: 0x00,
        }
    }

    /// Crea una entrada de código kernel 32-bit (para modo protegido).
    pub fn kernel_code_32() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0x0000,
            base_mid: 0x00,
            access: 0x9A,      // Present, Ring 0, Code, Execute/Read
            flags_limit: 0xCF, // 32-bit, 4KB granularity
            base_high: 0x00,
        }
    }

    /// Serializa la entrada a 8 bytes.
    pub fn to_bytes(&self) -> [u8; 8] {
        [
            (self.limit_low & 0xFF) as u8,
            ((self.limit_low >> 8) & 0xFF) as u8,
            (self.base_low & 0xFF) as u8,
            ((self.base_low >> 8) & 0xFF) as u8,
            self.base_mid,
            self.access,
            self.flags_limit,
            self.base_high,
        ]
    }
}

/// Generador de GDT completa.
pub struct GdtGenerator {
    entries: Vec<GdtEntry>,
}

impl GdtGenerator {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Agrega una entrada a la GDT.
    pub fn add_entry(&mut self, entry: GdtEntry) -> u16 {
        let selector = (self.entries.len() * 8) as u16;
        self.entries.push(entry);
        selector
    }

    /// Genera la GDT estándar para un OS (null + kernel code + kernel data + user code + user data).
    pub fn generate_standard_gdt() -> Self {
        let mut gdt = Self::new();
        gdt.add_entry(GdtEntry::null()); // 0x00: Null
        gdt.add_entry(GdtEntry::kernel_code_64()); // 0x08: Kernel Code 64
        gdt.add_entry(GdtEntry::kernel_data()); // 0x10: Kernel Data
        gdt.add_entry(GdtEntry::user_code_64()); // 0x18: User Code 64
        gdt.add_entry(GdtEntry::user_data()); // 0x20: User Data
        gdt
    }

    /// Genera la GDT para boot (null + code32 + data).
    pub fn generate_boot_gdt() -> Self {
        let mut gdt = Self::new();
        gdt.add_entry(GdtEntry::null()); // 0x00: Null
        gdt.add_entry(GdtEntry::kernel_code_32()); // 0x08: Code 32-bit
        gdt.add_entry(GdtEntry::kernel_data()); // 0x10: Data
        gdt
    }

    /// Serializa toda la GDT a bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for entry in &self.entries {
            bytes.extend_from_slice(&entry.to_bytes());
        }
        bytes
    }

    /// Genera el GDT pointer (limit + base) para LGDT.
    /// El pointer es de 6 bytes en modo protegido, 10 bytes en modo largo.
    pub fn generate_pointer(&self, base_address: u32) -> Vec<u8> {
        let limit = (self.entries.len() * 8 - 1) as u16;
        let mut ptr = Vec::new();
        ptr.extend_from_slice(&limit.to_le_bytes());
        ptr.extend_from_slice(&base_address.to_le_bytes());
        ptr
    }

    /// Número de entradas en la GDT.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Tamaño total en bytes.
    pub fn size_bytes(&self) -> usize {
        self.entries.len() * 8
    }
}

// ============================================================
// IDT (Interrupt Descriptor Table) Generation
// ============================================================

/// Entrada de la IDT para modo largo (64-bit) — 16 bytes.
#[derive(Debug, Clone, Copy)]
pub struct IdtEntry64 {
    pub offset_low: u16,  // Bits 0-15 del handler address
    pub selector: u16,    // Code segment selector (GDT)
    pub ist: u8,          // Interrupt Stack Table index (0 = no IST)
    pub type_attr: u8,    // Type + DPL + Present
    pub offset_mid: u16,  // Bits 16-31 del handler address
    pub offset_high: u32, // Bits 32-63 del handler address
    pub reserved: u32,    // Must be 0
}

impl IdtEntry64 {
    /// Crea una entrada IDT vacía (not present).
    pub fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Crea una entrada IDT para un interrupt gate.
    ///
    /// # Arguments
    /// * `handler_addr` - Dirección del handler
    /// * `selector` - Selector de código GDT (normalmente 0x08)
    /// * `ist` - IST index (0 = no IST)
    /// * `dpl` - Descriptor Privilege Level (0 = kernel, 3 = user)
    pub fn interrupt_gate(handler_addr: u64, selector: u16, ist: u8, dpl: u8) -> Self {
        Self {
            offset_low: (handler_addr & 0xFFFF) as u16,
            selector,
            ist: ist & 0x07,
            type_attr: 0x80 | ((dpl & 0x03) << 5) | 0x0E, // Present + DPL + Interrupt Gate (0xE)
            offset_mid: ((handler_addr >> 16) & 0xFFFF) as u16,
            offset_high: ((handler_addr >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    /// Crea una entrada IDT para un trap gate (no desactiva interrupciones).
    pub fn trap_gate(handler_addr: u64, selector: u16, ist: u8, dpl: u8) -> Self {
        Self {
            offset_low: (handler_addr & 0xFFFF) as u16,
            selector,
            ist: ist & 0x07,
            type_attr: 0x80 | ((dpl & 0x03) << 5) | 0x0F, // Present + DPL + Trap Gate (0xF)
            offset_mid: ((handler_addr >> 16) & 0xFFFF) as u16,
            offset_high: ((handler_addr >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    /// Serializa la entrada a 16 bytes.
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..2].copy_from_slice(&self.offset_low.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.selector.to_le_bytes());
        bytes[4] = self.ist;
        bytes[5] = self.type_attr;
        bytes[6..8].copy_from_slice(&self.offset_mid.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.offset_high.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.reserved.to_le_bytes());
        bytes
    }
}

/// Generador de IDT completa (256 entradas para x86-64).
pub struct IdtGenerator {
    entries: Vec<IdtEntry64>,
}

impl IdtGenerator {
    pub fn new() -> Self {
        Self {
            entries: vec![IdtEntry64::empty(); 256],
        }
    }

    /// Registra un handler para un vector de interrupción.
    pub fn set_handler(&mut self, vector: u8, handler_addr: u64, selector: u16, ist: u8) {
        self.entries[vector as usize] = IdtEntry64::interrupt_gate(handler_addr, selector, ist, 0);
    }

    /// Registra un trap handler (para excepciones).
    pub fn set_trap(&mut self, vector: u8, handler_addr: u64, selector: u16) {
        self.entries[vector as usize] = IdtEntry64::trap_gate(handler_addr, selector, 0, 0);
    }

    /// Serializa toda la IDT a bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for entry in &self.entries {
            bytes.extend_from_slice(&entry.to_bytes());
        }
        bytes
    }

    /// Genera el IDT pointer para LIDT (10 bytes: 2 limit + 8 base).
    pub fn generate_pointer(&self, base_address: u64) -> Vec<u8> {
        let limit = (self.entries.len() * 16 - 1) as u16;
        let mut ptr = Vec::new();
        ptr.extend_from_slice(&limit.to_le_bytes());
        ptr.extend_from_slice(&base_address.to_le_bytes());
        ptr
    }

    /// Tamaño total en bytes.
    pub fn size_bytes(&self) -> usize {
        self.entries.len() * 16 // 4096 bytes para 256 entradas
    }
}

// ============================================================
// Paging Setup Helpers
// ============================================================

/// Flags de página x86-64.
pub const PAGE_PRESENT: u64 = 1 << 0;
pub const PAGE_WRITABLE: u64 = 1 << 1;
pub const PAGE_USER: u64 = 1 << 2;
pub const PAGE_WRITE_THROUGH: u64 = 1 << 3;
pub const PAGE_CACHE_DISABLE: u64 = 1 << 4;
pub const PAGE_HUGE: u64 = 1 << 7; // 2MB page (PD level) or 1GB page (PDPT level)
pub const PAGE_NO_EXECUTE: u64 = 1 << 63;

/// Generador de tablas de paginación x86-64.
///
/// Estructura de paginación de 4 niveles:
/// PML4 → PDPT → PD → PT → Physical Page
///
/// Cada tabla tiene 512 entradas de 8 bytes = 4096 bytes (una página).
pub struct PagingSetup {
    /// PML4 — Page Map Level 4 (raíz)
    pub pml4: Vec<u64>,
    /// PDPT — Page Directory Pointer Table
    pub pdpt: Vec<u64>,
    /// PD — Page Directory
    pub pd: Vec<u64>,
}

impl PagingSetup {
    pub fn new() -> Self {
        Self {
            pml4: vec![0u64; 512],
            pdpt: vec![0u64; 512],
            pd: vec![0u64; 512],
        }
    }

    /// Configura identity mapping para los primeros N megabytes usando páginas de 2MB.
    ///
    /// Esto mapea dirección virtual = dirección física, esencial para
    /// la transición a modo largo donde el código necesita seguir
    /// ejecutándose en la misma dirección.
    pub fn identity_map_2mb(&mut self, megabytes: usize, pdpt_phys: u64, pd_phys: u64) {
        // PML4[0] → PDPT
        self.pml4[0] = pdpt_phys | PAGE_PRESENT | PAGE_WRITABLE;

        // PDPT[0] → PD
        self.pdpt[0] = pd_phys | PAGE_PRESENT | PAGE_WRITABLE;

        // PD entries: cada una mapea 2MB
        let num_pages = (megabytes + 1) / 2; // Redondear arriba
        for i in 0..num_pages.min(512) {
            let phys_addr = (i as u64) * 0x200000; // 2MB por página
            self.pd[i] = phys_addr | PAGE_PRESENT | PAGE_WRITABLE | PAGE_HUGE;
        }
    }

    /// Serializa PML4 a bytes (4096 bytes).
    pub fn pml4_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4096);
        for entry in &self.pml4 {
            bytes.extend_from_slice(&entry.to_le_bytes());
        }
        bytes
    }

    /// Serializa PDPT a bytes (4096 bytes).
    pub fn pdpt_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4096);
        for entry in &self.pdpt {
            bytes.extend_from_slice(&entry.to_le_bytes());
        }
        bytes
    }

    /// Serializa PD a bytes (4096 bytes).
    pub fn pd_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4096);
        for entry in &self.pd {
            bytes.extend_from_slice(&entry.to_le_bytes());
        }
        bytes
    }

    /// Genera todas las tablas de paginación concatenadas.
    /// Orden: PML4 (4KB) + PDPT (4KB) + PD (4KB) = 12KB total
    pub fn generate_all(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12288);
        bytes.extend_from_slice(&self.pml4_bytes());
        bytes.extend_from_slice(&self.pdpt_bytes());
        bytes.extend_from_slice(&self.pd_bytes());
        bytes
    }
}

// ============================================================
// Rust Kernel Integration Bridge
// ============================================================

/// Puente de integración ADead-BIB ↔ Rust para desarrollo de OS.
///
/// ADead-BIB maneja: boot, GDT, IDT, ISR wrappers, hardware init
/// Rust maneja: scheduler, filesystem, memory manager, drivers
///
/// La comunicación se hace via:
/// 1. Calling convention compatible (System V AMD64 o Windows x64)
/// 2. Símbolos exportados con nombres C-compatible
/// 3. Estructuras con layout #[repr(C)]
pub struct RustKernelBridge {
    /// Símbolos exportados por ADead-BIB para que Rust los llame
    pub exported_symbols: Vec<ExportedSymbol>,
    /// Símbolos que ADead-BIB espera de Rust
    pub imported_symbols: Vec<ImportedSymbol>,
    /// Calling convention usada
    pub calling_convention: CallingConvention,
}

/// Símbolo exportado por ADead-BIB.
#[derive(Debug, Clone)]
pub struct ExportedSymbol {
    pub name: String,
    pub address: u64,
    pub symbol_type: SymbolType,
}

/// Símbolo importado de Rust.
#[derive(Debug, Clone)]
pub struct ImportedSymbol {
    pub name: String,
    pub symbol_type: SymbolType,
}

/// Tipo de símbolo.
#[derive(Debug, Clone, Copy)]
pub enum SymbolType {
    Function,
    Data,
    Bss,
}

/// Calling convention para la interfaz.
#[derive(Debug, Clone, Copy)]
pub enum CallingConvention {
    /// System V AMD64 (Linux, macOS) — args en RDI, RSI, RDX, RCX, R8, R9
    SystemV,
    /// Windows x64 — args en RCX, RDX, R8, R9
    Win64,
}

impl RustKernelBridge {
    pub fn new(convention: CallingConvention) -> Self {
        Self {
            exported_symbols: Vec::new(),
            imported_symbols: Vec::new(),
            calling_convention: convention,
        }
    }

    /// Registra un símbolo exportado por ADead-BIB.
    pub fn export(&mut self, name: &str, address: u64, sym_type: SymbolType) {
        self.exported_symbols.push(ExportedSymbol {
            name: name.to_string(),
            address,
            symbol_type: sym_type,
        });
    }

    /// Registra un símbolo que ADead-BIB necesita de Rust.
    pub fn import(&mut self, name: &str, sym_type: SymbolType) {
        self.imported_symbols.push(ImportedSymbol {
            name: name.to_string(),
            symbol_type: sym_type,
        });
    }

    /// Genera un linker script para combinar objetos ADead-BIB y Rust.
    pub fn generate_linker_script(&self, kernel_base: u64) -> String {
        let mut script = String::new();
        script.push_str(&format!(
            "/* ADead-BIB + Rust Kernel Linker Script */\n\
             /* Auto-generated by ADead-BIB OS Codegen */\n\n\
             ENTRY(_start)\n\n\
             SECTIONS {{\n\
             \t. = 0x{:X};\n\n\
             \t.text : {{\n\
             \t\t*(.text.boot)    /* ADead-BIB boot code */\n\
             \t\t*(.text)         /* All code */\n\
             \t\t*(.text.*)       /* Rust code sections */\n\
             \t}}\n\n\
             \t.rodata : {{\n\
             \t\t*(.rodata)\n\
             \t\t*(.rodata.*)\n\
             \t}}\n\n\
             \t.data : {{\n\
             \t\t*(.data)\n\
             \t\t*(.data.*)\n\
             \t}}\n\n\
             \t.bss : {{\n\
             \t\t*(COMMON)\n\
             \t\t*(.bss)\n\
             \t\t*(.bss.*)\n\
             \t}}\n\n",
            kernel_base
        ));

        // Exported symbols
        if !self.exported_symbols.is_empty() {
            script.push_str("\t/* ADead-BIB Exported Symbols */\n");
            for sym in &self.exported_symbols {
                script.push_str(&format!("\tPROVIDE({} = 0x{:X});\n", sym.name, sym.address));
            }
        }

        script.push_str("}\n");
        script
    }

    /// Genera un header C para la interfaz ADead-BIB ↔ Rust.
    pub fn generate_c_header(&self) -> String {
        let mut header = String::new();
        header.push_str(
            "/* ADead-BIB Kernel Interface Header */\n\
             /* Auto-generated — Do not edit */\n\n\
             #ifndef ADEAD_KERNEL_H\n\
             #define ADEAD_KERNEL_H\n\n\
             #include <stdint.h>\n\n",
        );

        header.push_str("/* Functions exported by ADead-BIB */\n");
        for sym in &self.exported_symbols {
            if matches!(sym.symbol_type, SymbolType::Function) {
                header.push_str(&format!("extern void {}(void);\n", sym.name));
            }
        }

        header.push_str("\n/* Functions that ADead-BIB expects from Rust */\n");
        for sym in &self.imported_symbols {
            if matches!(sym.symbol_type, SymbolType::Function) {
                header.push_str(&format!("void {}(void);\n", sym.name));
            }
        }

        header.push_str("\n#endif /* ADEAD_KERNEL_H */\n");
        header
    }
}

// ============================================================
// Mode Transition Helpers
// ============================================================

/// Genera la secuencia completa de transición Real Mode → Protected Mode.
pub fn generate_real_to_protected(gdt_address: u32, code32_offset: u32) -> Vec<u8> {
    let mut rm = RealModeCodegen::new();

    // 1. Desactivar interrupciones
    rm.cli();

    // 2. Habilitar A20 gate
    rm.enable_a20_fast();

    // 3. Cargar GDT
    rm.lgdt_mem16(gdt_address as u16);

    // 4. Set PE bit in CR0
    rm.mov_eax_cr0();
    rm.or_eax_1();
    rm.mov_cr0_eax();

    // 5. Far jump a código 32-bit (flush pipeline + load CS)
    rm.far_jmp(0x08, code32_offset);

    rm.finalize()
}

/// Genera la secuencia de transición Protected Mode → Long Mode.
pub fn generate_protected_to_long(
    pml4_address: u32,
    gdt64_address: u32,
    code64_offset: u32,
) -> Vec<u8> {
    let mut pm = ProtectedModeCodegen::new();

    // 1. Desactivar paginación si estaba activa
    pm.mov_eax_cr0();
    pm.emit(&[0x25]); // and eax, ~(1 << 31) — clear PG bit
    pm.emit(&0x7FFFFFFFu32.to_le_bytes());
    pm.mov_cr0_eax();

    // 2. Habilitar PAE (Physical Address Extension) en CR4
    pm.mov_eax_cr4();
    pm.emit(&[0x0D]); // or eax, (1 << 5) — set PAE bit
    pm.emit(&0x00000020u32.to_le_bytes());
    pm.mov_cr4_eax();

    // 3. Cargar PML4 en CR3
    pm.mov_eax_imm32(pml4_address);
    pm.mov_cr3_eax();

    // 4. Habilitar Long Mode en EFER MSR
    pm.emit(&[0xB9]); // mov ecx, 0xC0000080 (EFER MSR)
    pm.emit(&0xC0000080u32.to_le_bytes());
    pm.emit(&[0x0F, 0x32]); // rdmsr
    pm.emit(&[0x0D]); // or eax, (1 << 8) — set LME bit
    pm.emit(&0x00000100u32.to_le_bytes());
    pm.emit(&[0x0F, 0x30]); // wrmsr

    // 5. Habilitar paginación (PG bit en CR0)
    pm.mov_eax_cr0();
    pm.emit(&[0x0D]); // or eax, (1 << 31)
    pm.emit(&0x80000000u32.to_le_bytes());
    pm.mov_cr0_eax();

    // 6. Cargar GDT de 64-bit
    pm.lgdt(gdt64_address);

    // 7. Far jump a código 64-bit
    pm.far_jmp(0x08, code64_offset);

    pm.finalize()
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_mode_operand_size() {
        assert_eq!(CpuMode::Real16.default_operand_size(), 2);
        assert_eq!(CpuMode::Protected32.default_operand_size(), 4);
        assert_eq!(CpuMode::Long64.default_operand_size(), 8);
    }

    #[test]
    fn test_gdt_entry_null() {
        let entry = GdtEntry::null();
        assert_eq!(entry.to_bytes(), [0; 8]);
    }

    #[test]
    fn test_gdt_entry_kernel_code() {
        let entry = GdtEntry::kernel_code_64();
        let bytes = entry.to_bytes();
        assert_eq!(bytes[5], 0x9A); // access byte
        assert_eq!(bytes[6], 0xAF); // flags + limit high
    }

    #[test]
    fn test_gdt_generator_standard() {
        let gdt = GdtGenerator::generate_standard_gdt();
        assert_eq!(gdt.entry_count(), 5);
        assert_eq!(gdt.size_bytes(), 40);
    }

    #[test]
    fn test_idt_entry_interrupt_gate() {
        let entry = IdtEntry64::interrupt_gate(0x1000, 0x08, 0, 0);
        let bytes = entry.to_bytes();
        assert_eq!(bytes[0], 0x00); // offset_low low byte
        assert_eq!(bytes[1], 0x10); // offset_low high byte
        assert_eq!(bytes[2], 0x08); // selector low
        assert_eq!(bytes[3], 0x00); // selector high
        assert_eq!(bytes[5], 0x8E); // type_attr: Present + Interrupt Gate
    }

    #[test]
    fn test_paging_identity_map() {
        let mut paging = PagingSetup::new();
        paging.identity_map_2mb(4, 0x2000, 0x3000);

        // PML4[0] should point to PDPT
        assert_ne!(paging.pml4[0], 0);
        assert_eq!(paging.pml4[0] & 0xFFFFF000, 0x2000);

        // PDPT[0] should point to PD
        assert_ne!(paging.pdpt[0], 0);

        // PD should have 2 entries (4MB / 2MB = 2)
        assert_ne!(paging.pd[0], 0);
        assert_ne!(paging.pd[1], 0);
        assert_eq!(paging.pd[2], 0);
    }

    #[test]
    fn test_interrupt_handler_wrap() {
        let gen = InterruptHandlerGen::new(CpuMode::Long64);
        let user_code = vec![0x90]; // NOP
        let handler = gen.wrap_handler(&user_code);

        // Should start with push rax (0x50)
        assert_eq!(handler[0], 0x50);
        // Should end with iretq (0x48, 0xCF)
        let len = handler.len();
        assert_eq!(handler[len - 2], 0x48);
        assert_eq!(handler[len - 1], 0xCF);
    }

    #[test]
    fn test_packed_struct() {
        let mut gdt_entry = PackedStruct::new("GDTEntry");
        gdt_entry.add_field("limit_low", 2);
        gdt_entry.add_field("base_low", 2);
        gdt_entry.add_field("base_mid", 1);
        gdt_entry.add_field("access", 1);
        gdt_entry.add_field("flags_limit", 1);
        gdt_entry.add_field("base_high", 1);

        assert_eq!(gdt_entry.total_size, 8);

        let bytes = gdt_entry.generate_bytes(&[0xFFFF, 0x0000, 0x00, 0x9A, 0xCF, 0x00]);
        assert_eq!(bytes.len(), 8);
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xFF);
        assert_eq!(bytes[5], 0x9A);
    }

    #[test]
    fn test_real_mode_codegen() {
        let mut rm = RealModeCodegen::new();
        rm.cli();
        rm.xor_reg16(0); // xor ax, ax
        rm.sti();
        rm.hlt();

        let code = rm.finalize();
        assert_eq!(code[0], 0xFA); // cli
        assert_eq!(code[3], 0xFB); // sti
        assert_eq!(code[4], 0xF4); // hlt
    }

    #[test]
    fn test_rust_kernel_bridge() {
        let mut bridge = RustKernelBridge::new(CallingConvention::SystemV);
        bridge.export("_start", 0x100000, SymbolType::Function);
        bridge.export("gdt_setup", 0x100100, SymbolType::Function);
        bridge.import("kernel_main", SymbolType::Function);

        let script = bridge.generate_linker_script(0x100000);
        assert!(script.contains("ENTRY(_start)"));
        assert!(script.contains("_start"));

        let header = bridge.generate_c_header();
        assert!(header.contains("gdt_setup"));
        assert!(header.contains("kernel_main"));
    }
}
