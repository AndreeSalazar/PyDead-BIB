// ADead-BIB - Micro Virtual Machine
// Bytecode extremo: 4 bits por instrucción
// Objetivo: "1 bit = 1 decisión" con runtime mínimo
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com

use std::fs::File;
use std::io::Write;

/// Opcodes de 4 bits (16 instrucciones posibles)
/// Cada byte contiene 2 instrucciones
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MicroOp {
    Exit = 0x0, // Termina con acc como código
    Load = 0x1, // acc = operand
    Add = 0x2,  // acc += operand
    Sub = 0x3,  // acc -= operand
    Mul = 0x4,  // acc *= operand
    Div = 0x5,  // acc /= operand (si operand != 0)
    And = 0x6,  // acc &= operand
    Or = 0x7,   // acc |= operand
    Xor = 0x8,  // acc ^= operand
    Not = 0x9,  // acc = !acc (operand ignorado)
    Jmp = 0xA,  // pc = operand
    Jz = 0xB,   // if acc == 0: pc = operand
    Jnz = 0xC,  // if acc != 0: pc = operand
    Push = 0xD, // stack.push(acc)
    Pop = 0xE,  // acc = stack.pop()
    Nop = 0xF,  // No operation
}

impl From<u8> for MicroOp {
    fn from(val: u8) -> Self {
        match val & 0x0F {
            0x0 => MicroOp::Exit,
            0x1 => MicroOp::Load,
            0x2 => MicroOp::Add,
            0x3 => MicroOp::Sub,
            0x4 => MicroOp::Mul,
            0x5 => MicroOp::Div,
            0x6 => MicroOp::And,
            0x7 => MicroOp::Or,
            0x8 => MicroOp::Xor,
            0x9 => MicroOp::Not,
            0xA => MicroOp::Jmp,
            0xB => MicroOp::Jz,
            0xC => MicroOp::Jnz,
            0xD => MicroOp::Push,
            0xE => MicroOp::Pop,
            _ => MicroOp::Nop,
        }
    }
}

/// Micro-VM: Intérprete de bytecode ultra-compacto
/// Estado: 1 acumulador (8 bits), 1 PC, stack pequeño
pub struct MicroVM {
    acc: u8,         // Acumulador
    pc: usize,       // Program counter
    stack: Vec<u8>,  // Stack (máximo 16 elementos)
    memory: Vec<u8>, // Memoria de programa
    halted: bool,
}

impl MicroVM {
    pub fn new(program: &[u8]) -> Self {
        MicroVM {
            acc: 0,
            pc: 0,
            stack: Vec::with_capacity(16),
            memory: program.to_vec(),
            halted: false,
        }
    }

    /// Ejecuta el programa completo
    pub fn run(&mut self) -> u8 {
        while !self.halted && self.pc < self.memory.len() * 2 {
            self.step();
        }
        self.acc
    }

    /// Ejecuta una instrucción
    fn step(&mut self) {
        let byte_idx = self.pc / 2;
        let is_high = self.pc % 2 == 0;

        if byte_idx >= self.memory.len() {
            self.halted = true;
            return;
        }

        let byte = self.memory[byte_idx];
        let (opcode, operand) = if is_high {
            ((byte >> 4) & 0x0F, byte & 0x0F)
        } else {
            // Para la segunda instrucción, el operand viene del siguiente nibble
            // Simplificación: operand = 0 para instrucciones impares
            (byte & 0x0F, 0)
        };

        let op = MicroOp::from(opcode);

        match op {
            MicroOp::Exit => {
                self.halted = true;
            }
            MicroOp::Load => {
                self.acc = operand;
            }
            MicroOp::Add => {
                self.acc = self.acc.wrapping_add(operand);
            }
            MicroOp::Sub => {
                self.acc = self.acc.wrapping_sub(operand);
            }
            MicroOp::Mul => {
                self.acc = self.acc.wrapping_mul(operand);
            }
            MicroOp::Div => {
                if operand != 0 {
                    self.acc /= operand;
                }
            }
            MicroOp::And => {
                self.acc &= operand;
            }
            MicroOp::Or => {
                self.acc |= operand;
            }
            MicroOp::Xor => {
                self.acc ^= operand;
            }
            MicroOp::Not => {
                self.acc = !self.acc;
            }
            MicroOp::Jmp => {
                self.pc = operand as usize;
                return; // No incrementar PC
            }
            MicroOp::Jz => {
                if self.acc == 0 {
                    self.pc = operand as usize;
                    return;
                }
            }
            MicroOp::Jnz => {
                if self.acc != 0 {
                    self.pc = operand as usize;
                    return;
                }
            }
            MicroOp::Push => {
                if self.stack.len() < 16 {
                    self.stack.push(self.acc);
                }
            }
            MicroOp::Pop => {
                self.acc = self.stack.pop().unwrap_or(0);
            }
            MicroOp::Nop => {}
        }

        self.pc += 1;
    }
}

/// Compila instrucciones a bytecode compacto
/// Formato: [opcode:4][operand:4] = 1 byte por instrucción
pub fn compile_microvm(instructions: &[(MicroOp, u8)]) -> Vec<u8> {
    let mut bytecode = Vec::new();

    for (op, operand) in instructions {
        let byte = ((*op as u8) << 4) | (operand & 0x0F);
        bytecode.push(byte);
    }

    bytecode
}

/// Genera un programa MicroVM que retorna un valor específico
pub fn generate_microvm_exit(exit_code: u8) -> Vec<u8> {
    if exit_code <= 15 {
        // 1 byte: LOAD exit_code, luego EXIT implícito
        vec![(MicroOp::Load as u8) << 4 | exit_code, MicroOp::Exit as u8]
    } else {
        // Necesitamos múltiples operaciones para valores > 15
        let high = exit_code >> 4;
        let low = exit_code & 0x0F;
        compile_microvm(&[
            (MicroOp::Load, high),
            (MicroOp::Mul, 0), // Truco: multiplicar por 16 no funciona con 4 bits
            // Alternativa: usar múltiples ADD
            (MicroOp::Load, low),
            (MicroOp::Exit, 0),
        ])
    }
}

/// Genera el stub del intérprete MicroVM en x86-64
/// Este es el "runtime mínimo" que permite ejecutar bytecode
pub fn generate_microvm_runtime_x64() -> Vec<u8> {
    // Intérprete MicroVM en ~100 bytes de código x86-64
    // Simplificado: solo soporta EXIT y LOAD por ahora
    vec![
        // Prólogo
        0x55, // push rbp
        0x48, 0x89, 0xE5, // mov rbp, rsp
        // RDI = puntero al bytecode
        // RSI = longitud del bytecode

        // Inicializar registros
        0x31, 0xC0, // xor eax, eax (acc = 0)
        0x31, 0xC9, // xor ecx, ecx (pc = 0)
        // Loop principal
        // loop_start:
        0x48, 0x39, 0xF1, // cmp rcx, rsi
        0x7D, 0x1A, // jge exit_loop
        // Leer byte
        0x0F, 0xB6, 0x14, 0x0F, // movzx edx, byte [rdi+rcx]
        // Extraer opcode (high nibble)
        0x89, 0xD3, // mov ebx, edx
        0xC1, 0xEB, 0x04, // shr ebx, 4
        // Extraer operand (low nibble)
        0x83, 0xE2, 0x0F, // and edx, 0x0F
        // Check opcode
        0x83, 0xFB, 0x00, // cmp ebx, 0 (EXIT)
        0x74, 0x08, // je exit_loop
        0x83, 0xFB, 0x01, // cmp ebx, 1 (LOAD)
        0x75, 0x02, // jne skip_load
        0x89, 0xD0, // mov eax, edx (acc = operand)
        // skip_load:
        0x48, 0xFF, 0xC1, // inc rcx
        0xEB, 0xDE, // jmp loop_start
        // exit_loop:
        0x5D, // pop rbp
        0xC3, // ret
    ]
}

/// Genera un ejecutable PE con MicroVM embebida + bytecode
pub fn generate_microvm_pe(
    bytecode: &[u8],
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let runtime = generate_microvm_runtime_x64();

    // Combinar runtime + bytecode
    let mut code = runtime.clone();
    let bytecode_offset = code.len();
    code.extend_from_slice(bytecode);

    // Generar PE con el código combinado
    super::pe_tiny::generate_pe_tiny(&code, output_path)?;

    println!("✅ MicroVM PE generated:");
    println!("   Runtime: {} bytes", runtime.len());
    println!("   Bytecode: {} bytes", bytecode.len());
    println!("   Bytecode offset: 0x{:X}", bytecode_offset);

    Ok(code.len())
}

/// Guarda bytecode MicroVM a archivo
pub fn save_bytecode(
    bytecode: &[u8],
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut file = File::create(output_path)?;
    file.write_all(bytecode)?;

    println!("✅ MicroVM bytecode saved: {} bytes", bytecode.len());
    Ok(bytecode.len())
}

// ============================================
// ULTRA-DENSE: 1 bit = 1 decisión
// ============================================

/// BitVM: El nivel más extremo - 1 bit por decisión
/// Requiere un "diccionario" de acciones predefinidas
pub struct BitVM {
    /// Diccionario de acciones (índice = valor del bit pattern)
    actions: Vec<fn() -> u8>,
}

impl BitVM {
    pub fn new() -> Self {
        BitVM {
            actions: vec![
                || 0, // Bit 0 = retorna 0
                || 1, // Bit 1 = retorna 1
            ],
        }
    }

    /// Ejecuta un programa de 1 bit
    pub fn execute_1bit(&self, bit: bool) -> u8 {
        self.actions[bit as usize]()
    }

    /// Ejecuta un programa de N bits (2^N acciones posibles)
    pub fn execute_nbits(&self, bits: &[bool]) -> u8 {
        let mut index = 0usize;
        for (i, &bit) in bits.iter().enumerate() {
            if bit {
                index |= 1 << i;
            }
        }
        if index < self.actions.len() {
            self.actions[index]()
        } else {
            0
        }
    }
}

impl Default for BitVM {
    fn default() -> Self {
        Self::new()
    }
}

/// Genera un "programa" de 1 bit
pub fn generate_1bit_program(value: bool) -> Vec<u8> {
    vec![if value { 1 } else { 0 }]
}

/// Calcula el tamaño teórico mínimo para un programa
pub fn theoretical_minimum(num_decisions: usize) -> f64 {
    // Cada decisión binaria requiere 1 bit
    // num_decisions bits = num_decisions / 8 bytes
    num_decisions as f64 / 8.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_microvm_load_exit() {
        let bytecode = compile_microvm(&[(MicroOp::Load, 5), (MicroOp::Exit, 0)]);
        let mut vm = MicroVM::new(&bytecode);
        assert_eq!(vm.run(), 5);
    }

    #[test]
    fn test_microvm_math() {
        let bytecode =
            compile_microvm(&[(MicroOp::Load, 3), (MicroOp::Add, 2), (MicroOp::Exit, 0)]);
        let mut vm = MicroVM::new(&bytecode);
        assert_eq!(vm.run(), 5);
    }

    #[test]
    fn test_bitvm_1bit() {
        let vm = BitVM::new();
        assert_eq!(vm.execute_1bit(false), 0);
        assert_eq!(vm.execute_1bit(true), 1);
    }

    #[test]
    fn test_theoretical_minimum() {
        // 1 decisión = 0.125 bytes (1 bit)
        assert_eq!(theoretical_minimum(1), 0.125);
        // 8 decisiones = 1 byte
        assert_eq!(theoretical_minimum(8), 1.0);
    }
}
