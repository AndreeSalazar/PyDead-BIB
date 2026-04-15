// ============================================================
// PyDead-BIB JIT 2.0 — ELF Generator (Linux x64)
// ============================================================

use crate::backend::bg::StampedProgram;

// ── ELF Constants ─────────────────────────────────────────────
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

pub fn align_up64(val: u64, alignment: u64) -> u64 {
    (val + alignment - 1) & !(alignment - 1)
}

pub fn emit_elf(program: &StampedProgram) -> Vec<u8> {
    let mut bin = Vec::new();

    let ehdr_size: u16 = 64;
    let phdr_size: u16 = 56;
    let phdr_count: u16 = 2;

    let base_addr: u64 = 0x400000;
    let text_offset: u64 = (ehdr_size + phdr_size * phdr_count) as u64;
    let text_offset_aligned = align_up64(text_offset, 16);
    let text_size = program.text.len() as u64;
    let data_offset = text_offset_aligned + text_size;
    let data_offset_aligned = align_up64(data_offset, 16);
    let bg_stamp_bytes = program.stamp.to_bytes();
    let data_total = bg_stamp_bytes.len() as u64 + program.data.len() as u64;
    let entry_addr = base_addr + text_offset_aligned + program.entry_point as u64;

    bin.extend_from_slice(&ELF_MAGIC);
    bin.push(ELFCLASS64);
    bin.push(ELFDATA2LSB);
    bin.push(1); bin.push(0);
    bin.extend_from_slice(&[0u8; 8]);
    bin.extend_from_slice(&ET_EXEC.to_le_bytes());
    bin.extend_from_slice(&EM_X86_64.to_le_bytes());
    bin.extend_from_slice(&1u32.to_le_bytes());
    bin.extend_from_slice(&entry_addr.to_le_bytes());
    bin.extend_from_slice(&(ehdr_size as u64).to_le_bytes());
    bin.extend_from_slice(&0u64.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&ehdr_size.to_le_bytes());
    bin.extend_from_slice(&phdr_size.to_le_bytes());
    bin.extend_from_slice(&phdr_count.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());

    bin.extend_from_slice(&PT_LOAD.to_le_bytes());
    bin.extend_from_slice(&(PF_R | PF_X).to_le_bytes());
    bin.extend_from_slice(&text_offset_aligned.to_le_bytes());
    bin.extend_from_slice(&(base_addr + text_offset_aligned).to_le_bytes());
    bin.extend_from_slice(&(base_addr + text_offset_aligned).to_le_bytes());
    bin.extend_from_slice(&text_size.to_le_bytes());
    bin.extend_from_slice(&text_size.to_le_bytes());
    bin.extend_from_slice(&0x1000u64.to_le_bytes());

    bin.extend_from_slice(&PT_LOAD.to_le_bytes());
    bin.extend_from_slice(&(PF_R | PF_W).to_le_bytes());
    bin.extend_from_slice(&data_offset_aligned.to_le_bytes());
    bin.extend_from_slice(&(base_addr + data_offset_aligned).to_le_bytes());
    bin.extend_from_slice(&(base_addr + data_offset_aligned).to_le_bytes());
    bin.extend_from_slice(&data_total.to_le_bytes());
    bin.extend_from_slice(&data_total.to_le_bytes());
    bin.extend_from_slice(&0x1000u64.to_le_bytes());

    while bin.len() < text_offset_aligned as usize { bin.push(0); }
    bin.extend_from_slice(&program.text);
    while bin.len() < data_offset_aligned as usize { bin.push(0); }
    bin.extend_from_slice(&bg_stamp_bytes);
    bin.extend_from_slice(&program.data);

    bin
}
