// ============================================================
// ADead-BIB - PE ISA Direct (Ultra-Compacto con Imports)
// ============================================================
// Genera PE mínimo usando ISA layer directamente.
// Soporta imports (printf/scanf) con headers optimizados.
//
// Objetivo: ~1KB con funcionalidad completa (printf)
//
// Técnicas:
// - FileAlignment = 0x200 (mínimo válido)
// - SectionAlignment = 0x200 (igual que FileAlignment)
// - Headers + código en una sola página
// - .idata inline con .text cuando posible
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use std::fs::File;
use std::io::Write;

/// Genera un PE compacto con soporte de imports usando ISA directo.
/// Target: ~1KB para Hello World con printf.
pub fn generate_pe_isa(
    code: &[u8],
    data: &[u8],
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut pe = Vec::new();

    // ============================================
    // Layout del PE ISA Direct
    // ============================================
    // 0x000 - 0x03F: DOS Header (64 bytes)
    // 0x040 - 0x043: PE Signature (4 bytes)
    // 0x044 - 0x057: COFF Header (20 bytes)
    // 0x058 - 0x147: Optional Header (240 bytes)
    // 0x148 - 0x16F: Section .text (40 bytes)
    // 0x170 - 0x197: Section .idata (40 bytes)
    // 0x198 - 0x1FF: Padding (104 bytes)
    // 0x200 - 0x3FF: .text section (code) - 512 bytes
    // 0x400 - 0x5FF: .idata section (imports + strings) - 512 bytes
    // Total: 0x600 = 1536 bytes (1.5 KB)

    const FILE_ALIGN: u32 = 0x200;
    const SECTION_ALIGN: u32 = 0x200;
    const IMAGE_BASE: u64 = 0x0000000140000000;

    let code_size = code.len() as u32;
    let code_raw_size = align_up(code_size, FILE_ALIGN);

    // .idata layout:
    // 0x00-0x27: IDT (2 entries * 20 bytes = 40 bytes)
    // 0x28-0x3F: ILT (3 entries * 8 bytes = 24 bytes)
    // 0x40-0x57: IAT (3 entries * 8 bytes = 24 bytes)
    // 0x58-0x67: DLL name "msvcrt.dll\0" + padding
    // 0x68-0x77: "printf\0" hint/name
    // 0x78-0x87: "scanf\0" hint/name
    // 0x88+: Program strings (data)

    let idata_base_size = 0x88;
    let idata_total = idata_base_size + data.len();
    let idata_raw_size = align_up(idata_total as u32, FILE_ALIGN);

    // RVAs
    let text_rva: u32 = 0x200;
    let idata_rva: u32 = text_rva + code_raw_size;
    let iat_rva: u32 = idata_rva + 0x40;

    // ============================================
    // DOS Header (64 bytes)
    // ============================================
    let mut dos = [0u8; 64];
    dos[0] = 0x4D; // 'M'
    dos[1] = 0x5A; // 'Z'
    dos[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes()); // e_lfanew
    pe.extend_from_slice(&dos);

    // ============================================
    // PE Signature (4 bytes)
    // ============================================
    pe.extend_from_slice(b"PE\0\0");

    // ============================================
    // COFF Header (20 bytes)
    // ============================================
    let mut coff = [0u8; 20];
    coff[0..2].copy_from_slice(&0x8664u16.to_le_bytes()); // Machine: x64
    coff[2..4].copy_from_slice(&2u16.to_le_bytes()); // NumberOfSections: 2
    coff[16..18].copy_from_slice(&240u16.to_le_bytes()); // SizeOfOptionalHeader
    coff[18..20].copy_from_slice(&0x0022u16.to_le_bytes()); // Characteristics
    pe.extend_from_slice(&coff);

    // ============================================
    // Optional Header PE32+ (240 bytes)
    // ============================================
    let mut opt = [0u8; 240];

    // Magic
    opt[0..2].copy_from_slice(&0x020Bu16.to_le_bytes()); // PE32+
    opt[2] = 14; // Linker version major

    // SizeOfCode
    opt[4..8].copy_from_slice(&code_raw_size.to_le_bytes());

    // SizeOfInitializedData
    opt[8..12].copy_from_slice(&idata_raw_size.to_le_bytes());

    // AddressOfEntryPoint
    opt[16..20].copy_from_slice(&text_rva.to_le_bytes());

    // BaseOfCode
    opt[20..24].copy_from_slice(&text_rva.to_le_bytes());

    // ImageBase
    opt[24..32].copy_from_slice(&IMAGE_BASE.to_le_bytes());

    // SectionAlignment
    opt[32..36].copy_from_slice(&SECTION_ALIGN.to_le_bytes());

    // FileAlignment
    opt[36..40].copy_from_slice(&FILE_ALIGN.to_le_bytes());

    // OS Version
    opt[40..42].copy_from_slice(&6u16.to_le_bytes()); // MajorOSVersion
    opt[48..50].copy_from_slice(&6u16.to_le_bytes()); // MajorSubsystemVersion

    // SizeOfImage
    let size_of_image = idata_rva + align_up(idata_raw_size, SECTION_ALIGN);
    opt[56..60].copy_from_slice(&size_of_image.to_le_bytes());

    // SizeOfHeaders
    opt[60..64].copy_from_slice(&0x200u32.to_le_bytes());

    // Subsystem: CUI
    opt[68..70].copy_from_slice(&3u16.to_le_bytes());

    // DLL Characteristics
    opt[70..72].copy_from_slice(&0x8160u16.to_le_bytes()); // ASLR, DEP, etc.

    // Stack/Heap
    opt[72..80].copy_from_slice(&0x100000u64.to_le_bytes()); // StackReserve
    opt[80..88].copy_from_slice(&0x1000u64.to_le_bytes()); // StackCommit
    opt[88..96].copy_from_slice(&0x100000u64.to_le_bytes()); // HeapReserve
    opt[96..104].copy_from_slice(&0x1000u64.to_le_bytes()); // HeapCommit

    // NumberOfRvaAndSizes
    opt[108..112].copy_from_slice(&16u32.to_le_bytes());

    // Data Directory [1]: Import Table
    opt[120..124].copy_from_slice(&idata_rva.to_le_bytes()); // RVA
    opt[124..128].copy_from_slice(&40u32.to_le_bytes()); // Size (IDT)

    // Data Directory [12]: IAT
    opt[192..196].copy_from_slice(&iat_rva.to_le_bytes()); // RVA
    opt[196..200].copy_from_slice(&24u32.to_le_bytes()); // Size

    pe.extend_from_slice(&opt);

    // ============================================
    // Section Headers
    // ============================================

    // .text section header
    let mut sec_text = [0u8; 40];
    sec_text[0..5].copy_from_slice(b".text");
    sec_text[8..12].copy_from_slice(&code_size.to_le_bytes()); // VirtualSize
    sec_text[12..16].copy_from_slice(&text_rva.to_le_bytes()); // VirtualAddress
    sec_text[16..20].copy_from_slice(&code_raw_size.to_le_bytes()); // SizeOfRawData
    sec_text[20..24].copy_from_slice(&0x200u32.to_le_bytes()); // PointerToRawData
    sec_text[36..40].copy_from_slice(&0x60000020u32.to_le_bytes()); // Characteristics
    pe.extend_from_slice(&sec_text);

    // .idata section header
    let mut sec_idata = [0u8; 40];
    sec_idata[0..6].copy_from_slice(b".idata");
    sec_idata[8..12].copy_from_slice(&(idata_total as u32).to_le_bytes()); // VirtualSize
    sec_idata[12..16].copy_from_slice(&idata_rva.to_le_bytes()); // VirtualAddress
    sec_idata[16..20].copy_from_slice(&idata_raw_size.to_le_bytes()); // SizeOfRawData
    let idata_file_offset = 0x200 + code_raw_size;
    sec_idata[20..24].copy_from_slice(&idata_file_offset.to_le_bytes()); // PointerToRawData
    sec_idata[36..40].copy_from_slice(&0xC0000040u32.to_le_bytes()); // Characteristics
    pe.extend_from_slice(&sec_idata);

    // ============================================
    // Padding to 0x200
    // ============================================
    let headers_size = pe.len();
    let padding = 0x200 - headers_size;
    pe.extend_from_slice(&vec![0u8; padding]);

    // ============================================
    // .text section (code)
    // ============================================
    pe.extend_from_slice(code);
    let text_padding = code_raw_size as usize - code.len();
    pe.extend_from_slice(&vec![0u8; text_padding]);

    // ============================================
    // .idata section (imports)
    // ============================================
    let mut idata = vec![0u8; idata_raw_size as usize];

    // IDT[0] for msvcrt.dll
    let ilt_rva = idata_rva + 0x28;
    let dll_name_rva = idata_rva + 0x58;

    idata[0..4].copy_from_slice(&ilt_rva.to_le_bytes()); // OriginalFirstThunk
    idata[12..16].copy_from_slice(&dll_name_rva.to_le_bytes()); // Name
    idata[16..20].copy_from_slice(&iat_rva.to_le_bytes()); // FirstThunk
                                                           // IDT[1] = null terminator (already zeros)

    // ILT entries
    let printf_hint_rva = idata_rva + 0x68;
    let scanf_hint_rva = idata_rva + 0x78;

    idata[0x28..0x30].copy_from_slice(&(printf_hint_rva as u64).to_le_bytes());
    idata[0x30..0x38].copy_from_slice(&(scanf_hint_rva as u64).to_le_bytes());
    // ILT[2] = null

    // IAT entries (same as ILT initially)
    idata[0x40..0x48].copy_from_slice(&(printf_hint_rva as u64).to_le_bytes());
    idata[0x48..0x50].copy_from_slice(&(scanf_hint_rva as u64).to_le_bytes());
    // IAT[2] = null

    // DLL name
    idata[0x58..0x63].copy_from_slice(b"msvcrt.dll\0");

    // printf hint/name
    idata[0x68..0x6A].copy_from_slice(&0u16.to_le_bytes()); // Hint
    idata[0x6A..0x71].copy_from_slice(b"printf\0");

    // scanf hint/name
    idata[0x78..0x7A].copy_from_slice(&0u16.to_le_bytes()); // Hint
    idata[0x7A..0x80].copy_from_slice(b"scanf\0");

    // Program strings
    if !data.is_empty() {
        idata[0x88..0x88 + data.len()].copy_from_slice(data);
    }

    pe.extend_from_slice(&idata);

    // ============================================
    // Write to file
    // ============================================
    let mut file = File::create(output_path)?;
    file.write_all(&pe)?;

    Ok(pe.len())
}

/// Genera código con offsets IAT corregidos para el nuevo layout.
/// Recalcula los offsets RIP-relative para printf/scanf.
pub fn patch_iat_calls(code: &mut [u8], code_rva: u32, iat_printf_rva: u32, iat_scanf_rva: u32) {
    // Buscar patrones de call [rip+disp32] (FF 15 xx xx xx xx)
    let mut i = 0;
    while i + 5 < code.len() {
        if code[i] == 0xFF && code[i + 1] == 0x15 {
            // Leer el offset actual
            let current_offset =
                i32::from_le_bytes([code[i + 2], code[i + 3], code[i + 4], code[i + 5]]);

            // Calcular RVA del final de esta instrucción
            let call_end_rva = code_rva + (i as u32) + 6;

            // Determinar si es printf o scanf basado en el offset esperado
            // Si el offset apunta cerca de 0x2040, es printf del layout antiguo
            let target_rva = (call_end_rva as i64 + current_offset as i64) as u32;

            let new_offset = if target_rva >= 0x2040 && target_rva < 0x2048 {
                // printf
                iat_printf_rva as i32 - call_end_rva as i32
            } else if target_rva >= 0x2048 && target_rva < 0x2050 {
                // scanf
                iat_scanf_rva as i32 - call_end_rva as i32
            } else {
                // Mantener el offset original
                current_offset
            };

            code[i + 2..i + 6].copy_from_slice(&new_offset.to_le_bytes());
            i += 6;
        } else {
            i += 1;
        }
    }
}

#[inline]
fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(100, 512), 512);
        assert_eq!(align_up(512, 512), 512);
        assert_eq!(align_up(513, 512), 1024);
    }
}
