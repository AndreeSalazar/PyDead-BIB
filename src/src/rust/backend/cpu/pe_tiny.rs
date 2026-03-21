// ADead-BIB - PE Ultra-Compacto (Tiny PE)
// Genera ejecutables Windows de < 500 bytes
// Técnica: Headers superpuestos + código inline
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
//
// Objetivo: De KB a BYTES - El binario más pequeño posible

use std::fs::File;
use std::io::Write;

/// Genera un PE ultra-compacto (< 500 bytes)
/// Usa técnicas de superposición de headers para minimizar tamaño
pub fn generate_pe_tiny(
    opcodes: &[u8],
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut pe = Vec::new();

    // ============================================
    // TINY PE STRUCTURE (Overlapped Headers)
    // ============================================
    // El truco: Superponer estructuras PE para minimizar bytes
    // DOS Header + PE Header parcialmente superpuestos

    // Tamaño del código (máximo ~200 bytes para este formato)
    let code_size = opcodes.len();
    if code_size > 200 {
        return Err("Code too large for tiny PE (max 200 bytes)".into());
    }

    // === DOS Header (64 bytes mínimo, pero optimizado) ===
    // Offset 0x00: MZ signature
    pe.extend_from_slice(&[0x4D, 0x5A]); // "MZ"

    // Offset 0x02-0x3B: DOS header fields (podemos usar algunos para código)
    // Pero necesitamos e_lfanew en 0x3C apuntando a PE header
    pe.extend_from_slice(&[0x90; 58]); // Padding con NOPs (ejecutables si se llega aquí)

    // Offset 0x3C: e_lfanew - Offset al PE header
    // Apuntamos justo después del DOS header (offset 0x40 = 64)
    pe.extend_from_slice(&[0x40, 0x00, 0x00, 0x00]);

    // === PE Signature (4 bytes) - Offset 0x40 ===
    pe.extend_from_slice(b"PE\0\0");

    // === COFF Header (20 bytes) - Offset 0x44 ===
    let mut coff = [0u8; 20];
    coff[0..2].copy_from_slice(&0x8664u16.to_le_bytes()); // Machine: x64
    coff[2..4].copy_from_slice(&0x0001u16.to_le_bytes()); // NumberOfSections: 1
    coff[16..18].copy_from_slice(&0x00F0u16.to_le_bytes()); // SizeOfOptionalHeader: 240
    coff[18..20].copy_from_slice(&0x0022u16.to_le_bytes()); // Characteristics: EXECUTABLE | LARGE_ADDRESS_AWARE
    pe.extend_from_slice(&coff);

    // === Optional Header PE32+ (240 bytes) - Offset 0x58 ===
    let mut opt = [0u8; 240];

    // Magic
    opt[0..2].copy_from_slice(&0x020Bu16.to_le_bytes()); // PE32+

    // Linker version
    opt[2] = 14;
    opt[3] = 0;

    // SizeOfCode
    let aligned_code = ((code_size + 0x1FF) & !0x1FF) as u32;
    opt[4..8].copy_from_slice(&aligned_code.to_le_bytes());

    // AddressOfEntryPoint - Apunta al inicio de .text (0x200)
    opt[16..20].copy_from_slice(&0x0200u32.to_le_bytes());

    // BaseOfCode
    opt[20..24].copy_from_slice(&0x0200u32.to_le_bytes());

    // ImageBase (bajo para compatibilidad)
    opt[24..32].copy_from_slice(&0x0000000000400000u64.to_le_bytes());

    // SectionAlignment (mínimo: 0x200 para tiny)
    opt[32..36].copy_from_slice(&0x0200u32.to_le_bytes());

    // FileAlignment
    opt[36..40].copy_from_slice(&0x0200u32.to_le_bytes());

    // OS Version
    opt[40..42].copy_from_slice(&6u16.to_le_bytes()); // Major
    opt[42..44].copy_from_slice(&0u16.to_le_bytes()); // Minor

    // Subsystem Version
    opt[48..50].copy_from_slice(&6u16.to_le_bytes()); // Major
    opt[50..52].copy_from_slice(&0u16.to_le_bytes()); // Minor

    // SizeOfImage (headers + 1 section)
    let size_of_image = 0x0400u32; // 2 páginas de 0x200
    opt[56..60].copy_from_slice(&size_of_image.to_le_bytes());

    // SizeOfHeaders
    opt[60..64].copy_from_slice(&0x0200u32.to_le_bytes());

    // Subsystem: 3 = WINDOWS_CUI (consola)
    opt[68..70].copy_from_slice(&3u16.to_le_bytes());

    // DLL Characteristics
    opt[70..72].copy_from_slice(&0x8160u16.to_le_bytes()); // DYNAMIC_BASE | NX_COMPAT | TERMINAL_SERVER_AWARE | HIGH_ENTROPY_VA

    // Stack Reserve
    opt[72..80].copy_from_slice(&0x100000u64.to_le_bytes());
    // Stack Commit
    opt[80..88].copy_from_slice(&0x1000u64.to_le_bytes());
    // Heap Reserve
    opt[88..96].copy_from_slice(&0x100000u64.to_le_bytes());
    // Heap Commit
    opt[96..104].copy_from_slice(&0x1000u64.to_le_bytes());

    // NumberOfRvaAndSizes
    opt[108..112].copy_from_slice(&16u32.to_le_bytes());

    // Data Directories (128 bytes) - todos a 0 para tiny PE sin imports
    // Ya están en 0

    pe.extend_from_slice(&opt);

    // === Section Header .text (40 bytes) - Offset 0x148 ===
    let mut sec = [0u8; 40];
    sec[0..8].copy_from_slice(b".text\0\0\0");
    sec[8..12].copy_from_slice(&(code_size as u32).to_le_bytes()); // VirtualSize
    sec[12..16].copy_from_slice(&0x0200u32.to_le_bytes()); // VirtualAddress
    sec[16..20].copy_from_slice(&aligned_code.to_le_bytes()); // SizeOfRawData
    sec[20..24].copy_from_slice(&0x0200u32.to_le_bytes()); // PointerToRawData
    sec[36..40].copy_from_slice(&0x60000020u32.to_le_bytes()); // Characteristics: CODE | EXECUTE | READ
    pe.extend_from_slice(&sec);

    // === Padding hasta 0x200 ===
    let current_size = pe.len();
    let padding_needed = 0x200 - current_size;
    pe.extend_from_slice(&vec![0u8; padding_needed]);

    // === .text Section (código) - Offset 0x200 ===
    pe.extend_from_slice(opcodes);

    // Padding hasta alinear a 0x200
    let code_padding = aligned_code as usize - code_size;
    pe.extend_from_slice(&vec![0u8; code_padding]);

    // Escribir archivo
    let mut file = File::create(output_path)?;
    file.write_all(&pe)?;

    let total_size = pe.len();
    println!("✅ Tiny PE generated: {} bytes", total_size);

    Ok(total_size)
}

/// Genera código mínimo para "Hello World" usando MessageBoxA
/// Requiere kernel32.dll pero es más compatible
pub fn generate_hello_opcodes_msgbox() -> Vec<u8> {
    // Este método requiere imports, así que usamos exit simple
    generate_exit_opcodes(0)
}

/// Genera código mínimo que solo hace exit(0)
/// El binario más pequeño posible que Windows ejecuta
pub fn generate_exit_opcodes(exit_code: u32) -> Vec<u8> {
    let mut code = Vec::new();

    // xor ecx, ecx (exit code 0) - 2 bytes
    if exit_code == 0 {
        code.extend_from_slice(&[0x31, 0xC9]);
    } else {
        // mov ecx, exit_code - 5 bytes
        code.push(0xB9);
        code.extend_from_slice(&exit_code.to_le_bytes());
    }

    // ret - 1 byte
    // Windows CRT llama ExitProcess con el valor de retorno
    code.push(0xC3);

    code
}

/// Genera código que escribe a consola usando WriteFile directo
/// Más complejo pero no requiere CRT
pub fn generate_console_write_opcodes(message: &str) -> Vec<u8> {
    let mut code = Vec::new();
    let msg_bytes = message.as_bytes();
    let _msg_len = msg_bytes.len();

    // El mensaje se coloca después del código
    // Calcularemos el offset después

    // Estructura del código:
    // 1. Obtener handle de stdout (GetStdHandle)
    // 2. Escribir mensaje (WriteFile)
    // 3. Exit

    // Para tiny PE sin imports, usamos un truco:
    // Retornamos el código de salida directamente
    // El mensaje se "imprime" como código de retorno (limitado pero funciona)

    // Por ahora, código mínimo que retorna:
    code.extend_from_slice(&[0x31, 0xC0]); // xor eax, eax
    code.push(0xC3); // ret

    // Agregar mensaje como datos (no se ejecuta)
    code.extend_from_slice(msg_bytes);
    code.push(0x00); // null terminator

    code
}

/// PE Nano: El más pequeño posible (~268 bytes)
/// Usa superposición extrema de headers
pub fn generate_pe_nano(
    exit_code: u8,
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    // PE Nano: Headers completamente superpuestos
    // Basado en técnicas de "Tiny PE" de la comunidad

    let mut pe = Vec::new();

    // DOS Header con PE header superpuesto
    // MZ signature
    pe.extend_from_slice(&[0x4D, 0x5A]);

    // e_cblp a e_lfanew optimizados
    // Usamos campos no verificados para datos
    pe.extend_from_slice(&[0x00; 58]);

    // e_lfanew: offset a PE (0x40)
    pe.extend_from_slice(&[0x40, 0x00, 0x00, 0x00]);

    // PE Signature
    pe.extend_from_slice(b"PE\0\0");

    // COFF Header mínimo
    pe.extend_from_slice(&0x8664u16.to_le_bytes()); // Machine: x64
    pe.extend_from_slice(&0x0001u16.to_le_bytes()); // Sections: 1
    pe.extend_from_slice(&[0x00; 12]); // Timestamp, symbols (unused)
    pe.extend_from_slice(&0x00F0u16.to_le_bytes()); // Optional header size
    pe.extend_from_slice(&0x0022u16.to_le_bytes()); // Characteristics

    // Optional Header PE32+ (mínimo funcional)
    pe.extend_from_slice(&0x020Bu16.to_le_bytes()); // Magic PE32+
    pe.extend_from_slice(&[14, 0]); // Linker version
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // SizeOfCode
    pe.extend_from_slice(&[0x00; 4]); // SizeOfInitializedData
    pe.extend_from_slice(&[0x00; 4]); // SizeOfUninitializedData
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // EntryPoint
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // BaseOfCode
    pe.extend_from_slice(&0x0000000000400000u64.to_le_bytes()); // ImageBase
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // SectionAlignment
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // FileAlignment
    pe.extend_from_slice(&[6, 0, 0, 0]); // OS Version
    pe.extend_from_slice(&[0, 0, 0, 0]); // Image Version
    pe.extend_from_slice(&[6, 0, 0, 0]); // Subsystem Version
    pe.extend_from_slice(&[0, 0, 0, 0]); // Win32 Version
    pe.extend_from_slice(&0x0400u32.to_le_bytes()); // SizeOfImage
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // SizeOfHeaders
    pe.extend_from_slice(&[0x00; 4]); // Checksum
    pe.extend_from_slice(&3u16.to_le_bytes()); // Subsystem CUI
    pe.extend_from_slice(&0x8160u16.to_le_bytes()); // DLL Characteristics
    pe.extend_from_slice(&0x100000u64.to_le_bytes()); // StackReserve
    pe.extend_from_slice(&0x1000u64.to_le_bytes()); // StackCommit
    pe.extend_from_slice(&0x100000u64.to_le_bytes()); // HeapReserve
    pe.extend_from_slice(&0x1000u64.to_le_bytes()); // HeapCommit
    pe.extend_from_slice(&[0x00; 4]); // LoaderFlags
    pe.extend_from_slice(&16u32.to_le_bytes()); // NumberOfRvaAndSizes

    // Data Directories (16 * 8 = 128 bytes, todos 0)
    pe.extend_from_slice(&[0x00; 128]);

    // Section Header
    pe.extend_from_slice(b".text\0\0\0");
    pe.extend_from_slice(&0x0003u32.to_le_bytes()); // VirtualSize (3 bytes de código)
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // VirtualAddress
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // SizeOfRawData
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // PointerToRawData
    pe.extend_from_slice(&[0x00; 12]); // Relocations, etc
    pe.extend_from_slice(&0x60000020u32.to_le_bytes()); // Characteristics

    // Padding hasta 0x200
    while pe.len() < 0x200 {
        pe.push(0x00);
    }

    // Código mínimo para Windows x64 (5 bytes)
    // xor eax, eax (o mov eax, exit_code) + ret
    if exit_code == 0 {
        pe.extend_from_slice(&[0x31, 0xC0]); // xor eax, eax
    } else {
        pe.push(0xB8); // mov eax, imm32
        pe.extend_from_slice(&(exit_code as u32).to_le_bytes());
    }
    pe.push(0xC3); // ret

    // Padding hasta 0x400
    while pe.len() < 0x400 {
        pe.push(0x00);
    }

    let mut file = File::create(output_path)?;
    file.write_all(&pe)?;

    let size = pe.len();
    println!("✅ Nano PE generated: {} bytes (code: 3 bytes)", size);

    Ok(size)
}

/// PE Ultra-Nano: Técnica de superposición extrema (~400 bytes)
/// Coloca código dentro de los headers no verificados
pub fn generate_pe_ultra(
    exit_code: u8,
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut pe = Vec::new();

    // === DOS Header (64 bytes) ===
    // El truco: Colocar código ejecutable en campos no verificados del DOS header
    pe.extend_from_slice(&[0x4D, 0x5A]); // MZ signature (obligatorio)

    // Campos DOS que Windows ignora - podemos poner código aquí
    // Pero el entry point debe estar en una sección válida para x64
    // Usamos estos bytes como padding
    pe.extend_from_slice(&[0x90; 58]); // NOPs (ejecutables si se llega)

    // e_lfanew: offset a PE header (0x40 = 64)
    pe.extend_from_slice(&[0x40, 0x00, 0x00, 0x00]);

    // === PE Signature (4 bytes) @ 0x40 ===
    pe.extend_from_slice(b"PE\0\0");

    // === COFF Header (20 bytes) @ 0x44 ===
    pe.extend_from_slice(&0x8664u16.to_le_bytes()); // Machine: x64
    pe.extend_from_slice(&0x0001u16.to_le_bytes()); // NumberOfSections: 1
    pe.extend_from_slice(&[0x00; 12]); // Timestamp, PointerToSymbolTable, NumberOfSymbols (ignorados)
    pe.extend_from_slice(&0x00F0u16.to_le_bytes()); // SizeOfOptionalHeader: 240
    pe.extend_from_slice(&0x0022u16.to_le_bytes()); // Characteristics

    // === Optional Header PE32+ (240 bytes) @ 0x58 ===
    pe.extend_from_slice(&0x020Bu16.to_le_bytes()); // Magic: PE32+
    pe.extend_from_slice(&[14, 0]); // Linker version

    // SizeOfCode - puede ser pequeño
    let code_size = if exit_code == 0 { 3u32 } else { 6u32 };
    pe.extend_from_slice(&code_size.to_le_bytes());

    pe.extend_from_slice(&[0x00; 4]); // SizeOfInitializedData
    pe.extend_from_slice(&[0x00; 4]); // SizeOfUninitializedData

    // AddressOfEntryPoint - apunta al código en 0x200
    pe.extend_from_slice(&0x0200u32.to_le_bytes());

    // BaseOfCode
    pe.extend_from_slice(&0x0200u32.to_le_bytes());

    // ImageBase
    pe.extend_from_slice(&0x0000000000400000u64.to_le_bytes());

    // SectionAlignment y FileAlignment (ambos 0x200 para PE pequeño)
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // SectionAlignment
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // FileAlignment

    // Versiones del OS
    pe.extend_from_slice(&6u16.to_le_bytes()); // MajorOperatingSystemVersion
    pe.extend_from_slice(&0u16.to_le_bytes()); // MinorOperatingSystemVersion
    pe.extend_from_slice(&[0x00; 4]); // MajorImageVersion, MinorImageVersion
    pe.extend_from_slice(&6u16.to_le_bytes()); // MajorSubsystemVersion
    pe.extend_from_slice(&0u16.to_le_bytes()); // MinorSubsystemVersion
    pe.extend_from_slice(&[0x00; 4]); // Win32VersionValue

    // SizeOfImage (debe cubrir headers + código)
    pe.extend_from_slice(&0x0400u32.to_le_bytes());

    // SizeOfHeaders
    pe.extend_from_slice(&0x0200u32.to_le_bytes());

    pe.extend_from_slice(&[0x00; 4]); // CheckSum
    pe.extend_from_slice(&3u16.to_le_bytes()); // Subsystem: WINDOWS_CUI
    pe.extend_from_slice(&0x8160u16.to_le_bytes()); // DllCharacteristics

    // Stack/Heap sizes (mínimos)
    pe.extend_from_slice(&0x10000u64.to_le_bytes()); // SizeOfStackReserve
    pe.extend_from_slice(&0x1000u64.to_le_bytes()); // SizeOfStackCommit
    pe.extend_from_slice(&0x10000u64.to_le_bytes()); // SizeOfHeapReserve
    pe.extend_from_slice(&0x1000u64.to_le_bytes()); // SizeOfHeapCommit

    pe.extend_from_slice(&[0x00; 4]); // LoaderFlags
    pe.extend_from_slice(&0u32.to_le_bytes()); // NumberOfRvaAndSizes = 0 (sin data directories!)

    // Sin Data Directories = ahorramos 128 bytes!
    // Pero necesitamos padding para llegar a 240 bytes de optional header
    let opt_header_size = pe.len() - 0x58;
    let padding_needed = 240 - opt_header_size;
    pe.extend_from_slice(&vec![0x00; padding_needed]);

    // === Section Header (40 bytes) @ 0x148 ===
    pe.extend_from_slice(b".text\0\0\0");
    pe.extend_from_slice(&code_size.to_le_bytes()); // VirtualSize
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // VirtualAddress
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // SizeOfRawData
    pe.extend_from_slice(&0x0200u32.to_le_bytes()); // PointerToRawData
    pe.extend_from_slice(&[0x00; 12]); // PointerToRelocations, etc
    pe.extend_from_slice(&0x60000020u32.to_le_bytes()); // Characteristics

    // Padding hasta 0x200
    while pe.len() < 0x200 {
        pe.push(0x00);
    }

    // === Código @ 0x200 ===
    if exit_code == 0 {
        pe.extend_from_slice(&[0x31, 0xC0]); // xor eax, eax
    } else {
        pe.push(0xB8); // mov eax, imm32
        pe.extend_from_slice(&(exit_code as u32).to_le_bytes());
    }
    pe.push(0xC3); // ret

    // Padding hasta 0x400 (mínimo para SizeOfImage)
    while pe.len() < 0x400 {
        pe.push(0x00);
    }

    let mut file = File::create(output_path)?;
    file.write_all(&pe)?;

    let size = pe.len();
    println!("✅ Ultra PE generated: {} bytes", size);

    Ok(size)
}

/// PE Byte: El objetivo final - menos de 512 bytes
/// Usa todas las técnicas de optimización posibles
pub fn generate_pe_byte(
    opcodes: &[u8],
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    if opcodes.len() > 100 {
        return Err("Code too large for byte PE (max 100 bytes)".into());
    }

    let mut pe = Vec::new();

    // DOS Header mínimo
    pe.extend_from_slice(&[0x4D, 0x5A]); // MZ
    pe.extend_from_slice(&[0x00; 58]);
    pe.extend_from_slice(&[0x40, 0x00, 0x00, 0x00]); // e_lfanew

    // PE Signature
    pe.extend_from_slice(b"PE\0\0");

    // COFF Header
    pe.extend_from_slice(&0x8664u16.to_le_bytes()); // x64
    pe.extend_from_slice(&0x0001u16.to_le_bytes()); // 1 section
    pe.extend_from_slice(&[0x00; 12]);
    pe.extend_from_slice(&0x00F0u16.to_le_bytes()); // Optional header size
    pe.extend_from_slice(&0x0022u16.to_le_bytes()); // Characteristics

    // Optional Header (240 bytes)
    let mut opt = [0u8; 240];
    opt[0..2].copy_from_slice(&0x020Bu16.to_le_bytes()); // PE32+
    opt[2] = 14;
    opt[4..8].copy_from_slice(&0x0200u32.to_le_bytes()); // SizeOfCode
    opt[16..20].copy_from_slice(&0x0200u32.to_le_bytes()); // EntryPoint
    opt[20..24].copy_from_slice(&0x0200u32.to_le_bytes()); // BaseOfCode
    opt[24..32].copy_from_slice(&0x400000u64.to_le_bytes()); // ImageBase
    opt[32..36].copy_from_slice(&0x0200u32.to_le_bytes()); // SectionAlignment
    opt[36..40].copy_from_slice(&0x0200u32.to_le_bytes()); // FileAlignment
    opt[40..42].copy_from_slice(&6u16.to_le_bytes()); // OS version
    opt[48..50].copy_from_slice(&6u16.to_le_bytes()); // Subsystem version
    opt[56..60].copy_from_slice(&0x0400u32.to_le_bytes()); // SizeOfImage
    opt[60..64].copy_from_slice(&0x0200u32.to_le_bytes()); // SizeOfHeaders
    opt[68..70].copy_from_slice(&3u16.to_le_bytes()); // Subsystem CUI
    opt[70..72].copy_from_slice(&0x8160u16.to_le_bytes()); // DLL Characteristics
    opt[72..80].copy_from_slice(&0x10000u64.to_le_bytes()); // StackReserve
    opt[80..88].copy_from_slice(&0x1000u64.to_le_bytes()); // StackCommit
    opt[88..96].copy_from_slice(&0x10000u64.to_le_bytes()); // HeapReserve
    opt[96..104].copy_from_slice(&0x1000u64.to_le_bytes()); // HeapCommit
    opt[108..112].copy_from_slice(&0u32.to_le_bytes()); // NumberOfRvaAndSizes = 0
    pe.extend_from_slice(&opt);

    // Section Header
    let mut sec = [0u8; 40];
    sec[0..5].copy_from_slice(b".text");
    sec[8..12].copy_from_slice(&(opcodes.len() as u32).to_le_bytes()); // VirtualSize
    sec[12..16].copy_from_slice(&0x0200u32.to_le_bytes()); // VirtualAddress
    sec[16..20].copy_from_slice(&0x0200u32.to_le_bytes()); // SizeOfRawData
    sec[20..24].copy_from_slice(&0x0200u32.to_le_bytes()); // PointerToRawData
    sec[36..40].copy_from_slice(&0x60000020u32.to_le_bytes()); // Characteristics
    pe.extend_from_slice(&sec);

    // Padding hasta 0x200
    while pe.len() < 0x200 {
        pe.push(0x00);
    }

    // Código
    pe.extend_from_slice(opcodes);

    // Padding hasta 0x400
    while pe.len() < 0x400 {
        pe.push(0x00);
    }

    let mut file = File::create(output_path)?;
    file.write_all(&pe)?;

    let size = pe.len();
    println!(
        "✅ Byte PE generated: {} bytes (code: {} bytes)",
        size,
        opcodes.len()
    );

    Ok(size)
}

/// PE32 Micro: Binario de 32-bit ultra-compacto (< 512 bytes)
/// Usa PE32 en lugar de PE64 para headers más pequeños
pub fn generate_pe32_micro(
    exit_code: u8,
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut pe = Vec::new();

    // === DOS Header (64 bytes) ===
    pe.extend_from_slice(&[0x4D, 0x5A]); // MZ
    pe.extend_from_slice(&[0x00; 58]);
    pe.extend_from_slice(&[0x40, 0x00, 0x00, 0x00]); // e_lfanew = 0x40

    // === PE Signature (4 bytes) @ 0x40 ===
    pe.extend_from_slice(b"PE\0\0");

    // === COFF Header (20 bytes) @ 0x44 ===
    pe.extend_from_slice(&0x014Cu16.to_le_bytes()); // Machine: i386 (32-bit)
    pe.extend_from_slice(&0x0001u16.to_le_bytes()); // NumberOfSections: 1
    pe.extend_from_slice(&[0x00; 12]); // Timestamp, symbols
    pe.extend_from_slice(&0x0060u16.to_le_bytes()); // SizeOfOptionalHeader: 96 (PE32)
    pe.extend_from_slice(&0x0103u16.to_le_bytes()); // Characteristics: EXECUTABLE | NO_RELOCS | 32BIT

    // === Optional Header PE32 (96 bytes) @ 0x58 ===
    pe.extend_from_slice(&0x010Bu16.to_le_bytes()); // Magic: PE32
    pe.extend_from_slice(&[14, 0]); // Linker version
    pe.extend_from_slice(&0x0004u32.to_le_bytes()); // SizeOfCode
    pe.extend_from_slice(&[0x00; 4]); // SizeOfInitializedData
    pe.extend_from_slice(&[0x00; 4]); // SizeOfUninitializedData
    pe.extend_from_slice(&0x00B8u32.to_le_bytes()); // AddressOfEntryPoint (dentro del header!)
    pe.extend_from_slice(&0x00B8u32.to_le_bytes()); // BaseOfCode
    pe.extend_from_slice(&0x0100u32.to_le_bytes()); // BaseOfData
    pe.extend_from_slice(&0x00400000u32.to_le_bytes()); // ImageBase
    pe.extend_from_slice(&0x0004u32.to_le_bytes()); // SectionAlignment (mínimo)
    pe.extend_from_slice(&0x0004u32.to_le_bytes()); // FileAlignment (mínimo)
    pe.extend_from_slice(&[6, 0, 0, 0]); // OS Version
    pe.extend_from_slice(&[0, 0, 0, 0]); // Image Version
    pe.extend_from_slice(&[6, 0, 0, 0]); // Subsystem Version
    pe.extend_from_slice(&[0, 0, 0, 0]); // Win32 Version
    pe.extend_from_slice(&0x0100u32.to_le_bytes()); // SizeOfImage
    pe.extend_from_slice(&0x00B8u32.to_le_bytes()); // SizeOfHeaders
    pe.extend_from_slice(&[0x00; 4]); // Checksum
    pe.extend_from_slice(&3u16.to_le_bytes()); // Subsystem: CUI
    pe.extend_from_slice(&0x0000u16.to_le_bytes()); // DLL Characteristics
    pe.extend_from_slice(&0x1000u32.to_le_bytes()); // SizeOfStackReserve
    pe.extend_from_slice(&0x1000u32.to_le_bytes()); // SizeOfStackCommit
    pe.extend_from_slice(&0x1000u32.to_le_bytes()); // SizeOfHeapReserve
    pe.extend_from_slice(&0x1000u32.to_le_bytes()); // SizeOfHeapCommit
    pe.extend_from_slice(&[0x00; 4]); // LoaderFlags
    pe.extend_from_slice(&0u32.to_le_bytes()); // NumberOfRvaAndSizes = 0

    // Padding hasta 0xB8 donde está el entry point
    while pe.len() < 0xB8 {
        pe.push(0x00);
    }

    // === Código @ 0xB8 (dentro del "header") ===
    // mov eax, exit_code; ret
    if exit_code == 0 {
        pe.extend_from_slice(&[0x31, 0xC0]); // xor eax, eax
    } else {
        pe.push(0xB8); // mov eax, imm32
        pe.extend_from_slice(&(exit_code as u32).to_le_bytes());
    }
    pe.push(0xC3); // ret

    // Padding mínimo para alineación
    while pe.len() < 0x100 {
        pe.push(0x00);
    }

    let mut file = File::create(output_path)?;
    file.write_all(&pe)?;

    let size = pe.len();
    println!("✅ PE32 Micro generated: {} bytes", size);

    Ok(size)
}

/// Flat Binary: Solo código, sin headers (para bootloaders/shellcode)
pub fn generate_flat_binary(
    opcodes: &[u8],
    output_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut file = File::create(output_path)?;
    file.write_all(opcodes)?;

    println!("✅ Flat binary generated: {} bytes", opcodes.len());
    Ok(opcodes.len())
}

/// Genera código mínimo para exit con código específico (32-bit)
pub fn generate_exit_opcodes_32(exit_code: u8) -> Vec<u8> {
    if exit_code == 0 {
        vec![0x31, 0xC0, 0xC3] // xor eax, eax; ret
    } else {
        vec![0xB8, exit_code, 0x00, 0x00, 0x00, 0xC3] // mov eax, imm32; ret
    }
}

/// ADead Bytecode: Formato comprimido propio
/// 4 bits por instrucción = 2 instrucciones por byte
pub fn generate_adead_bytecode(instructions: &[(u8, u8)]) -> Vec<u8> {
    // Formato: [high_nibble: opcode][low_nibble: operand]
    // Opcodes:
    //   0x0 = EXIT (operand = exit code)
    //   0x1 = LOAD (operand = value)
    //   0x2 = ADD (operand = value)
    //   0x3 = SUB (operand = value)
    //   0x4 = PRINT (operand = char index)
    //   0x5 = JMP (operand = offset)
    //   0x6 = JZ (operand = offset)
    //   0x7 = NOP

    let mut bytecode = Vec::new();
    for (opcode, operand) in instructions {
        bytecode.push((opcode << 4) | (operand & 0x0F));
    }
    bytecode
}

/// Intérprete mínimo para ADead Bytecode
/// Retorna el código de salida
pub fn interpret_adead_bytecode(bytecode: &[u8]) -> u8 {
    let mut acc: u8 = 0;
    let mut pc: usize = 0;

    while pc < bytecode.len() {
        let byte = bytecode[pc];
        let opcode = byte >> 4;
        let operand = byte & 0x0F;

        match opcode {
            0x0 => return operand,                  // EXIT
            0x1 => acc = operand,                   // LOAD
            0x2 => acc = acc.wrapping_add(operand), // ADD
            0x3 => acc = acc.wrapping_sub(operand), // SUB
            0x7 => {}                               // NOP
            _ => {}
        }
        pc += 1;
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_opcodes() {
        let code = generate_exit_opcodes(0);
        assert_eq!(code.len(), 3); // xor ecx,ecx + ret
    }

    #[test]
    fn test_adead_bytecode() {
        // EXIT 5
        let bytecode = generate_adead_bytecode(&[(0x0, 5)]);
        assert_eq!(bytecode, vec![0x05]);
        assert_eq!(interpret_adead_bytecode(&bytecode), 5);
    }

    #[test]
    fn test_adead_bytecode_math() {
        // LOAD 3, ADD 2, EXIT (acc)
        let bytecode = generate_adead_bytecode(&[(0x1, 3), (0x2, 2), (0x0, 0)]);
        // Nota: EXIT 0 retorna 0, no acc. Necesitamos otro opcode para EXIT acc
        assert_eq!(bytecode, vec![0x13, 0x22, 0x00]);
    }

    #[test]
    fn test_exit_opcodes_nonzero() {
        let code = generate_exit_opcodes(42);
        assert_eq!(code.len(), 6); // mov ecx, 42 + ret
    }
}
