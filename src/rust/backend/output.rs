// ============================================================
// PyDead-BIB Output v1.2 — PE/ELF/Po Generator
// ============================================================
// Generates executable binaries directly — sin linker
// PE (Windows x64) with .idata import table for kernel32.dll
// ELF (Linux x64), Po (FastOS)
// Patches IAT fixups and data fixups from ISA compiler
// ============================================================

use crate::backend::bg::StampedProgram;
use crate::backend::isa::{Target, IAT_SLOT_COUNT};

// ── PE Constants ──────────────────────────────────────────────
const PE_SIGNATURE: u32 = 0x00004550;
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
const IMAGE_FILE_EXECUTABLE_IMAGE: u16 = 0x0002;
const IMAGE_FILE_LARGE_ADDRESS_AWARE: u16 = 0x0020;
const OPTIONAL_HEADER_MAGIC_PE32PLUS: u16 = 0x020B;
const IMAGE_SUBSYSTEM_CONSOLE: u16 = 3;
const SECTION_ALIGNMENT: u32 = 0x1000;
const FILE_ALIGNMENT: u32 = 0x200;

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

const PO_MAGIC: u32 = 0x506F4F53;

// ── Import table function names ───────────────────────────────
const IMPORT_FUNCS: [&str; IAT_SLOT_COUNT] = [
    "GetStdHandle",
    "WriteFile",
    "ExitProcess",
];
const IMPORT_DLL: &str = "KERNEL32.dll";

// ── Emit binary ───────────────────────────────────────────────
pub fn emit(program: &StampedProgram) -> Vec<u8> {
    match program.target {
        Target::Windows => emit_pe(program),
        Target::Linux => emit_elf(program),
        Target::FastOS64 | Target::FastOS128 | Target::FastOS256 => emit_po(program),
    }
}

// ══════════════════════════════════════════════════════════════
// PE Generator (Windows x64) with Import Table
// ══════════════════════════════════════════════════════════════
fn emit_pe(program: &StampedProgram) -> Vec<u8> {
    let image_base: u64 = 0x0000000140000000;

    // ── Build .idata section content first ────────────────
    let idata = build_idata(IAT_SLOT_COUNT);

    // ── Calculate layout ──────────────────────────────────
    let num_sections: u16 = 3; // .text, .rdata (idata), .data
    let dos_size: u32 = 64;
    let pe_sig_size: u32 = 4;
    let coff_size: u32 = 20;
    let opt_hdr_size: u32 = 240;
    let section_hdr_size: u32 = 40 * num_sections as u32;
    let headers_raw = dos_size + pe_sig_size + coff_size + opt_hdr_size + section_hdr_size;
    let size_of_headers = align_up(headers_raw, FILE_ALIGNMENT);

    let text_rva: u32 = SECTION_ALIGNMENT;
    let text_raw_size = align_up(program.text.len() as u32, FILE_ALIGNMENT);
    let text_file_off = size_of_headers;

    let rdata_rva: u32 = text_rva + align_up(text_raw_size, SECTION_ALIGNMENT);
    let rdata_raw_size = align_up(idata.total_size as u32, FILE_ALIGNMENT);
    let rdata_file_off = text_file_off + text_raw_size;

    let data_rva: u32 = rdata_rva + align_up(rdata_raw_size, SECTION_ALIGNMENT);
    let bg_stamp_bytes = program.stamp.to_bytes();
    let data_virt_size = bg_stamp_bytes.len() as u32 + program.data.len() as u32;
    let data_raw_size = align_up(data_virt_size, FILE_ALIGNMENT);
    let data_file_off = rdata_file_off + rdata_raw_size;

    let size_of_image = data_rva + align_up(data_raw_size, SECTION_ALIGNMENT);
    let entry_rva = text_rva + program.entry_point;

    // IAT RVA within .rdata
    let iat_rva = rdata_rva + idata.iat_offset as u32;
    let iat_size = (IAT_SLOT_COUNT + 1) as u32 * 8; // +1 for null terminator
    let import_dir_rva = rdata_rva; // import directory is at start of .rdata

    // ── Patch .text with IAT fixups ───────────────────────
    let mut text = program.text.clone();

    // Patch IAT fixups: each is a CALL [RIP+disp32]
    // The disp32 = target_addr - (instr_addr + 4)
    // target_addr = image_base + iat_rva + slot*8
    // instr_addr = image_base + text_rva + fixup_offset
    for &(fixup_offset, slot_idx) in &program.iat_fixups {
        let iat_entry_rva = iat_rva + (slot_idx as u32) * 8;
        let instr_rva = text_rva + fixup_offset;
        let disp32 = (iat_entry_rva as i32) - (instr_rva as i32 + 4);
        let off = fixup_offset as usize;
        if off + 4 <= text.len() {
            text[off..off+4].copy_from_slice(&disp32.to_le_bytes());
        }
    }

    // Patch data fixups: each is a LEA RAX, [RIP+disp32]
    // target_addr = image_base + data_rva + bg_stamp_size + data_label_offset
    let bg_stamp_size = bg_stamp_bytes.len() as u32;
    for &(fixup_offset, ref label) in &program.data_fixups {
        if let Some((_, data_off)) = program.data_labels.iter().find(|(n, _)| n == label) {
            let target_rva = data_rva + bg_stamp_size + data_off;
            let instr_rva = text_rva + fixup_offset;
            let disp32 = (target_rva as i32) - (instr_rva as i32 + 4);
            let off = fixup_offset as usize;
            if off + 4 <= text.len() {
                text[off..off+4].copy_from_slice(&disp32.to_le_bytes());
            }
        }
    }

    // ── Build PE ──────────────────────────────────────────
    let mut bin = Vec::with_capacity(size_of_image as usize);

    // DOS Header
    let mut dos = vec![0u8; 64];
    dos[0] = 0x4D; dos[1] = 0x5A;
    dos[0x3C..0x40].copy_from_slice(&64u32.to_le_bytes());
    bin.extend_from_slice(&dos);

    // PE Signature
    bin.extend_from_slice(&PE_SIGNATURE.to_le_bytes());

    // COFF Header
    bin.extend_from_slice(&IMAGE_FILE_MACHINE_AMD64.to_le_bytes());
    bin.extend_from_slice(&num_sections.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes()); // timestamp
    bin.extend_from_slice(&0u32.to_le_bytes()); // sym table
    bin.extend_from_slice(&0u32.to_le_bytes()); // num syms
    bin.extend_from_slice(&(opt_hdr_size as u16).to_le_bytes());
    let chars = IMAGE_FILE_EXECUTABLE_IMAGE | IMAGE_FILE_LARGE_ADDRESS_AWARE;
    bin.extend_from_slice(&chars.to_le_bytes());

    // Optional Header (PE32+)
    bin.extend_from_slice(&OPTIONAL_HEADER_MAGIC_PE32PLUS.to_le_bytes());
    bin.push(1); bin.push(0); // linker version
    bin.extend_from_slice(&text_raw_size.to_le_bytes()); // SizeOfCode
    bin.extend_from_slice(&(rdata_raw_size + data_raw_size).to_le_bytes()); // SizeOfInitializedData
    bin.extend_from_slice(&0u32.to_le_bytes()); // SizeOfUninitializedData
    bin.extend_from_slice(&entry_rva.to_le_bytes());
    bin.extend_from_slice(&text_rva.to_le_bytes()); // BaseOfCode
    bin.extend_from_slice(&image_base.to_le_bytes());
    bin.extend_from_slice(&SECTION_ALIGNMENT.to_le_bytes());
    bin.extend_from_slice(&FILE_ALIGNMENT.to_le_bytes());
    // OS version
    bin.extend_from_slice(&6u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
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
    bin.extend_from_slice(&0x0100u16.to_le_bytes()); // DLL chars: NX_COMPAT only (no DYNAMIC_BASE — no relocs)
    // Stack/Heap sizes
    bin.extend_from_slice(&0x100000u64.to_le_bytes());
    bin.extend_from_slice(&0x1000u64.to_le_bytes());
    bin.extend_from_slice(&0x100000u64.to_le_bytes());
    bin.extend_from_slice(&0x1000u64.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes()); // LoaderFlags
    bin.extend_from_slice(&16u32.to_le_bytes()); // NumberOfRvaAndSizes

    // Data Directories (16 entries)
    // [0] Export = 0
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    // [1] Import Directory
    bin.extend_from_slice(&import_dir_rva.to_le_bytes());
    bin.extend_from_slice(&(idata.import_dir_size as u32).to_le_bytes());
    // [2..11] = 0
    for _ in 2..12 {
        bin.extend_from_slice(&0u32.to_le_bytes());
        bin.extend_from_slice(&0u32.to_le_bytes());
    }
    // [12] IAT
    bin.extend_from_slice(&iat_rva.to_le_bytes());
    bin.extend_from_slice(&iat_size.to_le_bytes());
    // [13..15] = 0
    for _ in 13..16 {
        bin.extend_from_slice(&0u32.to_le_bytes());
        bin.extend_from_slice(&0u32.to_le_bytes());
    }

    // Section Headers
    // .text
    write_section_header(&mut bin, b".text\0\0\0",
        program.text.len() as u32, text_rva, text_raw_size, text_file_off,
        0x60000020); // CODE|EXECUTE|READ

    // .rdata (import table)
    write_section_header(&mut bin, b".rdata\0\0",
        idata.total_size as u32, rdata_rva, rdata_raw_size, rdata_file_off,
        0x40000040); // INITIALIZED|READ

    // .data
    write_section_header(&mut bin, b".data\0\0\0",
        data_virt_size, data_rva, data_raw_size, data_file_off,
        0xC0000040); // INITIALIZED|READ|WRITE

    // Pad to size_of_headers
    while bin.len() < size_of_headers as usize { bin.push(0); }

    // .text section
    bin.extend_from_slice(&text);
    while bin.len() < (text_file_off + text_raw_size) as usize { bin.push(0xCC); }

    // .rdata section (import table)
    // Write import directory, ILT, IAT, hint/name table, DLL name
    let rdata_bytes = build_idata_bytes(&idata, rdata_rva);
    bin.extend_from_slice(&rdata_bytes);
    while bin.len() < (rdata_file_off + rdata_raw_size) as usize { bin.push(0); }

    // .data section
    bin.extend_from_slice(&bg_stamp_bytes);
    bin.extend_from_slice(&program.data);
    while bin.len() < (data_file_off + data_raw_size) as usize { bin.push(0); }

    bin
}

// ── Import table layout calculation ───────────────────────────
struct IdataLayout {
    import_dir_size: usize,  // 2 entries × 20 bytes (1 real + 1 null)
    ilt_offset: usize,       // Import Lookup Table offset within .rdata
    iat_offset: usize,       // Import Address Table offset
    hints_offset: usize,     // Hint/Name table offset
    dll_name_offset: usize,  // DLL name string offset
    total_size: usize,
}

fn build_idata(num_funcs: usize) -> IdataLayout {
    let import_dir_size = 40; // 1 entry (20 bytes) + 1 null entry (20 bytes)
    let ilt_offset = import_dir_size;
    let ilt_size = (num_funcs + 1) * 8; // +1 null terminator
    let iat_offset = ilt_offset + ilt_size;
    let iat_size = (num_funcs + 1) * 8;
    let hints_offset = iat_offset + iat_size;

    // Calculate hint/name table size
    let mut hints_size = 0;
    for func_name in &IMPORT_FUNCS {
        hints_size += 2; // hint (u16)
        hints_size += func_name.len() + 1; // name + null
        if hints_size % 2 != 0 { hints_size += 1; } // pad to even
    }

    let dll_name_offset = hints_offset + hints_size;
    let dll_name_size = IMPORT_DLL.len() + 1;
    let total_size = dll_name_offset + dll_name_size;

    IdataLayout {
        import_dir_size,
        ilt_offset,
        iat_offset,
        hints_offset,
        dll_name_offset,
        total_size,
    }
}

fn build_idata_bytes(layout: &IdataLayout, rdata_rva: u32) -> Vec<u8> {
    let mut buf = vec![0u8; layout.total_size];

    let ilt_rva = rdata_rva + layout.ilt_offset as u32;
    let iat_rva = rdata_rva + layout.iat_offset as u32;
    let dll_name_rva = rdata_rva + layout.dll_name_offset as u32;

    // Import Directory Entry (20 bytes)
    // OriginalFirstThunk (ILT RVA)
    buf[0..4].copy_from_slice(&ilt_rva.to_le_bytes());
    // TimeDateStamp
    buf[4..8].copy_from_slice(&0u32.to_le_bytes());
    // ForwarderChain
    buf[8..12].copy_from_slice(&0u32.to_le_bytes());
    // Name (DLL name RVA)
    buf[12..16].copy_from_slice(&dll_name_rva.to_le_bytes());
    // FirstThunk (IAT RVA)
    buf[16..20].copy_from_slice(&iat_rva.to_le_bytes());
    // Null terminator entry (bytes 20..40 already zero)

    // Build hint/name entries and fill ILT + IAT
    let mut hint_off = layout.hints_offset;
    for (i, func_name) in IMPORT_FUNCS.iter().enumerate() {
        let hint_rva = rdata_rva + hint_off as u32;

        // ILT entry: RVA to hint/name
        let ilt_entry_off = layout.ilt_offset + i * 8;
        buf[ilt_entry_off..ilt_entry_off+8].copy_from_slice(&(hint_rva as u64).to_le_bytes());

        // IAT entry: same as ILT (loader overwrites at load time)
        let iat_entry_off = layout.iat_offset + i * 8;
        buf[iat_entry_off..iat_entry_off+8].copy_from_slice(&(hint_rva as u64).to_le_bytes());

        // Hint/Name entry: u16 hint + name + null + pad
        buf[hint_off] = 0; buf[hint_off+1] = 0; // hint = 0 (ordinal hint)
        hint_off += 2;
        for &b in func_name.as_bytes() {
            buf[hint_off] = b;
            hint_off += 1;
        }
        buf[hint_off] = 0; // null terminator
        hint_off += 1;
        if hint_off % 2 != 0 {
            buf[hint_off] = 0; // pad to even
            hint_off += 1;
        }
    }

    // DLL name string
    let dll_off = layout.dll_name_offset;
    for (i, &b) in IMPORT_DLL.as_bytes().iter().enumerate() {
        buf[dll_off + i] = b;
    }
    buf[dll_off + IMPORT_DLL.len()] = 0;

    buf
}

fn write_section_header(bin: &mut Vec<u8>, name: &[u8; 8],
    virt_size: u32, virt_addr: u32, raw_size: u32, raw_ptr: u32, chars: u32)
{
    bin.extend_from_slice(name);
    bin.extend_from_slice(&virt_size.to_le_bytes());
    bin.extend_from_slice(&virt_addr.to_le_bytes());
    bin.extend_from_slice(&raw_size.to_le_bytes());
    bin.extend_from_slice(&raw_ptr.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes()); // PointerToRelocations
    bin.extend_from_slice(&0u32.to_le_bytes()); // PointerToLinenumbers
    bin.extend_from_slice(&0u16.to_le_bytes()); // NumberOfRelocations
    bin.extend_from_slice(&0u16.to_le_bytes()); // NumberOfLinenumbers
    bin.extend_from_slice(&chars.to_le_bytes());
}

// ══════════════════════════════════════════════════════════════
// ELF Generator (Linux x64)
// ══════════════════════════════════════════════════════════════
fn emit_elf(program: &StampedProgram) -> Vec<u8> {
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

    // ELF Header
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

    // Program Header: .text
    bin.extend_from_slice(&PT_LOAD.to_le_bytes());
    bin.extend_from_slice(&(PF_R | PF_X).to_le_bytes());
    bin.extend_from_slice(&text_offset_aligned.to_le_bytes());
    bin.extend_from_slice(&(base_addr + text_offset_aligned).to_le_bytes());
    bin.extend_from_slice(&(base_addr + text_offset_aligned).to_le_bytes());
    bin.extend_from_slice(&text_size.to_le_bytes());
    bin.extend_from_slice(&text_size.to_le_bytes());
    bin.extend_from_slice(&0x1000u64.to_le_bytes());

    // Program Header: .data
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

// ── Po Generator (FastOS) ─────────────────────────────────────
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
