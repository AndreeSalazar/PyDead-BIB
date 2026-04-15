// ============================================================
// PyDead-BIB JIT 2.0 — PE Generator (Windows x64)
// ============================================================

use crate::backend::bg::StampedProgram;
use crate::backend::isa::{IAT_SLOT_COUNT};

// ── PE Constants ──────────────────────────────────────────────
pub const PE_SIGNATURE: u32 = 0x00004550;
pub const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
pub const IMAGE_FILE_EXECUTABLE_IMAGE: u16 = 0x0002;
pub const IMAGE_FILE_LARGE_ADDRESS_AWARE: u16 = 0x0020;
pub const OPTIONAL_HEADER_MAGIC_PE32PLUS: u16 = 0x020B;
pub const IMAGE_SUBSYSTEM_CONSOLE: u16 = 3;
pub const SECTION_ALIGNMENT: u32 = 0x1000;
pub const FILE_ALIGNMENT: u32 = 0x200;

const IMPORT_FUNCS: [&str; IAT_SLOT_COUNT] = [
    "GetStdHandle", "WriteFile", "ExitProcess", "GetProcessHeap", "HeapAlloc",
    "GetCurrentDirectoryA", "GetFileAttributesA", "GetCurrentProcessId", "CreateFileA",
    "ReadFile", "CloseHandle", "CreateDirectoryA", "DeleteFileA", "MoveFileA",
    "FindFirstFileA", "FindNextFileA", "FindClose", "GetEnvironmentVariableA",
    "GetCommandLineA", "GetFileSizeEx", "LoadLibraryA", "GetProcAddress", "FreeLibrary",
];
const IMPORT_DLL: &str = "KERNEL32.dll";

pub fn align_up(val: u32, alignment: u32) -> u32 {
    (val + alignment - 1) & !(alignment - 1)
}

pub fn emit_pe(program: &StampedProgram) -> Vec<u8> {
    let image_base: u64 = 0x0000000140000000;

    let idata = build_idata(IAT_SLOT_COUNT);

    let num_sections: u16 = 3;
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

    let iat_rva = rdata_rva + idata.iat_offset as u32;
    let iat_size = (IAT_SLOT_COUNT + 1) as u32 * 8;
    let import_dir_rva = rdata_rva;

    let mut text = program.text.clone();

    for &(fixup_offset, slot_idx) in &program.iat_fixups {
        let iat_entry_rva = iat_rva + (slot_idx as u32) * 8;
        let instr_rva = text_rva + fixup_offset;
        let disp32 = (iat_entry_rva as i32) - (instr_rva as i32 + 4);
        let off = fixup_offset as usize;
        if off + 4 <= text.len() {
            text[off..off+4].copy_from_slice(&disp32.to_le_bytes());
        }
    }

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

    let mut bin = Vec::with_capacity(size_of_image as usize);

    let mut dos = vec![0u8; 64];
    dos[0] = 0x4D; dos[1] = 0x5A;
    dos[0x3C..0x40].copy_from_slice(&64u32.to_le_bytes());
    bin.extend_from_slice(&dos);

    bin.extend_from_slice(&PE_SIGNATURE.to_le_bytes());
    bin.extend_from_slice(&IMAGE_FILE_MACHINE_AMD64.to_le_bytes());
    bin.extend_from_slice(&num_sections.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&(opt_hdr_size as u16).to_le_bytes());
    let chars = IMAGE_FILE_EXECUTABLE_IMAGE | IMAGE_FILE_LARGE_ADDRESS_AWARE;
    bin.extend_from_slice(&chars.to_le_bytes());

    bin.extend_from_slice(&OPTIONAL_HEADER_MAGIC_PE32PLUS.to_le_bytes());
    bin.push(1); bin.push(0);
    bin.extend_from_slice(&text_raw_size.to_le_bytes());
    bin.extend_from_slice(&(rdata_raw_size + data_raw_size).to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&entry_rva.to_le_bytes());
    bin.extend_from_slice(&text_rva.to_le_bytes());
    bin.extend_from_slice(&image_base.to_le_bytes());
    bin.extend_from_slice(&SECTION_ALIGNMENT.to_le_bytes());
    bin.extend_from_slice(&FILE_ALIGNMENT.to_le_bytes());
    bin.extend_from_slice(&6u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&6u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&size_of_image.to_le_bytes());
    bin.extend_from_slice(&size_of_headers.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&IMAGE_SUBSYSTEM_CONSOLE.to_le_bytes());
    bin.extend_from_slice(&0x0100u16.to_le_bytes());
    
    bin.extend_from_slice(&0x100000u64.to_le_bytes());
    bin.extend_from_slice(&0x1000u64.to_le_bytes());
    bin.extend_from_slice(&0x100000u64.to_le_bytes());
    bin.extend_from_slice(&0x1000u64.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&16u32.to_le_bytes());

    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&import_dir_rva.to_le_bytes());
    bin.extend_from_slice(&(idata.import_dir_size as u32).to_le_bytes());
    for _ in 2..12 {
        bin.extend_from_slice(&0u32.to_le_bytes());
        bin.extend_from_slice(&0u32.to_le_bytes());
    }
    bin.extend_from_slice(&iat_rva.to_le_bytes());
    bin.extend_from_slice(&iat_size.to_le_bytes());
    for _ in 13..16 {
        bin.extend_from_slice(&0u32.to_le_bytes());
        bin.extend_from_slice(&0u32.to_le_bytes());
    }

    write_section_header(&mut bin, b".text\0\0\0",
        program.text.len() as u32, text_rva, text_raw_size, text_file_off,
        0x60000020);

    write_section_header(&mut bin, b".rdata\0\0",
        idata.total_size as u32, rdata_rva, rdata_raw_size, rdata_file_off,
        0x40000040);

    write_section_header(&mut bin, b".data\0\0\0",
        data_virt_size, data_rva, data_raw_size, data_file_off,
        0xC0000040);

    while bin.len() < size_of_headers as usize { bin.push(0); }

    bin.extend_from_slice(&text);
    while bin.len() < (text_file_off + text_raw_size) as usize { bin.push(0xCC); }

    let rdata_bytes = build_idata_bytes(&idata, rdata_rva);
    bin.extend_from_slice(&rdata_bytes);
    while bin.len() < (rdata_file_off + rdata_raw_size) as usize { bin.push(0); }

    bin.extend_from_slice(&bg_stamp_bytes);
    bin.extend_from_slice(&program.data);
    while bin.len() < (data_file_off + data_raw_size) as usize { bin.push(0); }

    bin
}

struct IdataLayout {
    import_dir_size: usize,
    ilt_offset: usize,
    iat_offset: usize,
    hints_offset: usize,
    dll_name_offset: usize,
    total_size: usize,
}

fn build_idata(num_funcs: usize) -> IdataLayout {
    let import_dir_size = 40;
    let ilt_offset = import_dir_size;
    let ilt_size = (num_funcs + 1) * 8;
    let iat_offset = ilt_offset + ilt_size;
    let iat_size = (num_funcs + 1) * 8;
    let hints_offset = iat_offset + iat_size;

    let mut hints_size = 0;
    for func_name in &IMPORT_FUNCS {
        hints_size += 2;
        hints_size += func_name.len() + 1;
        if hints_size % 2 != 0 { hints_size += 1; }
    }

    let dll_name_offset = hints_offset + hints_size;
    let dll_name_size = IMPORT_DLL.len() + 1;
    let total_size = dll_name_offset + dll_name_size;

    IdataLayout {
        import_dir_size, ilt_offset, iat_offset, hints_offset, dll_name_offset, total_size,
    }
}

fn build_idata_bytes(layout: &IdataLayout, rdata_rva: u32) -> Vec<u8> {
    let mut buf = vec![0u8; layout.total_size];

    let ilt_rva = rdata_rva + layout.ilt_offset as u32;
    let iat_rva = rdata_rva + layout.iat_offset as u32;
    let dll_name_rva = rdata_rva + layout.dll_name_offset as u32;

    buf[0..4].copy_from_slice(&ilt_rva.to_le_bytes());
    buf[4..8].copy_from_slice(&0u32.to_le_bytes());
    buf[8..12].copy_from_slice(&0u32.to_le_bytes());
    buf[12..16].copy_from_slice(&dll_name_rva.to_le_bytes());
    buf[16..20].copy_from_slice(&iat_rva.to_le_bytes());

    let mut hint_off = layout.hints_offset;
    for (i, func_name) in IMPORT_FUNCS.iter().enumerate() {
        let hint_rva = rdata_rva + hint_off as u32;

        let ilt_entry_off = layout.ilt_offset + i * 8;
        buf[ilt_entry_off..ilt_entry_off+8].copy_from_slice(&(hint_rva as u64).to_le_bytes());

        let iat_entry_off = layout.iat_offset + i * 8;
        buf[iat_entry_off..iat_entry_off+8].copy_from_slice(&(hint_rva as u64).to_le_bytes());

        buf[hint_off] = 0; buf[hint_off+1] = 0;
        hint_off += 2;
        for &b in func_name.as_bytes() {
            buf[hint_off] = b;
            hint_off += 1;
        }
        buf[hint_off] = 0;
        hint_off += 1;
        if hint_off % 2 != 0 {
            buf[hint_off] = 0;
            hint_off += 1;
        }
    }

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
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&0u16.to_le_bytes());
    bin.extend_from_slice(&chars.to_le_bytes());
}
