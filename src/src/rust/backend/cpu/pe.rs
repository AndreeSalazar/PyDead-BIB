// PE (Portable Executable) Generator
// Genera binarios Windows .exe funcionales con soporte multi-DLL imports
// Versión v2.0: Dynamic IAT via iat_registry

use std::fs::File;
use std::io::Write;
use super::iat_registry;

pub fn generate_pe(
    opcodes: &[u8],
    _data: &[u8],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    generate_pe_with_offsets(opcodes, _data, output_path, &[], &[])
}

/// Generate PE bytes in memory (no file I/O) — used by integration tests
pub fn generate_pe_bytes(
    opcodes: &[u8],
    data: &[u8],
    iat_call_offsets: &[usize],
    string_imm64_offsets: &[usize],
) -> Vec<u8> {
    build_pe_bytes(opcodes, data, iat_call_offsets, string_imm64_offsets, 3) // CUI
}

/// FASM-inspired PE generator with precise offset tracking.
/// Uses dynamic multi-DLL IAT via iat_registry.
pub fn generate_pe_with_offsets(
    opcodes: &[u8],
    data: &[u8],
    output_path: &str,
    iat_call_offsets: &[usize],
    string_imm64_offsets: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    let pe = build_pe_bytes(opcodes, data, iat_call_offsets, string_imm64_offsets, 3); // CUI
    let mut file = File::create(output_path)?;
    file.write_all(&pe)?;
    Ok(())
}

/// Generate PE for Windows GUI subsystem (subsystem=2)
pub fn generate_pe_gui(
    opcodes: &[u8],
    data: &[u8],
    output_path: &str,
    iat_call_offsets: &[usize],
    string_imm64_offsets: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    let pe = build_pe_bytes(opcodes, data, iat_call_offsets, string_imm64_offsets, 2); // GUI
    let mut file = File::create(output_path)?;
    file.write_all(&pe)?;
    Ok(())
}

fn build_pe_bytes(
    opcodes: &[u8],
    data: &[u8],
    iat_call_offsets: &[usize],
    string_imm64_offsets: &[usize],
    subsystem: u16,
) -> Vec<u8> {
    let file_align: usize = 0x200;
    let section_align: u32 = 0x1000;
    let image_base: u64 = 0x0000000140000000;

    let code_raw_size = ((opcodes.len() + file_align - 1) / file_align * file_align) as u32;
    let code_virtual_size = opcodes.len() as u32;
    let text_virtual_pages = (code_virtual_size + section_align - 1) / section_align;
    let idata_rva: u32 = 0x1000 + text_virtual_pages * section_align;

    // Build .idata dynamically using iat_registry
    let idata_result = iat_registry::build_idata(idata_rva, data);
    let idata = &idata_result.idata;
    let idata_raw_size = idata.len() as u32;
    let idata_virtual_size = idata_raw_size;
    let idata_virtual_pages = (idata_virtual_size + section_align - 1) / section_align;
    let size_of_image = idata_rva + idata_virtual_pages * section_align;

    // Patch opcodes: ISA compiler emits CallIAT with RVAs based on assumed idata_rva=0x2000.
    // We compute the real IAT RVAs and create a delta for patching.
    let mut patched_opcodes = opcodes.to_vec();

    // The ISA compiler assumes idata_rva = 0x2000 and uses slot_to_iat_rva from that base.
    // We need to compute what the ISA compiler thought the IAT RVAs were,
    // then patch them to the actual values.
    let assumed_idata_rva: u32 = 0x2000;
    let assumed_result = iat_registry::build_idata(assumed_idata_rva, &[]);
    let iat_delta = idata_result.slot_to_iat_rva[0] as i32 - assumed_result.slot_to_iat_rva[0] as i32;

    let old_string_base = image_base + assumed_idata_rva as u64 + idata_result.program_strings_offset as u64;
    let new_string_base = image_base + idata_rva as u64 + idata_result.program_strings_offset as u64;
    let string_delta = new_string_base as i64 - old_string_base as i64;

    if iat_delta != 0 || string_delta != 0 {
        if !iat_call_offsets.is_empty() || !string_imm64_offsets.is_empty() {
            for &offset in iat_call_offsets {
                if offset + 4 <= patched_opcodes.len() {
                    let old_val = i32::from_le_bytes([
                        patched_opcodes[offset], patched_opcodes[offset + 1],
                        patched_opcodes[offset + 2], patched_opcodes[offset + 3],
                    ]);
                    patched_opcodes[offset..offset + 4]
                        .copy_from_slice(&(old_val + iat_delta).to_le_bytes());
                }
            }
            for &offset in string_imm64_offsets {
                if offset + 8 <= patched_opcodes.len() {
                    let imm64 = u64::from_le_bytes([
                        patched_opcodes[offset], patched_opcodes[offset + 1],
                        patched_opcodes[offset + 2], patched_opcodes[offset + 3],
                        patched_opcodes[offset + 4], patched_opcodes[offset + 5],
                        patched_opcodes[offset + 6], patched_opcodes[offset + 7],
                    ]);
                    if imm64 >= old_string_base && imm64 < old_string_base + 0x10000 {
                        let new_imm64 = (imm64 as i64 + string_delta) as u64;
                        patched_opcodes[offset..offset + 8]
                            .copy_from_slice(&new_imm64.to_le_bytes());
                    }
                }
            }
        } else {
            // LEGACY MODE: byte-pattern scanning
            let mut i = 0;
            while i < patched_opcodes.len() {
                if i + 5 < patched_opcodes.len()
                    && patched_opcodes[i] == 0xFF
                    && patched_opcodes[i + 1] == 0x15
                {
                    let old_offset = i32::from_le_bytes([
                        patched_opcodes[i + 2], patched_opcodes[i + 3],
                        patched_opcodes[i + 4], patched_opcodes[i + 5],
                    ]);
                    patched_opcodes[i + 2..i + 6]
                        .copy_from_slice(&(old_offset + iat_delta).to_le_bytes());
                    i += 6;
                    continue;
                }
                if i + 9 < patched_opcodes.len() && patched_opcodes[i] == 0x48 {
                    let opcode = patched_opcodes[i + 1];
                    if opcode >= 0xB8 && opcode <= 0xBF {
                        let imm64 = u64::from_le_bytes([
                            patched_opcodes[i + 2], patched_opcodes[i + 3],
                            patched_opcodes[i + 4], patched_opcodes[i + 5],
                            patched_opcodes[i + 6], patched_opcodes[i + 7],
                            patched_opcodes[i + 8], patched_opcodes[i + 9],
                        ]);
                        if imm64 >= old_string_base && imm64 < old_string_base + 0x10000 {
                            let new_imm64 = (imm64 as i64 + string_delta) as u64;
                            patched_opcodes[i + 2..i + 10]
                                .copy_from_slice(&new_imm64.to_le_bytes());
                        }
                        i += 10;
                        continue;
                    }
                }
                i += 1;
            }
        }
    }

    let mut pe = Vec::new();

    // DOS Header (64 bytes)
    let mut dos = vec![0u8; 64];
    dos[0] = 0x4D; dos[1] = 0x5A; dos[0x3C] = 0x40;
    pe.extend_from_slice(&dos);
    pe.extend_from_slice(b"PE\0\0");

    // COFF Header (20 bytes)
    let mut coff = vec![0u8; 20];
    coff[0..2].copy_from_slice(&0x8664u16.to_le_bytes()); // x64
    coff[2..4].copy_from_slice(&2u16.to_le_bytes()); // NumberOfSections: 2
    coff[16..18].copy_from_slice(&240u16.to_le_bytes()); // SizeOfOptionalHeader
    coff[18..20].copy_from_slice(&0x0022u16.to_le_bytes()); // Characteristics
    pe.extend_from_slice(&coff);

    // Optional Header PE32+ (240 bytes)
    let mut opt = vec![0u8; 240];
    opt[0..2].copy_from_slice(&0x020Bu16.to_le_bytes()); // Magic PE32+
    opt[2] = 14; // Linker version
    opt[4..8].copy_from_slice(&code_raw_size.to_le_bytes());
    opt[8..12].copy_from_slice(&idata_raw_size.to_le_bytes());
    opt[16..20].copy_from_slice(&0x1000u32.to_le_bytes()); // EntryPoint
    opt[20..24].copy_from_slice(&0x1000u32.to_le_bytes()); // BaseOfCode
    opt[24..32].copy_from_slice(&image_base.to_le_bytes());
    opt[32..36].copy_from_slice(&section_align.to_le_bytes());
    opt[36..40].copy_from_slice(&(file_align as u32).to_le_bytes());
    opt[40..42].copy_from_slice(&6u16.to_le_bytes()); // MajorOSVersion
    opt[48..50].copy_from_slice(&6u16.to_le_bytes()); // MajorSubsystemVersion
    opt[56..60].copy_from_slice(&size_of_image.to_le_bytes());
    opt[60..64].copy_from_slice(&0x400u32.to_le_bytes()); // SizeOfHeaders
    opt[68..70].copy_from_slice(&subsystem.to_le_bytes());
    opt[70..72].copy_from_slice(&0x0100u16.to_le_bytes()); // DllCharacteristics: NX_COMPAT
    opt[72..80].copy_from_slice(&0x100000u64.to_le_bytes()); // StackReserve
    opt[80..88].copy_from_slice(&0x1000u64.to_le_bytes()); // StackCommit
    opt[88..96].copy_from_slice(&0x100000u64.to_le_bytes()); // HeapReserve
    opt[96..104].copy_from_slice(&0x1000u64.to_le_bytes()); // HeapCommit
    opt[108..112].copy_from_slice(&16u32.to_le_bytes()); // NumberOfRvaAndSizes

    // Data Directory [1] Import Table
    opt[120..124].copy_from_slice(&idata_rva.to_le_bytes());
    opt[124..128].copy_from_slice(&idata_result.idt_size.to_le_bytes());

    // Data Directory [12] IAT
    let iat_rva = idata_rva + idata_result.iat_offset;
    opt[208..212].copy_from_slice(&iat_rva.to_le_bytes());
    opt[212..216].copy_from_slice(&idata_result.total_iat_size.to_le_bytes());

    pe.extend_from_slice(&opt);

    // .text section header
    let mut sec_text = vec![0u8; 40];
    sec_text[0..5].copy_from_slice(b".text");
    sec_text[8..12].copy_from_slice(&code_virtual_size.to_le_bytes());
    sec_text[12..16].copy_from_slice(&0x1000u32.to_le_bytes());
    sec_text[16..20].copy_from_slice(&code_raw_size.to_le_bytes());
    sec_text[20..24].copy_from_slice(&0x400u32.to_le_bytes());
    sec_text[36..40].copy_from_slice(&0x60000020u32.to_le_bytes());
    pe.extend_from_slice(&sec_text);

    // .idata section header
    let mut sec_idata = vec![0u8; 40];
    sec_idata[0..6].copy_from_slice(b".idata");
    sec_idata[8..12].copy_from_slice(&idata_virtual_size.to_le_bytes());
    sec_idata[12..16].copy_from_slice(&idata_rva.to_le_bytes());
    sec_idata[16..20].copy_from_slice(&idata_raw_size.to_le_bytes());
    let idata_ptr = 0x400 + code_raw_size;
    sec_idata[20..24].copy_from_slice(&idata_ptr.to_le_bytes());
    sec_idata[36..40].copy_from_slice(&0xC0000040u32.to_le_bytes());
    pe.extend_from_slice(&sec_idata);

    // Padding to 0x400
    pe.resize(0x400, 0);

    // .text section data
    pe.extend_from_slice(&patched_opcodes);
    let text_padding = code_raw_size as usize - patched_opcodes.len();
    pe.extend(std::iter::repeat(0u8).take(text_padding));

    // .idata section data
    pe.extend_from_slice(idata);

    pe
}
