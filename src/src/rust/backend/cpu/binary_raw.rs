// ============================================================
// 🔥 BINARY RAW - GENERADOR DE BINARIO CRUDO 🔥
// ============================================================
// La técnica MÁS PROHIBIDA: Emitir bytes directamente
// Sin abstracciones, sin capas, PURO HEX AL METAL
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
//
// Este módulo genera código máquina x86-64 DIRECTAMENTE
// como secuencias de bytes (0s y 1s) sin ningún intermediario.
// ============================================================

use std::fs::File;
use std::io::Write;

/// Generador de binario crudo - sin abstracciones
pub struct BinaryRaw {
    /// Bytes crudos del código
    code: Vec<u8>,
    /// Bytes crudos de datos
    data: Vec<u8>,
}

impl BinaryRaw {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            data: Vec::new(),
        }
    }

    // ============================================================
    // EMISIÓN DIRECTA DE BYTES - NIVEL MÁS BAJO POSIBLE
    // ============================================================

    /// Emite un byte crudo
    #[inline(always)]
    pub fn emit_byte(&mut self, byte: u8) {
        self.code.push(byte);
    }

    /// Emite múltiples bytes crudos
    #[inline(always)]
    pub fn emit_bytes(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    /// Emite un i32 en little-endian
    #[inline(always)]
    pub fn emit_i32(&mut self, value: i32) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    /// Emite un u64 en little-endian
    #[inline(always)]
    pub fn emit_u64(&mut self, value: u64) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    // ============================================================
    // 🔥 LOOP ULTRA-CRUDO - LA TÉCNICA PROHIBIDA 🔥
    // ============================================================
    // Genera el loop más eficiente posible:
    // - Solo 8 bytes en el hot path
    // - Sin llamadas a funciones
    // - Sin abstracciones
    // - Bytes directos al metal
    // ============================================================

    /// Genera un loop de contador ultra-optimizado
    /// Código generado:
    /// ```asm
    /// ; Setup (fuera del loop)
    /// mov rcx, [rbp+offset]    ; cargar counter
    /// mov r8, limit            ; cargar límite
    /// cmp rcx, r8              ; verificar si ya terminamos
    /// jge end                  ; saltar si counter >= limit
    ///
    /// ; HOT LOOP - SOLO 8 BYTES!
    /// .loop:
    ///   inc rcx                ; 48 FF C1 (3 bytes)
    ///   cmp rcx, r8            ; 4C 39 C1 (3 bytes)
    ///   jl .loop               ; 7C F8    (2 bytes)
    ///
    /// ; Cleanup
    /// mov [rbp+offset], rcx    ; guardar resultado
    /// ```
    pub fn emit_counter_loop_raw(&mut self, var_offset: i32, limit: i64) {
        // mov rcx, [rbp+offset] - cargar counter inicial
        // REX.W + MOV r64, r/m64 + ModR/M + disp32
        self.emit_bytes(&[0x48, 0x8B, 0x8D]);
        self.emit_i32(var_offset);

        // mov r8, imm64 - cargar límite
        // REX.WB + MOV r64, imm64
        self.emit_bytes(&[0x49, 0xB8]);
        self.emit_u64(limit as u64);

        // cmp rcx, r8 - verificar si ya terminamos
        self.emit_bytes(&[0x4C, 0x39, 0xC1]);

        // jge skip (short jump) - saltar si counter >= limit
        self.emit_bytes(&[0x7D, 0x08]); // salta 8 bytes (el loop)

        // ============ HOT LOOP - 8 BYTES EXACTOS ============
        // inc rcx (3 bytes)
        self.emit_bytes(&[0x48, 0xFF, 0xC1]);

        // cmp rcx, r8 (3 bytes)
        self.emit_bytes(&[0x4C, 0x39, 0xC1]);

        // jl loop_start (2 bytes) - offset = -8
        self.emit_bytes(&[0x7C, 0xF8]);
        // ============ FIN HOT LOOP ============

        // mov [rbp+offset], rcx - guardar resultado
        self.emit_bytes(&[0x48, 0x89, 0x8D]);
        self.emit_i32(var_offset);
    }

    // ============================================================
    // 🔥🔥 LOOP HYPER-CRUDO v2 - AÚN MÁS PROHIBIDO 🔥🔥
    // ============================================================
    // Técnica: Usar LOOP instruction (más compacto pero más lento)
    // O usar LEA para incrementar (a veces más rápido)
    // ============================================================

    /// Loop usando la instrucción LOOP de x86
    /// Más compacto pero potencialmente más lento en CPUs modernas
    pub fn emit_counter_loop_x86_loop(&mut self, var_offset: i32, limit: i64) {
        // mov rcx, limit - RCX es el contador para LOOP
        self.emit_bytes(&[0x48, 0xB9]);
        self.emit_u64(limit as u64);

        // mov rax, 0 - contador real
        self.emit_bytes(&[0x48, 0x31, 0xC0]); // xor rax, rax

        // .loop:
        // inc rax (3 bytes)
        self.emit_bytes(&[0x48, 0xFF, 0xC0]);

        // loop .loop (2 bytes) - decrementa RCX y salta si RCX != 0
        self.emit_bytes(&[0xE2, 0xFB]); // loop -5

        // mov [rbp+offset], rax - guardar resultado
        self.emit_bytes(&[0x48, 0x89, 0x85]);
        self.emit_i32(var_offset);
    }

    // ============================================================
    // 🔥🔥🔥 LOOP NUCLEAR - LA TÉCNICA MÁS PROHIBIDA 🔥🔥🔥
    // ============================================================
    // Técnica: Unroll parcial + predicción de branch optimizada
    // Desenrollamos 4 iteraciones para reducir overhead de branch
    // ============================================================

    /// Loop con unrolling de 4 iteraciones
    /// Reduce el overhead de branch prediction
    pub fn emit_counter_loop_unrolled(&mut self, var_offset: i32, limit: i64) {
        // mov rcx, [rbp+offset] - cargar counter
        self.emit_bytes(&[0x48, 0x8B, 0x8D]);
        self.emit_i32(var_offset);

        // mov r8, limit
        self.emit_bytes(&[0x49, 0xB8]);
        self.emit_u64(limit as u64);

        // Calcular cuántas iteraciones completas de 4
        // mov r9, r8
        self.emit_bytes(&[0x4D, 0x89, 0xC1]);
        // sub r9, rcx (iteraciones restantes)
        self.emit_bytes(&[0x49, 0x29, 0xC9]);
        // shr r9, 2 (dividir por 4)
        self.emit_bytes(&[0x49, 0xC1, 0xE9, 0x02]);

        // test r9, r9
        self.emit_bytes(&[0x4D, 0x85, 0xC9]);
        // jz remainder
        self.emit_bytes(&[0x74]);
        let jz_pos = self.code.len();
        self.emit_byte(0x00); // placeholder

        // ============ LOOP UNROLLED x4 ============
        let loop_start = self.code.len();

        // add rcx, 4 (una sola instrucción para 4 incrementos!)
        self.emit_bytes(&[0x48, 0x83, 0xC1, 0x04]);

        // dec r9
        self.emit_bytes(&[0x49, 0xFF, 0xC9]);

        // jnz loop_start
        let offset = (loop_start as i64 - self.code.len() as i64 - 2) as i8;
        self.emit_bytes(&[0x75, offset as u8]);

        // Parchear jz
        let remainder_pos = self.code.len();
        self.code[jz_pos] = (remainder_pos - jz_pos - 1) as u8;

        // ============ REMAINDER LOOP ============
        // cmp rcx, r8
        self.emit_bytes(&[0x4C, 0x39, 0xC1]);
        // jge end
        self.emit_bytes(&[0x7D, 0x08]);

        // inc rcx
        self.emit_bytes(&[0x48, 0xFF, 0xC1]);
        // cmp rcx, r8
        self.emit_bytes(&[0x4C, 0x39, 0xC1]);
        // jl remainder_loop
        self.emit_bytes(&[0x7C, 0xF8]);

        // mov [rbp+offset], rcx
        self.emit_bytes(&[0x48, 0x89, 0x8D]);
        self.emit_i32(var_offset);
    }

    // ============================================================
    // GENERACIÓN DE PE CRUDO
    // ============================================================

    /// Genera un ejecutable PE mínimo con el código
    pub fn generate_pe(&self, entry_code: &[u8]) -> Vec<u8> {
        let mut pe = Vec::new();

        // DOS Header
        pe.extend_from_slice(&[0x4D, 0x5A]); // MZ
        pe.extend_from_slice(&[0x90; 58]); // padding
        pe.extend_from_slice(&[0x40, 0x00, 0x00, 0x00]); // e_lfanew = 0x40

        // PE Signature
        pe.extend_from_slice(b"PE\0\0");

        // COFF Header
        pe.extend_from_slice(&0x8664u16.to_le_bytes()); // Machine: x64
        pe.extend_from_slice(&0x0001u16.to_le_bytes()); // NumberOfSections
        pe.extend_from_slice(&[0; 12]); // Timestamp, etc
        pe.extend_from_slice(&0x00F0u16.to_le_bytes()); // SizeOfOptionalHeader
        pe.extend_from_slice(&0x0022u16.to_le_bytes()); // Characteristics

        // Optional Header
        let mut opt = [0u8; 240];
        opt[0..2].copy_from_slice(&0x020Bu16.to_le_bytes()); // PE32+
        opt[16..20].copy_from_slice(&0x1000u32.to_le_bytes()); // AddressOfEntryPoint
        opt[24..32].copy_from_slice(&0x0000000140000000u64.to_le_bytes()); // ImageBase
        opt[32..36].copy_from_slice(&0x1000u32.to_le_bytes()); // SectionAlignment
        opt[36..40].copy_from_slice(&0x0200u32.to_le_bytes()); // FileAlignment
        opt[40..42].copy_from_slice(&6u16.to_le_bytes()); // MajorOSVersion
        opt[48..50].copy_from_slice(&6u16.to_le_bytes()); // MajorSubsystemVersion
        opt[56..60].copy_from_slice(&0x3000u32.to_le_bytes()); // SizeOfImage
        opt[60..64].copy_from_slice(&0x0200u32.to_le_bytes()); // SizeOfHeaders
        opt[68..70].copy_from_slice(&3u16.to_le_bytes()); // Subsystem: Console
        opt[108..112].copy_from_slice(&16u32.to_le_bytes()); // NumberOfRvaAndSizes
        pe.extend_from_slice(&opt);

        // Section Header (.text)
        let mut section = [0u8; 40];
        section[0..8].copy_from_slice(b".text\0\0\0");
        let code_size = entry_code.len() as u32;
        section[8..12].copy_from_slice(&code_size.to_le_bytes()); // VirtualSize
        section[12..16].copy_from_slice(&0x1000u32.to_le_bytes()); // VirtualAddress
        let raw_size = ((code_size + 0x1FF) & !0x1FF) as u32;
        section[16..20].copy_from_slice(&raw_size.to_le_bytes()); // SizeOfRawData
        section[20..24].copy_from_slice(&0x0200u32.to_le_bytes()); // PointerToRawData
        section[36..40].copy_from_slice(&0x60000020u32.to_le_bytes()); // Characteristics
        pe.extend_from_slice(&section);

        // Padding hasta 0x200
        while pe.len() < 0x200 {
            pe.push(0);
        }

        // Code section
        pe.extend_from_slice(entry_code);

        // Padding hasta múltiplo de 0x200
        while pe.len() % 0x200 != 0 {
            pe.push(0);
        }

        pe
    }

    /// Escribe el PE a un archivo
    pub fn write_pe(&self, path: &str, entry_code: &[u8]) -> std::io::Result<()> {
        let pe = self.generate_pe(entry_code);
        let mut file = File::create(path)?;
        file.write_all(&pe)?;
        Ok(())
    }

    /// Obtiene el código generado
    pub fn get_code(&self) -> &[u8] {
        &self.code
    }

    /// Obtiene el tamaño del código
    pub fn code_size(&self) -> usize {
        self.code.len()
    }
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_bytes() {
        let mut gen = BinaryRaw::new();
        gen.emit_bytes(&[0x48, 0xFF, 0xC1]); // inc rcx
        assert_eq!(gen.get_code(), &[0x48, 0xFF, 0xC1]);
    }

    #[test]
    fn test_counter_loop_size() {
        let mut gen = BinaryRaw::new();
        gen.emit_counter_loop_raw(-8, 1000000000);
        // El loop debería ser compacto
        println!("Loop size: {} bytes", gen.code_size());
        assert!(gen.code_size() < 50); // Menos de 50 bytes total
    }
}
