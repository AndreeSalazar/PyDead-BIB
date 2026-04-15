// ============================================================
// PyDead-BIB ISA Compiler v1.3 — Full Runtime
// ============================================================
// IR → x86-64 machine code bytes
// Direct encoding — sin assembler externo — sin NASM
// Windows: GetStdHandle + WriteFile via IAT
// Linux: write syscall direct
// Runtime stubs: print_str, itoa, ftoa, print_nl
// Supports: int, float, str, bool, if/else, for, while, funcs
// ============================================================

use crate::middle::ir::{IRConstValue, IRCmpOp, IRInstruction, IROp};
use crate::backend::reg_alloc::{AllocatedFunction, AllocatedProgram, X86Reg};

// ── Compilation target ────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Windows,
    Linux,
    FastOS64,
    FastOS128,
    FastOS256,
}

impl Target {
    pub fn from_str(s: &str) -> Self {
        match s {
            "windows" | "win64" | "pe" => Target::Windows,
            "linux" | "elf" => Target::Linux,
            "fastos64" => Target::FastOS64,
            "fastos128" => Target::FastOS128,
            "fastos256" => Target::FastOS256,
            _ => Target::Windows,
        }
    }
}

// ── IAT slot indices (order must match output.rs import table) ──
pub const IAT_GET_STD_HANDLE: usize = 0;
pub const IAT_WRITE_FILE: usize = 1;
pub const IAT_EXIT_PROCESS: usize = 2;
pub const IAT_GET_PROCESS_HEAP: usize = 3;
pub const IAT_HEAP_ALLOC: usize = 4;
pub const IAT_GET_CURRENT_DIRECTORY: usize = 5;
pub const IAT_GET_FILE_ATTRIBUTES: usize = 6;
pub const IAT_GET_CURRENT_PROCESS_ID: usize = 7;
pub const IAT_CREATE_FILE: usize = 8;
pub const IAT_READ_FILE: usize = 9;
pub const IAT_CLOSE_HANDLE: usize = 10;
pub const IAT_CREATE_DIRECTORY: usize = 11;
pub const IAT_DELETE_FILE: usize = 12;
pub const IAT_MOVE_FILE: usize = 13;
pub const IAT_FIND_FIRST_FILE: usize = 14;
pub const IAT_FIND_NEXT_FILE: usize = 15;
pub const IAT_FIND_CLOSE: usize = 16;
pub const IAT_GET_ENVIRONMENT_VARIABLE: usize = 17;
pub const IAT_GET_COMMAND_LINE: usize = 18;
pub const IAT_GET_FILE_SIZE: usize = 19;
pub const IAT_LOAD_LIBRARY: usize = 20;
pub const IAT_GET_PROC_ADDRESS: usize = 21;
pub const IAT_FREE_LIBRARY: usize = 22;
pub const IAT_SLOT_COUNT: usize = 23;

// ── Compiled code section ─────────────────────────────────────
pub struct CompiledProgram {
    pub text: Vec<u8>,
    pub data: Vec<u8>,
    pub data_labels: Vec<(String, u32)>,
    pub functions: Vec<CompiledFunction>,
    pub entry_point: u32,
    pub target: Target,
    pub iat_fixups: Vec<(u32, usize)>,  // (offset_in_text, iat_slot_index)
    pub data_fixups: Vec<(u32, String)>, // (offset_in_text, data_label) for LEA
    pub stats: ISAStats,
}

pub struct CompiledFunction {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Default)]
pub struct ISAStats {
    pub total_bytes: usize,
    pub functions_compiled: usize,
    pub instructions_emitted: usize,
}

// ── x86-64 Encoder ────────────────────────────────────────────
