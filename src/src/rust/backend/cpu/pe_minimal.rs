// PE Generator Mínimo pero Funcional
// Genera un PE válido que Windows acepta

use std::fs::File;
use std::io::Write;

pub fn generate_pe_minimal(
    opcodes: &[u8],
    _data: &[u8],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(output_path)?;

    // DOS Header (64 bytes) - debe empezar con "MZ"
    let mut dos = vec![0u8; 64];
    dos[0] = 0x4D; // 'M'
    dos[1] = 0x5A; // 'Z'
    dos[0x3C] = 0x40; // Offset a PE header (64)
    dos[0x3D] = 0x00;
    file.write_all(&dos)?;

    // PE Signature
    file.write_all(b"PE\0\0")?;

    // COFF Header (20 bytes)
    let mut coff = vec![0u8; 20];
    // Machine: 0x8664 (x64)
    coff[0] = 0x64;
    coff[1] = 0x86;
    // NumberOfSections: 1
    coff[2] = 0x01;
    coff[3] = 0x00;
    // SizeOfOptionalHeader: 240 (0xF0)
    coff[16] = 0xF0;
    coff[17] = 0x00;
    // Characteristics: 0x22 (EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE)
    coff[18] = 0x22;
    coff[19] = 0x00;
    file.write_all(&coff)?;

    // Optional Header PE32+ (240 bytes)
    let mut opt = vec![0u8; 240];
    // Magic: 0x20B (PE32+)
    opt[0] = 0x0B;
    opt[1] = 0x02;
    // MajorLinkerVersion: 14
    opt[2] = 14;
    // SizeOfCode
    let code_size = ((opcodes.len() + 0x1FF) & !0x1FF) as u32; // Alinear a 512
    opt[4..8].copy_from_slice(&code_size.to_le_bytes());
    // AddressOfEntryPoint: 0x1000
    opt[16] = 0x00;
    opt[17] = 0x10;
    opt[18] = 0x00;
    opt[19] = 0x00;
    // BaseOfCode: 0x1000
    opt[20..24].copy_from_slice(&0x1000u32.to_le_bytes());
    // ImageBase: 0x400000
    opt[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
    // SectionAlignment: 0x1000
    opt[32..36].copy_from_slice(&0x1000u32.to_le_bytes());
    // FileAlignment: 0x200
    opt[36..40].copy_from_slice(&0x200u32.to_le_bytes());
    // SizeOfImage
    let size_of_image = 0x2000u32; // 2 páginas
    opt[56..60].copy_from_slice(&size_of_image.to_le_bytes());
    // SizeOfHeaders: 0x400
    opt[60..64].copy_from_slice(&0x400u32.to_le_bytes());
    // Subsystem: 3 (WINDOWS_CUI)
    opt[68] = 0x03;
    opt[69] = 0x00;
    // NumberOfRvaAndSizes: 16
    opt[108..112].copy_from_slice(&16u32.to_le_bytes());
    file.write_all(&opt)?;

    // Section Header .text (40 bytes)
    let mut sec = vec![0u8; 40];
    sec[0..8].copy_from_slice(b".text\0\0\0");
    sec[8..12].copy_from_slice(&code_size.to_le_bytes()); // VirtualSize
    sec[12..16].copy_from_slice(&0x400u32.to_le_bytes()); // VirtualAddress (después de headers)
    sec[16..20].copy_from_slice(&code_size.to_le_bytes()); // SizeOfRawData
    sec[20..24].copy_from_slice(&0x1000u32.to_le_bytes()); // PointerToRawData
                                                           // Characteristics: 0x60000020 (CODE | EXECUTE | READ)
    sec[36..40].copy_from_slice(&0x60000020u32.to_le_bytes());
    file.write_all(&sec)?;

    // Padding hasta 0x400
    let current = 64 + 4 + 20 + 240 + 40;
    file.write_all(&vec![0u8; 0x400 - current])?;

    // Padding hasta 0x1000 (inicio de .text)
    file.write_all(&vec![0u8; 0x1000 - 0x400])?;

    // .text section data
    file.write_all(opcodes)?;
    // Padding
    let padding = code_size - opcodes.len() as u32;
    file.write_all(&vec![0u8; padding as usize])?;

    Ok(())
}
