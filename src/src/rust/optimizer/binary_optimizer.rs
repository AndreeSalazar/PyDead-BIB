// ADead-BIB - Optimizador de Binarios
// Objetivo: Generar binarios MÁS PEQUEÑOS que ASM tradicional
//
// Técnicas:
// 1. Eliminación de código muerto (Dead Code Elimination)
// 2. Fusión de instrucciones (Instruction Fusion)
// 3. Selección óptima de opcodes (Opcode Selection)
// 4. Eliminación de NOPs y padding innecesario
// 5. Compresión de constantes
// 6. Reutilización de registros
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com

#[allow(unused_imports)]
use std::collections::HashMap;

/// Nivel de optimización
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OptLevel {
    /// Sin optimización (debug)
    None,
    /// Optimización básica (default)
    Basic,
    /// Optimización agresiva (tamaño mínimo)
    Aggressive,
    /// Ultra optimización (puede romper compatibilidad)
    Ultra,
}

/// Optimizador de código binario x86-64
pub struct BinaryOptimizer {
    level: OptLevel,
    stats: OptimizationStats,
}

/// Estadísticas de optimización
#[derive(Default, Clone, Debug)]
pub struct OptimizationStats {
    pub original_size: usize,
    pub optimized_size: usize,
    pub instructions_removed: usize,
    pub bytes_saved: usize,
    pub patterns_applied: Vec<String>,
}

impl BinaryOptimizer {
    pub fn new(level: OptLevel) -> Self {
        Self {
            level,
            stats: OptimizationStats::default(),
        }
    }

    /// Optimiza código x86-64
    pub fn optimize(&mut self, code: &[u8]) -> Vec<u8> {
        self.stats.original_size = code.len();

        let mut optimized = code.to_vec();

        match self.level {
            OptLevel::None => {}
            OptLevel::Basic => {
                optimized = self.remove_nops(&optimized);
                optimized = self.optimize_mov_patterns(&optimized);
            }
            OptLevel::Aggressive => {
                optimized = self.remove_nops(&optimized);
                optimized = self.optimize_mov_patterns(&optimized);
                optimized = self.fuse_instructions(&optimized);
                optimized = self.optimize_jumps(&optimized);
                optimized = self.compress_constants(&optimized);
            }
            OptLevel::Ultra => {
                optimized = self.remove_nops(&optimized);
                optimized = self.optimize_mov_patterns(&optimized);
                optimized = self.fuse_instructions(&optimized);
                optimized = self.optimize_jumps(&optimized);
                optimized = self.compress_constants(&optimized);
                optimized = self.remove_redundant_stack_ops(&optimized);
                optimized = self.use_shorter_encodings(&optimized);
            }
        }

        self.stats.optimized_size = optimized.len();
        self.stats.bytes_saved = self
            .stats
            .original_size
            .saturating_sub(self.stats.optimized_size);

        optimized
    }

    /// Elimina NOPs innecesarios
    fn remove_nops(&mut self, code: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(code.len());
        let mut i = 0;
        let mut nops_removed = 0;

        while i < code.len() {
            // NOP simple (0x90)
            if code[i] == 0x90 {
                // Mantener solo si es necesario para alineación
                nops_removed += 1;
                i += 1;
                continue;
            }

            // NOP largo (66 90)
            if i + 1 < code.len() && code[i] == 0x66 && code[i + 1] == 0x90 {
                nops_removed += 1;
                i += 2;
                continue;
            }

            // NOP de 3 bytes (0F 1F 00)
            if i + 2 < code.len() && code[i] == 0x0F && code[i + 1] == 0x1F && code[i + 2] == 0x00 {
                nops_removed += 1;
                i += 3;
                continue;
            }

            result.push(code[i]);
            i += 1;
        }

        if nops_removed > 0 {
            self.stats.instructions_removed += nops_removed;
            self.stats
                .patterns_applied
                .push(format!("NOP removal: {} removed", nops_removed));
        }

        result
    }

    /// Optimiza patrones de MOV
    fn optimize_mov_patterns(&mut self, code: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(code.len());
        let mut i = 0;
        let mut patterns_applied = 0;

        while i < code.len() {
            // Patrón: mov rax, 0 -> xor eax, eax (7 bytes -> 2 bytes)
            // 48 C7 C0 00 00 00 00 -> 31 C0
            if i + 6 < code.len()
                && code[i] == 0x48
                && code[i + 1] == 0xC7
                && code[i + 2] == 0xC0
                && code[i + 3..i + 7] == [0x00, 0x00, 0x00, 0x00]
            {
                result.extend_from_slice(&[0x31, 0xC0]); // xor eax, eax
                i += 7;
                patterns_applied += 1;
                continue;
            }

            // Patrón: mov rcx, 0 -> xor ecx, ecx
            // 48 C7 C1 00 00 00 00 -> 31 C9
            if i + 6 < code.len()
                && code[i] == 0x48
                && code[i + 1] == 0xC7
                && code[i + 2] == 0xC1
                && code[i + 3..i + 7] == [0x00, 0x00, 0x00, 0x00]
            {
                result.extend_from_slice(&[0x31, 0xC9]); // xor ecx, ecx
                i += 7;
                patterns_applied += 1;
                continue;
            }

            // Patrón: mov rdx, 0 -> xor edx, edx
            if i + 6 < code.len()
                && code[i] == 0x48
                && code[i + 1] == 0xC7
                && code[i + 2] == 0xC2
                && code[i + 3..i + 7] == [0x00, 0x00, 0x00, 0x00]
            {
                result.extend_from_slice(&[0x31, 0xD2]); // xor edx, edx
                i += 7;
                patterns_applied += 1;
                continue;
            }

            // Patrón: mov reg, imm32 pequeño -> usar encoding corto
            // Si el valor cabe en 8 bits, usar push imm8 + pop reg

            result.push(code[i]);
            i += 1;
        }

        if patterns_applied > 0 {
            self.stats
                .patterns_applied
                .push(format!("MOV optimization: {} patterns", patterns_applied));
        }

        result
    }

    /// Fusiona instrucciones cuando es posible
    fn fuse_instructions(&mut self, code: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(code.len());
        let mut i = 0;
        let mut fusions = 0;

        while i < code.len() {
            // Patrón: push rbp; mov rbp, rsp -> enter 0, 0 (más corto en algunos casos)
            // 55 48 89 E5 -> C8 00 00 00 (mismo tamaño, pero más claro)
            // En realidad, push+mov es más rápido, así que lo dejamos

            // Patrón: xor eax, eax; ret -> xor eax, eax; ret (ya óptimo)

            // Patrón: mov rsp, rbp; pop rbp -> leave (3 bytes -> 1 byte)
            // 48 89 EC 5D -> C9
            if i + 3 < code.len()
                && code[i] == 0x48
                && code[i + 1] == 0x89
                && code[i + 2] == 0xEC
                && code[i + 3] == 0x5D
            {
                result.push(0xC9); // leave
                i += 4;
                fusions += 1;
                continue;
            }

            // Patrón: add rsp, 8; pop reg -> pop reg; pop reg (si hay 2 pops)
            // Esto es más complejo, lo dejamos para después

            result.push(code[i]);
            i += 1;
        }

        if fusions > 0 {
            self.stats
                .patterns_applied
                .push(format!("Instruction fusion: {} fusions", fusions));
        }

        result
    }

    /// Optimiza saltos
    fn optimize_jumps(&mut self, code: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(code.len());
        let mut i = 0;
        let mut optimizations = 0;

        while i < code.len() {
            // Patrón: jmp rel32 con offset pequeño -> jmp rel8
            // E9 XX XX XX XX -> EB XX (si offset cabe en 8 bits)
            if i + 4 < code.len() && code[i] == 0xE9 {
                let offset =
                    i32::from_le_bytes([code[i + 1], code[i + 2], code[i + 3], code[i + 4]]);
                if offset >= -128 && offset <= 127 {
                    result.push(0xEB); // jmp rel8
                    result.push((offset as i8) as u8);
                    i += 5;
                    optimizations += 1;
                    continue;
                }
            }

            // Patrón: je/jne rel32 con offset pequeño -> je/jne rel8
            // 0F 84 XX XX XX XX -> 74 XX
            if i + 5 < code.len() && code[i] == 0x0F && code[i + 1] == 0x84 {
                let offset =
                    i32::from_le_bytes([code[i + 2], code[i + 3], code[i + 4], code[i + 5]]);
                if offset >= -128 && offset <= 127 {
                    result.push(0x74); // je rel8
                    result.push((offset as i8) as u8);
                    i += 6;
                    optimizations += 1;
                    continue;
                }
            }

            result.push(code[i]);
            i += 1;
        }

        if optimizations > 0 {
            self.stats.patterns_applied.push(format!(
                "Jump optimization: {} jumps shortened",
                optimizations
            ));
        }

        result
    }

    /// Comprime constantes
    fn compress_constants(&mut self, code: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(code.len());
        let mut i = 0;
        let mut compressions = 0;

        while i < code.len() {
            // Patrón: mov rax, imm64 pequeño -> mov eax, imm32
            // 48 B8 XX XX XX XX 00 00 00 00 -> B8 XX XX XX XX
            if i + 9 < code.len()
                && code[i] == 0x48
                && code[i + 1] == 0xB8
                && code[i + 6..i + 10] == [0x00, 0x00, 0x00, 0x00]
            {
                result.push(0xB8); // mov eax, imm32
                result.extend_from_slice(&code[i + 2..i + 6]);
                i += 10;
                compressions += 1;
                continue;
            }

            // Patrón: sub rsp, imm32 pequeño -> sub rsp, imm8
            // 48 81 EC XX XX XX XX -> 48 83 EC XX (si cabe en 8 bits)
            if i + 6 < code.len() && code[i] == 0x48 && code[i + 1] == 0x81 && code[i + 2] == 0xEC {
                let value =
                    u32::from_le_bytes([code[i + 3], code[i + 4], code[i + 5], code[i + 6]]);
                if value <= 127 {
                    result.extend_from_slice(&[0x48, 0x83, 0xEC, value as u8]);
                    i += 7;
                    compressions += 1;
                    continue;
                }
            }

            result.push(code[i]);
            i += 1;
        }

        if compressions > 0 {
            self.stats
                .patterns_applied
                .push(format!("Constant compression: {} compressed", compressions));
        }

        result
    }

    /// Elimina operaciones de stack redundantes
    fn remove_redundant_stack_ops(&mut self, code: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(code.len());
        let mut i = 0;
        let mut removals = 0;

        while i < code.len() {
            // Patrón: push reg; pop reg (mismo registro) -> nada
            // 5X 5X (donde X es el mismo)
            if i + 1 < code.len() {
                let is_push = code[i] >= 0x50 && code[i] <= 0x57;
                let is_pop = code[i + 1] >= 0x58 && code[i + 1] <= 0x5F;
                if is_push && is_pop && (code[i] - 0x50) == (code[i + 1] - 0x58) {
                    i += 2;
                    removals += 1;
                    continue;
                }
            }

            result.push(code[i]);
            i += 1;
        }

        if removals > 0 {
            self.stats
                .patterns_applied
                .push(format!("Redundant stack ops: {} removed", removals));
        }

        result
    }

    /// Usa encodings más cortos cuando es posible
    fn use_shorter_encodings(&mut self, code: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(code.len());
        let mut i = 0;
        let mut shortenings = 0;

        while i < code.len() {
            // Patrón: test rax, rax -> test eax, eax (si solo nos importa ZF)
            // 48 85 C0 -> 85 C0 (ahorra 1 byte, mismo resultado para ZF)
            if i + 2 < code.len() && code[i] == 0x48 && code[i + 1] == 0x85 && code[i + 2] == 0xC0 {
                result.extend_from_slice(&[0x85, 0xC0]); // test eax, eax
                i += 3;
                shortenings += 1;
                continue;
            }

            // Patrón: inc rax -> inc eax (si el valor cabe en 32 bits)
            // 48 FF C0 -> FF C0 (ahorra 1 byte)
            // Nota: Esto cambia el comportamiento si rax > 0xFFFFFFFF

            result.push(code[i]);
            i += 1;
        }

        if shortenings > 0 {
            self.stats
                .patterns_applied
                .push(format!("Shorter encodings: {} applied", shortenings));
        }

        result
    }

    /// Obtiene las estadísticas de optimización
    pub fn get_stats(&self) -> &OptimizationStats {
        &self.stats
    }

    /// Imprime un resumen de las optimizaciones
    pub fn print_summary(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║           📊 Optimización de Binario                         ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("   Tamaño original:   {} bytes", self.stats.original_size);
        println!("   Tamaño optimizado: {} bytes", self.stats.optimized_size);
        println!(
            "   Bytes ahorrados:   {} bytes ({:.1}%)",
            self.stats.bytes_saved,
            if self.stats.original_size > 0 {
                (self.stats.bytes_saved as f64 / self.stats.original_size as f64) * 100.0
            } else {
                0.0
            }
        );
        println!();
        if !self.stats.patterns_applied.is_empty() {
            println!("   Patrones aplicados:");
            for pattern in &self.stats.patterns_applied {
                println!("     • {}", pattern);
            }
        }
        println!();
    }
}

/// Optimizador de tamaño de PE
pub struct PESizeOptimizer {
    /// Eliminar padding innecesario
    pub strip_padding: bool,
    /// Usar headers mínimos
    pub minimal_headers: bool,
    /// Comprimir secciones
    pub compress_sections: bool,
    /// Eliminar data directories no usados
    pub strip_data_dirs: bool,
}

impl Default for PESizeOptimizer {
    fn default() -> Self {
        Self {
            strip_padding: true,
            minimal_headers: true,
            compress_sections: false,
            strip_data_dirs: true,
        }
    }
}

impl PESizeOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calcula el tamaño mínimo posible para un PE con el código dado
    pub fn calculate_minimum_size(&self, code_size: usize) -> usize {
        // Headers mínimos:
        // - DOS Header: 64 bytes (obligatorio)
        // - PE Signature: 4 bytes
        // - COFF Header: 20 bytes
        // - Optional Header: 240 bytes (PE32+)
        // - Section Header: 40 bytes
        // Total headers: 368 bytes
        //
        // Con FileAlignment de 0x200 (512), headers ocupan 512 bytes
        // Código alineado a 512 bytes

        let header_size = 0x200; // 512 bytes
        let aligned_code = ((code_size + 0x1FF) & !0x1FF).max(0x200);

        header_size + aligned_code
    }

    /// Estima el ahorro comparado con un PE estándar
    pub fn estimate_savings(&self, standard_size: usize, code_size: usize) -> (usize, f64) {
        let minimum = self.calculate_minimum_size(code_size);
        let savings = standard_size.saturating_sub(minimum);
        let percentage = if standard_size > 0 {
            (savings as f64 / standard_size as f64) * 100.0
        } else {
            0.0
        };
        (savings, percentage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nop_removal() {
        let mut opt = BinaryOptimizer::new(OptLevel::Basic);
        let code = vec![0x90, 0x90, 0x31, 0xC0, 0x90, 0xC3];
        let result = opt.optimize(&code);
        assert_eq!(result, vec![0x31, 0xC0, 0xC3]);
    }

    #[test]
    fn test_mov_zero_optimization() {
        let mut opt = BinaryOptimizer::new(OptLevel::Aggressive);
        // mov rax, 0
        let code = vec![0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00];
        let result = opt.optimize(&code);
        // Should become xor eax, eax
        assert_eq!(result, vec![0x31, 0xC0]);
    }

    #[test]
    fn test_leave_fusion() {
        let mut opt = BinaryOptimizer::new(OptLevel::Aggressive);
        // mov rsp, rbp; pop rbp
        let code = vec![0x48, 0x89, 0xEC, 0x5D];
        let result = opt.optimize(&code);
        // Should become leave
        assert_eq!(result, vec![0xC9]);
    }
}
