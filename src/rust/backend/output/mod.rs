// ============================================================
// PyDead-BIB Output Router v2.0
// ============================================================

pub mod pe_writer;
pub mod elf_writer;

use crate::backend::bg::StampedProgram;
use crate::backend::isa::{Target};
use pe_writer::emit_pe;
use elf_writer::emit_elf;

const PO_MAGIC: u32 = 0x506F4F53;

pub fn emit(program: &StampedProgram) -> Vec<u8> {
    match program.target {
        Target::Windows => emit_pe(program),
        Target::Linux => emit_elf(program),
        Target::FastOS64 | Target::FastOS128 | Target::FastOS256 => emit_po(program),
    }
}

fn emit_po(program: &StampedProgram) -> Vec<u8> {
    let mut bin = Vec::new();
    bin.extend_from_slice(&PO_MAGIC.to_le_bytes());
    let version: u8 = match program.target {
        Target::FastOS64 => 1, Target::FastOS128 => 2, Target::FastOS256 => 8, _ => 1,
    };
    bin.push(version); bin.push(0);
    bin.extend_from_slice(&(program.text.len() as u32).to_le_bytes());
    bin.extend_from_slice(&(program.data.len() as u32).to_le_bytes());
    bin.extend_from_slice(&program.entry_point.to_le_bytes());
    bin.extend_from_slice(&program.stamp.to_bytes());
    bin.extend_from_slice(&program.text);
    bin.extend_from_slice(&program.data);
    bin
}

pub fn binary_stats(binary: &[u8], program: &StampedProgram) -> BinaryStats {
    BinaryStats {
        total_bytes: binary.len(),
        text_bytes: program.text.len(),
        data_bytes: program.data.len(),
        functions: program.functions.len(),
        target: format!("{:?}", program.target),
    }
}

#[derive(Debug)]
pub struct BinaryStats {
    pub total_bytes: usize,
    pub text_bytes: usize,
    pub data_bytes: usize,
    pub functions: usize,
    pub target: String,
}
