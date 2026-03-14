// ============================================================
// PyDead-BIB Output — PE/ELF/Po Generator
// ============================================================
// Heredado de ADead-BIB v8.0
// Generates executable binaries directly — sin linker
// PE (Windows x64), ELF (Linux x64), Po (FastOS)
// ============================================================

use crate::backend::bg::StampedProgram;
use crate::backend::isa::Target;

// ── PE Constants (Windows x64) ────────────────────────────────
const DOS_HEADER_SIZE: usize = 64;
const PE_SIGNATURE: u32 = 0x00004550; // "PE\0\0"
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
const IMAGE_FILE_EXECUTABLE_IMAGE: u16 = 0x0002;
const IMAGE_FILE_LARGE_ADDRESS_AWARE: u16 = 0x0020;
const OPTIONAL_HEADER_MAGIC_PE32PLUS: u16 = 0x020B;
const IMAGE_SUBSYSTEM_CONSOLE: u16 = 3;
const SECTION_ALIGNMENT: u32 = 0x1000;
const FILE_ALIGNMENT: u32 = 0x200;

// ── ELF Constants (Linux x64) ─────────────────────────────────
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

// ── Po Constants (FastOS) ─────────────────────────────────────
const PO_MAGIC: u32 = 0x506F4F53; // "PoOS"

// ── Emit binary ───────────────────────────────────────────────
pub fn emit(program: &StampedProgram) -> Vec<u8> {
    match program.target {
        Target::Windows => emit_pe(program),
        Target::Linux => emit_elf(program),
        Target::FastOS64 | Target::FastOS128 | Target::FastOS256 => emit_po(program),
    }
}

// ── PE Generator (Windows x64) ────────────────────────────────
fn emit_pe(program: &StampedProgram) -> Vec<u8> {
    let mut bin = Vec::new();

    // ── DOS Header (64 bytes) ─────────────────────────────
    let mut dos = vec![0u8; DOS_HEADER_SIZE];
    dos[0] = 0x4D; dos[1] = 0x5A; // "MZ"
    // e_lfanew: offset to PE header
    let pe_offset: u32 = DOS_HEADER_SIZE as u32;
    dos[0x3C..0x40].copy_from_slice(&pe_offset.to_le_bytes());
    bin.extend_from_slice(&dos);

    // ── PE Signature ──────────────────────────────────────
    bin.extend_from_slice(&PE_SIGNATURE.to_le_bytes());

    // ── COFF Header (20 bytes) ────────────────────────────
    bin.extend_from_slice(&IMAGE_FILE_MACHINE_AMD64.to_le_bytes()); // Machine
    bin.extend_from_slice(&2u16.to_le_bytes());   // NumberOfSections (.text, .data)
    bin.extend_from_slice(&0u32.to_le_bytes());   // TimeDateStamp
    bin.extend_from_slice(&0u32.to_le_bytes());   // PointerToSymbolTable
    bin.extend_from_slice(&0u32.to_le_bytes());   // NumberOfSymbols
    bin.extend_from_slice(&240u16.to_le_bytes()); // SizeOfOptionalHeader
    let characteristics = IMAGE_FILE_EXECUTABLE_IMAGE | IMAGE_FILE_LARGE_ADDRESS_AWARE;
    bin.extend_from_slice(&characteristics.to_le_bytes());

    // ── Optional Header (PE32+) ───────────────────────────
    let image_base: u64 = 0x0000000140000000;
    let text_rva: u32 = SECTION_ALIGNMENT;
    let text_size = align_up(program.text.len() as u32, FILE_ALIGNMENT);
    let data_rva: u32 = text_rva + align_up(text_size, SECTION_ALIGNMENT);
    let data_size = align_up(program.data.len() as u32 + 24, FILE_ALIGNMENT); // +24 for BG stamp
    let size_of_image = data_rva + align_up(data_size, SECTION_ALIGNMENT);
    let size_of_headers = align_up(
        DOS_HEADER_SIZE as u32 + 4 + 20 + 240 + 2 * 40, // DOS + sig + COFF + opt + 2 sections
        FILE_ALIGNMENT,
    );

    let entry_rva = text_rva + program.entry_point;

    bin.extend_from_slice(&OPTIONAL_HEADER_MAGIC_PE32PLUS.to_le_bytes());
    bin.push(1); bin.push(0); // Linker version
    bin.extend_from_slice(&text_size.to_le_bytes());  // SizeOfCode
    bin.extend_from_slice(&data_size.to_le_bytes());  // SizeOfInitializedData
    bin.extend_from_slice(&0u32.to_le_bytes());       // SizeOfUninitializedData
    bin.extend_from_slice(&entry_rva.to_le_bytes());  // AddressOfEntryPoint
    bin.extend_from_slice(&text_rva.to_le_bytes());   // BaseOfCode
    bin.extend_from_slice(&image_base.to_le_bytes()); // ImageBase
    bin.extend_from_slice(&SECTION_ALIGNMENT.to_le_bytes());
    bin.extend_from_slice(&FILE_ALIGNMENT.to_le_bytes());
    // OS version
    bin.extend_from_slice(&6u16.to_le_bytes()); // Major
    bin.extend_from_slice(&0u16.to_le_bytes()); // Minor
    // Image version
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    // Subsystem version
    bin.extend_from_slice(&6u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes()); // Win32VersionValue
    bin.extend_from_slice(&size_of_image.to_le_bytes());
    bin.extend_from_slice(&size_of_headers.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes()); // CheckSum
    bin.extend_from_slice(&IMAGE_SUBSYSTEM_CONSOLE.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes()); // DllCharacteristics
    // Stack/Heap sizes (PE32+ = 8 bytes each)
    bin.extend_from_slice(&0x100000u64.to_le_bytes()); // SizeOfStackReserve
    bin.extend_from_slice(&0x1000u64.to_le_bytes());   // SizeOfStackCommit
    bin.extend_from_slice(&0x100000u64.to_le_bytes()); // SizeOfHeapReserve
    bin.extend_from_slice(&0x1000u64.to_le_bytes());   // SizeOfHeapCommit
    bin.extend_from_slice(&0u32.to_le_bytes());        // LoaderFlags
    bin.extend_from_slice(&16u32.to_le_bytes());       // NumberOfRvaAndSizes
    // Data directories (16 entries × 8 bytes each = 128 bytes)
    for _ in 0..16 {
        bin.extend_from_slice(&0u32.to_le_bytes()); // RVA
        bin.extend_from_slice(&0u32.to_le_bytes()); // Size
    }

    // ── Section Headers ───────────────────────────────────
    // .text section
    bin.extend_from_slice(b".text\0\0\0");            // Name
    bin.extend_from_slice(&(program.text.len() as u32).to_le_bytes()); // VirtualSize
    bin.extend_from_slice(&text_rva.to_le_bytes());   // VirtualAddress
    bin.extend_from_slice(&text_size.to_le_bytes());  // SizeOfRawData
    bin.extend_from_slice(&size_of_headers.to_le_bytes()); // PointerToRawData
    bin.extend_from_slice(&0u32.to_le_bytes());       // PointerToRelocations
    bin.extend_from_slice(&0u32.to_le_bytes());       // PointerToLinenumbers
    bin.extend_from_slice(&0u16.to_le_bytes());       // NumberOfRelocations
    bin.extend_from_slice(&0u16.to_le_bytes());       // NumberOfLinenumbers
    bin.extend_from_slice(&0x60000020u32.to_le_bytes()); // CODE|EXECUTE|READ

    // .data section
    let data_file_offset = size_of_headers + text_size;
    bin.extend_from_slice(b".data\0\0\0");
    bin.extend_from_slice(&(program.data.len() as u32 + 24).to_le_bytes());
    bin.extend_from_slice(&data_rva.to_le_bytes());
    bin.extend_from_slice(&data_size.to_le_bytes());
    bin.extend_from_slice(&data_file_offset.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0xC0000040u32.to_le_bytes()); // INITIALIZED|READ|WRITE

    // ── Pad headers to FILE_ALIGNMENT ─────────────────────
    while bin.len() < size_of_headers as usize {
        bin.push(0);
    }

    // ── .text section data ────────────────────────────────
    bin.extend_from_slice(&program.text);
    while bin.len() < (size_of_headers + text_size) as usize {
        bin.push(0xCC); // INT3 padding
    }

    // ── .data section data ────────────────────────────────
    bin.extend_from_slice(&program.stamp.to_bytes()); // BG stamp first
    bin.extend_from_slice(&program.data);
    while bin.len() < (size_of_headers + text_size + data_size) as usize {
        bin.push(0);
    }

    bin
}

// ── ELF Generator (Linux x64) ─────────────────────────────────
fn emit_elf(program: &StampedProgram) -> Vec<u8> {
    let mut bin = Vec::new();

    let ehdr_size: u16 = 64;
    let phdr_size: u16 = 56;
    let phdr_count: u16 = 2; // .text + .data

    let base_addr: u64 = 0x400000;
    let text_offset: u64 = (ehdr_size + phdr_size * phdr_count) as u64;
    let text_offset_aligned = align_up64(text_offset, 16);
    let text_size = program.text.len() as u64;
    let data_offset = text_offset_aligned + text_size;
    let data_offset_aligned = align_up64(data_offset, 16);
    let bg_stamp_bytes = program.stamp.to_bytes();
    let data_total = bg_stamp_bytes.len() as u64 + program.data.len() as u64;

    let entry_addr = base_addr + text_offset_aligned + program.entry_point as u64;

    // ── ELF Header ────────────────────────────────────────
    bin.extend_from_slice(&ELF_MAGIC);
    bin.push(ELFCLASS64);    // 64-bit
    bin.push(ELFDATA2LSB);   // Little endian
    bin.push(1);             // ELF version
    bin.push(0);             // OS/ABI (NONE)
    bin.extend_from_slice(&[0u8; 8]); // Padding
    bin.extend_from_slice(&ET_EXEC.to_le_bytes());
    bin.extend_from_slice(&EM_X86_64.to_le_bytes());
    bin.extend_from_slice(&1u32.to_le_bytes()); // ELF version
    bin.extend_from_slice(&entry_addr.to_le_bytes());
    bin.extend_from_slice(&(ehdr_size as u64).to_le_bytes()); // phoff
    bin.extend_from_slice(&0u64.to_le_bytes()); // shoff (no sections)
    bin.extend_from_slice(&0u32.to_le_bytes()); // flags
    bin.extend_from_slice(&ehdr_size.to_le_bytes());
    bin.extend_from_slice(&phdr_size.to_le_bytes());
    bin.extend_from_slice(&phdr_count.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes()); // shentsize
    bin.extend_from_slice(&0u16.to_le_bytes()); // shnum
    bin.extend_from_slice(&0u16.to_le_bytes()); // shstrndx

    // ── Program Header: .text (LOAD, R+X) ─────────────────
    bin.extend_from_slice(&PT_LOAD.to_le_bytes());
    bin.extend_from_slice(&(PF_R | PF_X).to_le_bytes()); // flags
    bin.extend_from_slice(&text_offset_aligned.to_le_bytes()); // offset
    bin.extend_from_slice(&(base_addr + text_offset_aligned).to_le_bytes()); // vaddr
    bin.extend_from_slice(&(base_addr + text_offset_aligned).to_le_bytes()); // paddr
    bin.extend_from_slice(&text_size.to_le_bytes()); // filesz
    bin.extend_from_slice(&text_size.to_le_bytes()); // memsz
    bin.extend_from_slice(&0x1000u64.to_le_bytes()); // align

    // ── Program Header: .data (LOAD, R+W) ─────────────────
    bin.extend_from_slice(&PT_LOAD.to_le_bytes());
    bin.extend_from_slice(&(PF_R | PF_W).to_le_bytes());
    bin.extend_from_slice(&data_offset_aligned.to_le_bytes());
    bin.extend_from_slice(&(base_addr + data_offset_aligned).to_le_bytes());
    bin.extend_from_slice(&(base_addr + data_offset_aligned).to_le_bytes());
    bin.extend_from_slice(&data_total.to_le_bytes());
    bin.extend_from_slice(&data_total.to_le_bytes());
    bin.extend_from_slice(&0x1000u64.to_le_bytes());

    // ── Pad to text offset ────────────────────────────────
    while bin.len() < text_offset_aligned as usize {
        bin.push(0);
    }

    // ── .text ─────────────────────────────────────────────
    bin.extend_from_slice(&program.text);

    // ── Pad to data offset ────────────────────────────────
    while bin.len() < data_offset_aligned as usize {
        bin.push(0);
    }

    // ── .data (BG stamp + string data) ────────────────────
    bin.extend_from_slice(&bg_stamp_bytes);
    bin.extend_from_slice(&program.data);

    bin
}

// ── Po Generator (FastOS) ─────────────────────────────────────
fn emit_po(program: &StampedProgram) -> Vec<u8> {
    let mut bin = Vec::new();

    // Po header
    bin.extend_from_slice(&PO_MAGIC.to_le_bytes());
    let version: u8 = match program.target {
        Target::FastOS64 => 1,
        Target::FastOS128 => 2,
        Target::FastOS256 => 8,
        _ => 1,
    };
    bin.push(version);
    bin.push(0); // flags
    bin.extend_from_slice(&(program.text.len() as u32).to_le_bytes());
    bin.extend_from_slice(&(program.data.len() as u32).to_le_bytes());
    bin.extend_from_slice(&program.entry_point.to_le_bytes());

    // BG stamp
    bin.extend_from_slice(&program.stamp.to_bytes());

    // .text
    bin.extend_from_slice(&program.text);

    // .data
    bin.extend_from_slice(&program.data);

    bin
}

// ── Helpers ───────────────────────────────────────────────────
fn align_up(val: u32, alignment: u32) -> u32 {
    (val + alignment - 1) & !(alignment - 1)
}

fn align_up64(val: u64, alignment: u64) -> u64 {
    (val + alignment - 1) & !(alignment - 1)
}

// ── Stats ─────────────────────────────────────────────────────
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
