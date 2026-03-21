// ============================================================
// ISA Compiler Core — Structs, Enums, Constructors
// ============================================================

use crate::isa::reg_alloc::TempAllocator;
use crate::isa::{ADeadIR, Label, Reg};
use std::collections::HashMap;

/// Target de compilación
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    Windows,
    Linux,
    Raw,
}

/// CPU Mode — 16-bit → 32-bit → 64-bit scaling
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CpuMode {
    Real16,
    Protected32,
    Long64,
}

impl CpuMode {
    pub fn operand_bits(&self) -> u8 {
        match self {
            CpuMode::Real16 => 16,
            CpuMode::Protected32 => 32,
            CpuMode::Long64 => 64,
        }
    }

    pub fn address_bits(&self) -> u8 {
        match self {
            CpuMode::Real16 => 16,
            CpuMode::Protected32 => 32,
            CpuMode::Long64 => 64,
        }
    }

    pub fn needs_rex(&self) -> bool {
        matches!(self, CpuMode::Long64)
    }

    pub fn stack_reg(&self) -> &'static str {
        match self {
            CpuMode::Real16 => "SP",
            CpuMode::Protected32 => "ESP",
            CpuMode::Long64 => "RSP",
        }
    }
}

impl Default for CpuMode {
    fn default() -> Self {
        CpuMode::Long64
    }
}

/// Función compilada (metadatos)
#[derive(Clone, Debug)]
pub struct CompiledFunction {
    pub name: String,
    pub label: Label,
    pub params: Vec<String>,
}

/// ISA Compiler — Genera ADeadIR desde AST
pub struct IsaCompiler {
    pub(crate) ir: ADeadIR,
    pub(crate) strings: Vec<String>,
    pub(crate) string_offsets: HashMap<String, u64>,
    pub(crate) functions: HashMap<String, CompiledFunction>,
    pub(crate) current_function: Option<String>,
    pub(crate) variables: HashMap<String, i32>,
    pub(crate) stack_offset: i32,
    pub(crate) target: Target,
    pub(crate) base_address: u64,
    pub(crate) data_rva: u64,
    pub(crate) cpu_mode: CpuMode,
    pub(crate) named_labels: HashMap<String, Label>,
    pub(crate) temp_alloc: TempAllocator,
    pub(crate) prologue_sub_index: Option<usize>,
    pub(crate) loop_stack: Vec<(Label, Label)>,
}

impl IsaCompiler {
    pub fn new(target: Target) -> Self {
        let (base, data_rva) = match target {
            Target::Windows => (0x0000000140000000, 0x2078),
            Target::Linux => (0x400000, 0x1000),
            Target::Raw => (0x0, 0x1000),
        };

        Self {
            ir: ADeadIR::new(),
            strings: Vec::new(),
            string_offsets: HashMap::new(),
            functions: HashMap::new(),
            current_function: None,
            variables: HashMap::new(),
            stack_offset: 0,
            target,
            base_address: base,
            data_rva,
            cpu_mode: CpuMode::Long64,
            named_labels: HashMap::new(),
            temp_alloc: TempAllocator::new(),
            prologue_sub_index: None,
            loop_stack: Vec::new(),
        }
    }

    pub fn with_cpu_mode(target: Target, mode: CpuMode) -> Self {
        let mut compiler = Self::new(target);
        compiler.cpu_mode = mode;
        compiler
    }

    pub fn new_real16() -> Self {
        Self::with_cpu_mode(Target::Raw, CpuMode::Real16)
    }

    pub fn new_protected32() -> Self {
        Self::with_cpu_mode(Target::Raw, CpuMode::Protected32)
    }

    pub fn new_long64(target: Target) -> Self {
        Self::with_cpu_mode(target, CpuMode::Long64)
    }

    pub fn set_cpu_mode(&mut self, mode: CpuMode) {
        self.cpu_mode = mode;
    }

    pub fn cpu_mode(&self) -> CpuMode {
        self.cpu_mode
    }

    pub fn ir(&self) -> &ADeadIR {
        &self.ir
    }

    pub(crate) fn get_string_address(&self, s: &str) -> u64 {
        if let Some(&offset) = self.string_offsets.get(s) {
            self.base_address + self.data_rva + offset
        } else {
            self.base_address + self.data_rva
        }
    }

    pub(crate) fn get_or_create_named_label(&mut self, name: &str) -> Label {
        if let Some(&label) = self.named_labels.get(name) {
            label
        } else {
            let label = self.ir.new_label();
            self.named_labels.insert(name.to_string(), label);
            label
        }
    }

    pub(crate) fn string_to_reg(name: &str) -> Option<Reg> {
        match name {
            "rax" => Some(Reg::RAX),
            "rbx" => Some(Reg::RBX),
            "rcx" => Some(Reg::RCX),
            "rdx" => Some(Reg::RDX),
            "rsi" => Some(Reg::RSI),
            "rdi" => Some(Reg::RDI),
            "rbp" => Some(Reg::RBP),
            "rsp" => Some(Reg::RSP),
            "r8" => Some(Reg::R8),
            "r9" => Some(Reg::R9),
            "r10" => Some(Reg::R10),
            "r11" => Some(Reg::R11),
            "r12" => Some(Reg::R12),
            "r13" => Some(Reg::R13),
            "r14" => Some(Reg::R14),
            "r15" => Some(Reg::R15),
            "eax" => Some(Reg::EAX),
            "ebx" => Some(Reg::EBX),
            "ecx" => Some(Reg::ECX),
            "edx" => Some(Reg::EDX),
            "esi" => Some(Reg::ESI),
            "edi" => Some(Reg::EDI),
            "esp" => Some(Reg::ESP),
            "ebp" => Some(Reg::EBP),
            "ax" => Some(Reg::AX),
            "bx" => Some(Reg::BX),
            "cx" => Some(Reg::CX),
            "dx" => Some(Reg::DX),
            "si" => Some(Reg::SI),
            "di" => Some(Reg::DI),
            "sp" => Some(Reg::SP),
            "bp" => Some(Reg::BP),
            "al" => Some(Reg::AL),
            "ah" => Some(Reg::AH),
            "bl" => Some(Reg::BL),
            "bh" => Some(Reg::BH),
            "cl" => Some(Reg::CL),
            "ch" => Some(Reg::CH),
            "dl" => Some(Reg::DL),
            "dh" => Some(Reg::DH),
            "cr0" => Some(Reg::CR0),
            "cr2" => Some(Reg::CR2),
            "cr3" => Some(Reg::CR3),
            "cr4" => Some(Reg::CR4),
            "cs" => Some(Reg::CS),
            "ds" => Some(Reg::DS),
            "es" => Some(Reg::ES),
            "fs" => Some(Reg::FS),
            "gs" => Some(Reg::GS),
            "ss" => Some(Reg::SS),
            _ => None,
        }
    }

    pub(crate) fn arg_register(&self, index: usize) -> Reg {
        match self.target {
            Target::Windows => match index {
                0 => Reg::RCX,
                1 => Reg::RDX,
                2 => Reg::R8,
                3 => Reg::R9,
                _ => Reg::RCX,
            },
            Target::Linux | Target::Raw => match index {
                0 => Reg::RDI,
                1 => Reg::RSI,
                2 => Reg::RDX,
                3 => Reg::RCX,
                _ => Reg::RDI,
            },
        }
    }
}
