// ADead-BIB - Bytecode → SPIR-V Compiler
// Puente directo: ADead Bytecode → SPIR-V IR → GPU
// Sin GLSL, sin HLSL - código en bits ejecutado en GPU
//
// Esto es lo que hace ÚNICO a ADead-BIB:
// "Escribir lógica en bits, ejecutarla en GPU"
//
// Autor: Eddi Andreé Salazar Matos

use super::super::vulkan::{ExecutionModel, SpirVOp, VulkanCapability};

/// Opcodes ADead para GPU (4 bits = 16 instrucciones)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ADeadGpuOp {
    // Terminación
    Exit = 0x0,

    // Carga/Almacenamiento
    Load = 0x1,    // acc = mem[operand]
    Store = 0x2,   // mem[operand] = acc
    LoadImm = 0x3, // acc = operand (inmediato)

    // Aritmética
    Add = 0x4, // acc += mem[operand]
    Sub = 0x5, // acc -= mem[operand]
    Mul = 0x6, // acc *= mem[operand]
    Div = 0x7, // acc /= mem[operand]

    // Vectorial/Matricial
    VecAdd = 0x8, // vec_acc += vec[operand]
    VecMul = 0x9, // vec_acc *= vec[operand]
    Dot = 0xA,    // acc = dot(vec_acc, vec[operand])
    MatMul = 0xB, // mat_acc = mat_acc * mat[operand]

    // Control (mínimo)
    Sync = 0xC, // barrier
    Nop = 0xD,

    // Reservados
    Reserved1 = 0xE,
    Reserved2 = 0xF,
}

impl From<u8> for ADeadGpuOp {
    fn from(val: u8) -> Self {
        match val & 0x0F {
            0x0 => ADeadGpuOp::Exit,
            0x1 => ADeadGpuOp::Load,
            0x2 => ADeadGpuOp::Store,
            0x3 => ADeadGpuOp::LoadImm,
            0x4 => ADeadGpuOp::Add,
            0x5 => ADeadGpuOp::Sub,
            0x6 => ADeadGpuOp::Mul,
            0x7 => ADeadGpuOp::Div,
            0x8 => ADeadGpuOp::VecAdd,
            0x9 => ADeadGpuOp::VecMul,
            0xA => ADeadGpuOp::Dot,
            0xB => ADeadGpuOp::MatMul,
            0xC => ADeadGpuOp::Sync,
            _ => ADeadGpuOp::Nop,
        }
    }
}

/// Instrucción ADead GPU
#[derive(Debug, Clone, Copy)]
pub struct ADeadGpuInstr {
    pub opcode: ADeadGpuOp,
    pub operand: u8,
}

impl ADeadGpuInstr {
    pub fn new(opcode: ADeadGpuOp, operand: u8) -> Self {
        ADeadGpuInstr { opcode, operand }
    }

    /// Decodifica desde byte (4 bits opcode + 4 bits operand)
    pub fn from_byte(byte: u8) -> Self {
        ADeadGpuInstr {
            opcode: ADeadGpuOp::from(byte >> 4),
            operand: byte & 0x0F,
        }
    }

    /// Codifica a byte
    pub fn to_byte(&self) -> u8 {
        ((self.opcode as u8) << 4) | (self.operand & 0x0F)
    }
}

/// Compilador ADead Bytecode → SPIR-V
pub struct BytecodeToSpirV {
    /// Instrucciones SPIR-V generadas
    instructions: Vec<u32>,
    /// Siguiente ID
    next_id: u32,
    /// IDs de tipos
    type_void: u32,
    type_float: u32,
    type_uint: u32,
    type_vec4: u32,
    type_ptr_storage: u32,
    type_ptr_float: u32,
    type_func: u32,
    /// IDs de variables
    var_acc: u32, // Acumulador escalar
    var_vec_acc: u32, // Acumulador vectorial
    var_input: u32,   // Buffer de entrada
    var_output: u32,  // Buffer de salida
    var_global_id: u32,
    /// Workgroup size
    workgroup_size: (u32, u32, u32),
}

impl BytecodeToSpirV {
    pub fn new() -> Self {
        BytecodeToSpirV {
            instructions: Vec::new(),
            next_id: 1,
            type_void: 0,
            type_float: 0,
            type_uint: 0,
            type_vec4: 0,
            type_ptr_storage: 0,
            type_ptr_float: 0,
            type_func: 0,
            var_acc: 0,
            var_vec_acc: 0,
            var_input: 0,
            var_output: 0,
            var_global_id: 0,
            workgroup_size: (256, 1, 1),
        }
    }

    pub fn set_workgroup_size(&mut self, x: u32, y: u32, z: u32) {
        self.workgroup_size = (x, y, z);
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn emit(&mut self, op: SpirVOp, operands: &[u32]) {
        let word_count = (1 + operands.len()) as u32;
        self.instructions.push((word_count << 16) | (op as u32));
        self.instructions.extend_from_slice(operands);
    }

    fn emit_result(&mut self, op: SpirVOp, result_type: u32, operands: &[u32]) -> u32 {
        let result_id = self.alloc_id();
        let word_count = (3 + operands.len()) as u32;
        self.instructions.push((word_count << 16) | (op as u32));
        self.instructions.push(result_type);
        self.instructions.push(result_id);
        self.instructions.extend_from_slice(operands);
        result_id
    }

    /// Genera header SPIR-V
    fn generate_header(&self) -> Vec<u32> {
        vec![
            0x07230203, // Magic
            0x00010500, // Version 1.5
            0x00080001, // Generator: ADead-BIB Bytecode Compiler
            self.next_id,
            0,
        ]
    }

    /// Emite preámbulo (capabilities, types, variables)
    fn emit_preamble(&mut self) {
        // Capabilities
        self.emit(SpirVOp::OpCapability, &[VulkanCapability::Shader as u32]);

        // Memory model
        self.emit(SpirVOp::OpMemoryModel, &[0, 1]); // Logical GLSL450

        // Types
        self.type_void = self.alloc_id();
        self.emit(SpirVOp::OpTypeVoid, &[self.type_void]);

        self.type_float = self.alloc_id();
        self.emit(SpirVOp::OpTypeFloat, &[self.type_float, 32]);

        self.type_uint = self.alloc_id();
        self.emit(SpirVOp::OpTypeInt, &[self.type_uint, 32, 0]);

        self.type_vec4 = self.alloc_id();
        self.emit(SpirVOp::OpTypeVector, &[self.type_vec4, self.type_float, 4]);

        // Array type para buffers
        let array_size_id = self.alloc_id();
        self.emit(SpirVOp::OpConstant, &[self.type_uint, array_size_id, 1024]);

        let array_type = self.alloc_id();
        self.emit(
            SpirVOp::OpTypeArray,
            &[array_type, self.type_float, array_size_id],
        );

        // Struct para buffer
        let struct_type = self.alloc_id();
        self.emit(SpirVOp::OpTypeStruct, &[struct_type, array_type]);

        // Pointer types
        self.type_ptr_storage = self.alloc_id();
        self.emit(
            SpirVOp::OpTypePointer,
            &[self.type_ptr_storage, 12, struct_type],
        );

        self.type_ptr_float = self.alloc_id();
        self.emit(
            SpirVOp::OpTypePointer,
            &[self.type_ptr_float, 12, self.type_float],
        );

        let ptr_func_float = self.alloc_id();
        self.emit(
            SpirVOp::OpTypePointer,
            &[ptr_func_float, 7, self.type_float],
        ); // Function

        // Function type
        self.type_func = self.alloc_id();
        self.emit(SpirVOp::OpTypeFunction, &[self.type_func, self.type_void]);

        // Variables globales
        self.var_input = self.alloc_id();
        self.emit(
            SpirVOp::OpVariable,
            &[self.type_ptr_storage, self.var_input, 12],
        );

        self.var_output = self.alloc_id();
        self.emit(
            SpirVOp::OpVariable,
            &[self.type_ptr_storage, self.var_output, 12],
        );

        // GlobalInvocationId
        let uvec3_type = self.alloc_id();
        self.emit(SpirVOp::OpTypeVector, &[uvec3_type, self.type_uint, 3]);

        let ptr_uvec3 = self.alloc_id();
        self.emit(SpirVOp::OpTypePointer, &[ptr_uvec3, 1, uvec3_type]);

        self.var_global_id = self.alloc_id();
        self.emit(SpirVOp::OpVariable, &[ptr_uvec3, self.var_global_id, 1]);

        // Decorations
        self.emit(SpirVOp::OpDecorate, &[self.var_input, 34, 0]); // DescriptorSet 0
        self.emit(SpirVOp::OpDecorate, &[self.var_input, 33, 0]); // Binding 0
        self.emit(SpirVOp::OpDecorate, &[self.var_output, 34, 0]);
        self.emit(SpirVOp::OpDecorate, &[self.var_output, 33, 1]);
        self.emit(SpirVOp::OpDecorate, &[self.var_global_id, 11, 28]); // BuiltIn GlobalInvocationId
    }

    /// Compila bytecode ADead a SPIR-V
    pub fn compile(&mut self, bytecode: &[u8]) -> Vec<u8> {
        self.instructions.clear();
        self.next_id = 1;

        // Emitir preámbulo
        self.emit_preamble();

        // Entry point
        let main_func = self.alloc_id();
        self.emit(
            SpirVOp::OpEntryPoint,
            &[
                ExecutionModel::GLCompute as u32,
                main_func,
                0x6E69616D,
                0x00000000, // "main"
                self.var_global_id,
                self.var_input,
                self.var_output,
            ],
        );

        // Execution mode
        self.emit(
            SpirVOp::OpExecutionMode,
            &[
                main_func,
                17,
                self.workgroup_size.0,
                self.workgroup_size.1,
                self.workgroup_size.2,
            ],
        );

        // Main function
        self.emit(
            SpirVOp::OpFunction,
            &[self.type_void, main_func, 0, self.type_func],
        );

        let label = self.alloc_id();
        self.emit(SpirVOp::OpLabel, &[label]);

        // Cargar global ID
        let uvec3_type = self.type_uint; // Simplificado
        let gid = self.emit_result(SpirVOp::OpLoad, uvec3_type, &[self.var_global_id]);

        // Constantes
        let const_0 = self.alloc_id();
        self.emit(SpirVOp::OpConstant, &[self.type_uint, const_0, 0]);

        // Variable local para acumulador
        let acc = self.alloc_id();
        let const_zero_f = self.alloc_id();
        self.emit(SpirVOp::OpConstant, &[self.type_float, const_zero_f, 0]);

        // Compilar cada instrucción del bytecode
        for &byte in bytecode {
            let instr = ADeadGpuInstr::from_byte(byte);
            self.compile_instruction(&instr, gid, const_0, acc);
        }

        // Return
        self.emit(SpirVOp::OpReturn, &[]);
        self.emit(SpirVOp::OpFunctionEnd, &[]);

        // Build final SPIR-V
        let mut spirv = self.generate_header();
        spirv.extend_from_slice(&self.instructions);

        spirv.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// Compila una instrucción individual
    fn compile_instruction(&mut self, instr: &ADeadGpuInstr, gid: u32, const_0: u32, _acc: u32) {
        match instr.opcode {
            ADeadGpuOp::Exit => {
                // No-op en GPU (el shader termina naturalmente)
            }
            ADeadGpuOp::Load => {
                // acc = input[gid + operand]
                let offset = self.alloc_id();
                self.emit(
                    SpirVOp::OpConstant,
                    &[self.type_uint, offset, instr.operand as u32],
                );

                let idx = self.emit_result(SpirVOp::OpIAdd, self.type_uint, &[gid, offset]);
                let ptr = self.emit_result(
                    SpirVOp::OpAccessChain,
                    self.type_ptr_float,
                    &[self.var_input, const_0, idx],
                );
                let _val = self.emit_result(SpirVOp::OpLoad, self.type_float, &[ptr]);
            }
            ADeadGpuOp::Store => {
                // output[gid + operand] = acc
                let offset = self.alloc_id();
                self.emit(
                    SpirVOp::OpConstant,
                    &[self.type_uint, offset, instr.operand as u32],
                );

                let idx = self.emit_result(SpirVOp::OpIAdd, self.type_uint, &[gid, offset]);
                let ptr = self.emit_result(
                    SpirVOp::OpAccessChain,
                    self.type_ptr_float,
                    &[self.var_output, const_0, idx],
                );

                // Store zero (placeholder - necesita valor real)
                let zero = self.alloc_id();
                self.emit(SpirVOp::OpConstant, &[self.type_float, zero, 0]);
                self.emit(SpirVOp::OpStore, &[ptr, zero]);
            }
            ADeadGpuOp::LoadImm => {
                // acc = operand (como float)
                let _val = self.alloc_id();
                self.emit(
                    SpirVOp::OpConstant,
                    &[self.type_float, _val, instr.operand as u32],
                );
            }
            ADeadGpuOp::Add | ADeadGpuOp::Sub | ADeadGpuOp::Mul | ADeadGpuOp::Div => {
                // Operaciones aritméticas
                let offset = self.alloc_id();
                self.emit(
                    SpirVOp::OpConstant,
                    &[self.type_uint, offset, instr.operand as u32],
                );

                let idx = self.emit_result(SpirVOp::OpIAdd, self.type_uint, &[gid, offset]);
                let ptr = self.emit_result(
                    SpirVOp::OpAccessChain,
                    self.type_ptr_float,
                    &[self.var_input, const_0, idx],
                );
                let _val = self.emit_result(SpirVOp::OpLoad, self.type_float, &[ptr]);

                // La operación real se haría aquí con el acumulador
            }
            ADeadGpuOp::Sync => {
                // Barrier (simplificado)
                // OpControlBarrier requiere más setup
            }
            _ => {
                // Nop y otros
            }
        }
    }
}

impl Default for BytecodeToSpirV {
    fn default() -> Self {
        Self::new()
    }
}

/// Genera bytecode ADead para una operación simple
pub fn generate_adead_gpu_bytecode(ops: &[(ADeadGpuOp, u8)]) -> Vec<u8> {
    ops.iter()
        .map(|(op, operand)| ADeadGpuInstr::new(*op, *operand).to_byte())
        .collect()
}

/// Programa de ejemplo: C[i] = A[i] + B[i]
pub fn example_vector_add() -> Vec<u8> {
    generate_adead_gpu_bytecode(&[
        (ADeadGpuOp::Load, 0),  // acc = A[gid]
        (ADeadGpuOp::Add, 1),   // acc += B[gid] (offset 1 = segundo buffer)
        (ADeadGpuOp::Store, 0), // C[gid] = acc
        (ADeadGpuOp::Exit, 0),
    ])
}

/// Programa de ejemplo: C[i] = A[i] * B[i]
pub fn example_vector_mul() -> Vec<u8> {
    generate_adead_gpu_bytecode(&[
        (ADeadGpuOp::Load, 0),
        (ADeadGpuOp::Mul, 1),
        (ADeadGpuOp::Store, 0),
        (ADeadGpuOp::Exit, 0),
    ])
}

/// Programa de ejemplo: MatMul simplificado
pub fn example_matmul() -> Vec<u8> {
    generate_adead_gpu_bytecode(&[
        (ADeadGpuOp::Load, 0),   // Load A
        (ADeadGpuOp::MatMul, 1), // MatMul with B
        (ADeadGpuOp::Store, 0),  // Store to C
        (ADeadGpuOp::Exit, 0),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_encoding() {
        let instr = ADeadGpuInstr::new(ADeadGpuOp::Load, 5);
        let byte = instr.to_byte();
        let decoded = ADeadGpuInstr::from_byte(byte);

        assert_eq!(decoded.opcode, ADeadGpuOp::Load);
        assert_eq!(decoded.operand, 5);
    }

    #[test]
    fn test_bytecode_generation() {
        let bytecode = example_vector_add();
        assert_eq!(bytecode.len(), 4);

        // Verificar primera instrucción (Load, 0)
        let first = ADeadGpuInstr::from_byte(bytecode[0]);
        assert_eq!(first.opcode, ADeadGpuOp::Load);
        assert_eq!(first.operand, 0);
    }

    #[test]
    fn test_compile_to_spirv() {
        let bytecode = example_vector_add();
        let mut compiler = BytecodeToSpirV::new();
        let spirv = compiler.compile(&bytecode);

        // Verificar magic number
        assert_eq!(&spirv[0..4], &[0x03, 0x02, 0x23, 0x07]);
        assert!(spirv.len() > 100); // Debe tener contenido
    }
}
