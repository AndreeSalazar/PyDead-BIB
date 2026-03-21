// PE Generator usando estructura validada
// Basado en especificación PE de Microsoft

use std::fs::File;
use std::io::Write;

pub fn generate_pe_valid(
    opcodes: &[u8],
    _data: &[u8],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(output_path)?;

    // DOS Header (64 bytes)
    let mut dos = vec![0u8; 64];
    dos[0] = 0x4D; // 'M'
    dos[1] = 0x5A; // 'Z'
    dos[0x3C] = 0x40; // e_lfanew (offset to PE header)
    dos[0x3D] = 0x00;
    dos[0x3E] = 0x00;
    dos[0x3F] = 0x00;
    file.write_all(&dos)?;

    // PE Signature (4 bytes)
    file.write_all(b"PE\0\0")?;

    // COFF Header (20 bytes)
    let mut coff = vec![0u8; 20];
    // Machine: 0x8664 (IMAGE_FILE_MACHINE_AMD64)
    coff[0] = 0x64;
    coff[1] = 0x86;
    // NumberOfSections: 1
    coff[2] = 0x01;
    coff[3] = 0x00;
    // TimeDateStamp: 0
    // PointerToSymbolTable: 0
    // NumberOfSymbols: 0
    // SizeOfOptionalHeader: 240 (0xF0)
    coff[16] = 0xF0;
    coff[17] = 0x00;
    // Characteristics: 0x22 (IMAGE_FILE_EXECUTABLE_IMAGE | IMAGE_FILE_LARGE_ADDRESS_AWARE)
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
    // MinorLinkerVersion: 0
    opt[3] = 0;

    // SizeOfCode: alineado a 0x200
    let code_size = ((opcodes.len() + 0x1FF) & !0x1FF) as u32;
    opt[4..8].copy_from_slice(&code_size.to_le_bytes());

    // SizeOfInitializedData: 0
    // SizeOfUninitializedData: 0

    // AddressOfEntryPoint: 0x1000 (RVA)
    opt[16..20].copy_from_slice(&0x1000u32.to_le_bytes());

    // BaseOfCode: 0x1000 (RVA)
    opt[20..24].copy_from_slice(&0x1000u32.to_le_bytes());

    // ImageBase: 0x400000
    opt[24..32].copy_from_slice(&0x400000u64.to_le_bytes());

    // SectionAlignment: 0x1000
    opt[32..36].copy_from_slice(&0x1000u32.to_le_bytes());

    // FileAlignment: 0x200
    opt[36..40].copy_from_slice(&0x200u32.to_le_bytes());

    // MajorOperatingSystemVersion: 6
    opt[40] = 6;
    opt[41] = 0;

    // MinorOperatingSystemVersion: 0
    // MajorImageVersion: 0
    // MinorImageVersion: 0
    // MajorSubsystemVersion: 6
    opt[48] = 6;
    opt[49] = 0;

    // MinorSubsystemVersion: 0
    // Win32VersionValue: 0

    // SizeOfImage: 0x2000 (2 páginas)
    opt[56..60].copy_from_slice(&0x2000u32.to_le_bytes());

    // SizeOfHeaders: 0x400
    opt[60..64].copy_from_slice(&0x400u32.to_le_bytes());

    // CheckSum: 0 (opcional)

    // Subsystem: 3 (IMAGE_SUBSYSTEM_WINDOWS_CUI)
    opt[68] = 0x03;
    opt[69] = 0x00;

    // DllCharacteristics: 0
    // SizeOfStackReserve: 0x100000
    opt[72..80].copy_from_slice(&0x100000u64.to_le_bytes());
    // SizeOfStackCommit: 0x1000
    opt[80..88].copy_from_slice(&0x1000u64.to_le_bytes());
    // SizeOfHeapReserve: 0x100000
    opt[88..96].copy_from_slice(&0x100000u64.to_le_bytes());
    // SizeOfHeapCommit: 0x1000
    opt[96..104].copy_from_slice(&0x1000u64.to_le_bytes());

    // LoaderFlags: 0

    // NumberOfRvaAndSizes: 16
    opt[108..112].copy_from_slice(&16u32.to_le_bytes());

    // Data Directories (todos 0 por ahora, excepto que necesitamos Import Table)
    // Import Table RVA y Size (serán 0 por ahora)

    file.write_all(&opt)?;

    // Section Header .text (40 bytes)
    let mut sec = vec![0u8; 40];
    // Name: ".text\0\0\0"
    sec[0..8].copy_from_slice(b".text\0\0\0");
    // VirtualSize: code_size
    sec[8..12].copy_from_slice(&code_size.to_le_bytes());
    // VirtualAddress: 0x1000 (RVA)
    sec[12..16].copy_from_slice(&0x1000u32.to_le_bytes());
    // SizeOfRawData: code_size
    sec[16..20].copy_from_slice(&code_size.to_le_bytes());
    // PointerToRawData: 0x1000
    sec[20..24].copy_from_slice(&0x1000u32.to_le_bytes());
    // PointerToRelocations: 0
    // PointerToLinenumbers: 0
    // NumberOfRelocations: 0
    // NumberOfLinenumbers: 0
    // Characteristics: 0x60000020 (IMAGE_SCN_CNT_CODE | IMAGE_SCN_MEM_EXECUTE | IMAGE_SCN_MEM_READ)
    sec[36..40].copy_from_slice(&0x60000020u32.to_le_bytes());
    file.write_all(&sec)?;

    // Padding hasta 0x400 (SizeOfHeaders)
    let current_pos = 64 + 4 + 20 + 240 + 40;
    let padding = 0x400 - current_pos;
    file.write_all(&vec![0u8; padding])?;

    // Padding hasta 0x1000 (inicio de .text)
    file.write_all(&vec![0u8; 0x1000 - 0x400])?;

    // .text section data
    file.write_all(opcodes)?;
    let code_padding = code_size - opcodes.len() as u32;
    file.write_all(&vec![0u8; code_padding as usize])?;

    Ok(())
}
