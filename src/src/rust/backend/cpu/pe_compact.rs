// ============================================================
// ADead-BIB - PE Compact (Optimizado para tamaño)
// ============================================================
// PE con SectionAlignment = FileAlignment = 0x200
// Reduce significativamente el tamaño del binario.
//
// Layout:
// 0x000-0x1FF: Headers (512 bytes)
// 0x200-0x3FF: .text (código) - 512 bytes
// 0x400-0x5FF: .idata (imports + strings) - 512 bytes
// Total: 1536 bytes (1.5 KB) para Hello World
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use std::fs::File;
use std::io::Write;

/// Genera un PE compacto con SectionAlignment = FileAlignment = 0x200
pub fn generate_pe_compact(
    opcodes: &[u8],
    data: &[u8],
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut file = File::create(output_path)?;

    const FILE_ALIGN: u32 = 0x200;
    const SECTION_ALIGN: u32 = 0x200; // Clave: igual que FileAlignment
    const IMAGE_BASE: u64 = 0x0000000140000000;

    // Calcular tamaños alineados
    let code_raw_size = align_up(opcodes.len() as u32, FILE_ALIGN);
    let code_virtual_size = opcodes.len() as u32;

    // Para mantener compatibilidad con el ISA compiler que asume:
    // - code_rva = 0x1000
    // - idata_rva = 0x2000
    // - strings en idata_rva + 0x78 = 0x2078
    // Usamos el mismo layout pero con FileAlignment más pequeño

    let _text_rva: u32 = 0x1000;
    let idata_rva: u32 = 0x2000;
    let iat_rva: u32 = idata_rva + 0x40;

    // Construir .idata (mismo layout que pe.rs)
    let mut idata = vec![0u8; FILE_ALIGN as usize];

    // IDT[0] for msvcrt.dll
    idata[0..4].copy_from_slice(&(idata_rva + 0x28).to_le_bytes()); // OriginalFirstThunk
    idata[12..16].copy_from_slice(&(idata_rva + 0x58).to_le_bytes()); // Name
    idata[16..20].copy_from_slice(&(idata_rva + 0x40).to_le_bytes()); // FirstThunk (IAT)

    // ILT entries
    let printf_hint_rva = idata_rva + 0x64;
    let scanf_hint_rva = idata_rva + 0x6E;

    idata[0x28..0x30].copy_from_slice(&(printf_hint_rva as u64).to_le_bytes());
    idata[0x30..0x38].copy_from_slice(&(scanf_hint_rva as u64).to_le_bytes());

    // IAT entries
    idata[0x40..0x48].copy_from_slice(&(printf_hint_rva as u64).to_le_bytes());
    idata[0x48..0x50].copy_from_slice(&(scanf_hint_rva as u64).to_le_bytes());

    // Strings
    idata[0x58..0x63].copy_from_slice(b"msvcrt.dll\0");
    idata[0x64..0x66].copy_from_slice(&0u16.to_le_bytes()); // Hint
    idata[0x66..0x6D].copy_from_slice(b"printf\0");
    idata[0x6E..0x70].copy_from_slice(&0u16.to_le_bytes()); // Hint
    idata[0x70..0x76].copy_from_slice(b"scanf\0");

    // Program strings at 0x78 (mismo offset que pe.rs)
    let strings_offset = 0x78usize;
    if strings_offset + data.len() <= idata.len() {
        idata[strings_offset..strings_offset + data.len()].copy_from_slice(data);
    } else {
        let needed = strings_offset + data.len();
        let aligned = align_up(needed as u32, FILE_ALIGN) as usize;
        idata.resize(aligned, 0);
        idata[strings_offset..strings_offset + data.len()].copy_from_slice(data);
    }

    let idata_raw_size = idata.len() as u32;
    let idata_virtual_size = idata_raw_size;

    // Parchear código con nuevos offsets IAT
    let mut patched_code = opcodes.to_vec();
    patch_iat_offsets(&mut patched_code, 0x200, iat_rva, iat_rva + 8);

    // ============================================
    // Headers
    // ============================================

    // DOS Header (64 bytes)
    let mut dos = vec![0u8; 64];
    dos[0] = 0x4D; // 'M'
    dos[1] = 0x5A; // 'Z'
    dos[0x3C] = 0x40; // e_lfanew
    file.write_all(&dos)?;

    // PE Signature
    file.write_all(b"PE\0\0")?;

    // COFF Header (20 bytes)
    let mut coff = vec![0u8; 20];
    coff[0..2].copy_from_slice(&0x8664u16.to_le_bytes()); // x64
    coff[2..4].copy_from_slice(&2u16.to_le_bytes()); // NumberOfSections: 2
    coff[16..18].copy_from_slice(&240u16.to_le_bytes()); // SizeOfOptionalHeader
    coff[18..20].copy_from_slice(&0x0022u16.to_le_bytes()); // Characteristics
    file.write_all(&coff)?;

    // Optional Header (240 bytes)
    let mut opt = vec![0u8; 240];
    opt[0..2].copy_from_slice(&0x020Bu16.to_le_bytes()); // PE32+
    opt[2] = 14; // Linker version
    opt[4..8].copy_from_slice(&code_raw_size.to_le_bytes()); // SizeOfCode
    opt[8..12].copy_from_slice(&idata_raw_size.to_le_bytes()); // SizeOfInitializedData
    opt[16..20].copy_from_slice(&0x200u32.to_le_bytes()); // AddressOfEntryPoint
    opt[20..24].copy_from_slice(&0x200u32.to_le_bytes()); // BaseOfCode
    opt[24..32].copy_from_slice(&IMAGE_BASE.to_le_bytes()); // ImageBase
    opt[32..36].copy_from_slice(&SECTION_ALIGN.to_le_bytes()); // SectionAlignment
    opt[36..40].copy_from_slice(&FILE_ALIGN.to_le_bytes()); // FileAlignment
    opt[40..42].copy_from_slice(&6u16.to_le_bytes()); // MajorOSVersion
    opt[48..50].copy_from_slice(&6u16.to_le_bytes()); // MajorSubsystemVersion

    // SizeOfImage
    let size_of_image = 0x200 + code_raw_size + idata_raw_size;
    opt[56..60].copy_from_slice(&size_of_image.to_le_bytes());
    opt[60..64].copy_from_slice(&0x200u32.to_le_bytes()); // SizeOfHeaders
    opt[68..70].copy_from_slice(&3u16.to_le_bytes()); // Subsystem CUI
    opt[72..80].copy_from_slice(&0x100000u64.to_le_bytes()); // StackReserve
    opt[80..88].copy_from_slice(&0x1000u64.to_le_bytes()); // StackCommit
    opt[88..96].copy_from_slice(&0x100000u64.to_le_bytes()); // HeapReserve
    opt[96..104].copy_from_slice(&0x1000u64.to_le_bytes()); // HeapCommit
    opt[108..112].copy_from_slice(&16u32.to_le_bytes()); // NumberOfRvaAndSizes

    // Data Directory [1] Import Table
    opt[120..124].copy_from_slice(&idata_rva.to_le_bytes());
    opt[124..128].copy_from_slice(&40u32.to_le_bytes());

    // Data Directory [12] IAT
    opt[192..196].copy_from_slice(&iat_rva.to_le_bytes());
    opt[196..200].copy_from_slice(&24u32.to_le_bytes());

    file.write_all(&opt)?;

    // Section Headers
    // .text
    let mut sec_text = vec![0u8; 40];
    sec_text[0..5].copy_from_slice(b".text");
    sec_text[8..12].copy_from_slice(&code_virtual_size.to_le_bytes());
    sec_text[12..16].copy_from_slice(&0x200u32.to_le_bytes()); // VirtualAddress
    sec_text[16..20].copy_from_slice(&code_raw_size.to_le_bytes());
    sec_text[20..24].copy_from_slice(&0x200u32.to_le_bytes()); // PointerToRawData
    sec_text[36..40].copy_from_slice(&0x60000020u32.to_le_bytes());
    file.write_all(&sec_text)?;

    // .idata
    let mut sec_idata = vec![0u8; 40];
    sec_idata[0..6].copy_from_slice(b".idata");
    sec_idata[8..12].copy_from_slice(&idata_virtual_size.to_le_bytes());
    sec_idata[12..16].copy_from_slice(&idata_rva.to_le_bytes());
    sec_idata[16..20].copy_from_slice(&idata_raw_size.to_le_bytes());
    let idata_file_ptr = 0x200 + code_raw_size;
    sec_idata[20..24].copy_from_slice(&idata_file_ptr.to_le_bytes());
    sec_idata[36..40].copy_from_slice(&0xC0000040u32.to_le_bytes());
    file.write_all(&sec_idata)?;

    // Padding to 0x200
    let headers_size = 64 + 4 + 20 + 240 + 40 + 40;
    let padding = 0x200 - headers_size;
    file.write_all(&vec![0u8; padding])?;

    // Write .text
    file.write_all(&patched_code)?;
    let text_padding = code_raw_size as usize - patched_code.len();
    file.write_all(&vec![0u8; text_padding])?;

    // Write .idata
    file.write_all(&idata)?;

    let total_size = 0x200 + code_raw_size as usize + idata.len();
    Ok(total_size)
}

/// Parchea los offsets de llamadas IAT en el código.
/// El encoder genera código asumiendo code_rva=0x1000 y IAT en 0x2040/0x2048.
/// Este parcheo recalcula para el nuevo layout con code_rva=0x200.
fn patch_iat_offsets(
    code: &mut [u8],
    new_code_rva: u32,
    new_printf_iat_rva: u32,
    new_scanf_iat_rva: u32,
) {
    // El encoder original asume:
    const OLD_CODE_RVA: u32 = 0x1000;
    const OLD_PRINTF_IAT: u32 = 0x2040;
    const OLD_SCANF_IAT: u32 = 0x2048;

    let mut i = 0;
    while i + 5 < code.len() {
        if code[i] == 0xFF && code[i + 1] == 0x15 {
            let current_offset =
                i32::from_le_bytes([code[i + 2], code[i + 3], code[i + 4], code[i + 5]]);

            // Calcular qué IAT apuntaba originalmente
            let old_call_end_rva = OLD_CODE_RVA + (i as u32) + 6;
            let old_target = (old_call_end_rva as i64 + current_offset as i64) as u32;

            // Calcular nuevo offset basado en el nuevo layout
            let new_call_end_rva = new_code_rva + (i as u32) + 6;

            let new_offset = if old_target == OLD_PRINTF_IAT {
                new_printf_iat_rva as i32 - new_call_end_rva as i32
            } else if old_target == OLD_SCANF_IAT {
                new_scanf_iat_rva as i32 - new_call_end_rva as i32
            } else {
                // No es una llamada IAT conocida, mantener
                current_offset
            };

            code[i + 2..i + 6].copy_from_slice(&new_offset.to_le_bytes());
            i += 6;
        } else {
            i += 1;
        }
    }
}

/// Calcula dirección de string en el nuevo layout
pub fn get_string_address_compact(string_offset: u64, idata_rva: u32, image_base: u64) -> u64 {
    image_base + idata_rva as u64 + 0x78 + string_offset
}

#[inline]
fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}
