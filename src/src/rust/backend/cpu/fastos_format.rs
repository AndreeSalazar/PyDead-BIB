// ============================================================
// ADead-BIB — FastOS Binary Format
// ============================================================
// Formato ejecutable propio de FastOS — alternativa a PE (Windows)
// y ELF (Linux). Inspirado en la simplicidad de ELF pero diseñado
// para ADead-BIB como lenguaje base.
//
// Características:
//   - Header compacto (64 bytes) vs ELF (64) vs PE (~400+)
//   - Soporte multi-modo: 16-bit, 32-bit, 64-bit nativo
//   - Secciones simples: .text, .data, .bss, .rodata
//   - Entry point directo — sin dynamic linking overhead
//   - Checksum CRC32 para integridad
//   - Diseñado para boot directo sin loader complejo
//
// Magic: "FsOS" (0x46 0x73 0x4F 0x53)
//
// Pipeline: AST → ADeadIR → Encoder → FastOSGenerator → .fos
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use std::fs;
use std::io;
use std::path::Path;

// ============================================================
// FastOS Magic & Constants
// ============================================================

/// Magic number: "FsOS" — identifica un binario FastOS
const FASTOS_MAGIC: [u8; 4] = [0x46, 0x73, 0x4F, 0x53]; // "FsOS"

/// Versión del formato
const FASTOS_VERSION: u16 = 1;

/// Tamaño del header FastOS (64 bytes — mismo que ELF, más eficiente que PE)
const FASTOS_HEADER_SIZE: usize = 64;

/// Tamaño de cada entrada de sección (32 bytes)
const FASTOS_SECTION_ENTRY_SIZE: usize = 32;

/// Base address por defecto para kernel FastOS
const FASTOS_KERNEL_BASE: u64 = 0x100000; // 1MB — después de memoria convencional

// ============================================================
// CPU Mode — Escalado natural 16→32→64 bits
// ============================================================

/// Modo de CPU del binario FastOS.
/// ADead-BIB escala naturalmente desde 16-bit hasta 64-bit.
/// Default: 64-bit (Long64).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FastOSCpuMode {
    /// 16-bit real mode — boot sectors, BIOS
    Real16 = 0x10,
    /// 32-bit protected mode — legacy, drivers
    Protected32 = 0x20,
    /// 64-bit long mode — kernel, aplicaciones (DEFAULT)
    Long64 = 0x40,
}

impl FastOSCpuMode {
    /// Tamaño de operando por defecto para este modo
    pub fn operand_size(&self) -> u8 {
        match self {
            FastOSCpuMode::Real16 => 16,
            FastOSCpuMode::Protected32 => 32,
            FastOSCpuMode::Long64 => 64,
        }
    }

    /// Tamaño de dirección por defecto para este modo
    pub fn address_size(&self) -> u8 {
        match self {
            FastOSCpuMode::Real16 => 20,      // 1MB address space
            FastOSCpuMode::Protected32 => 32, // 4GB
            FastOSCpuMode::Long64 => 64,      // Virtual 64-bit
        }
    }

    /// Prefijo REX necesario para este modo
    pub fn needs_rex(&self) -> bool {
        matches!(self, FastOSCpuMode::Long64)
    }

    /// Desde byte
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x10 => Some(FastOSCpuMode::Real16),
            0x20 => Some(FastOSCpuMode::Protected32),
            0x40 => Some(FastOSCpuMode::Long64),
            _ => None,
        }
    }
}

impl Default for FastOSCpuMode {
    fn default() -> Self {
        FastOSCpuMode::Long64 // ADead-BIB default: 64-bit
    }
}

// ============================================================
// FastOS Binary Type
// ============================================================

/// Tipo de binario FastOS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FastOSBinaryType {
    /// Ejecutable standalone
    Executable = 0x01,
    /// Kernel image (cargado por bootloader)
    Kernel = 0x02,
    /// Módulo/driver cargable
    Module = 0x03,
    /// Boot sector (512 bytes)
    BootSector = 0x04,
    /// Librería compartida
    SharedLib = 0x05,
}

// ============================================================
// FastOS Section
// ============================================================

/// Tipo de sección FastOS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FastOSSectionType {
    /// Código ejecutable
    Text = 0x01,
    /// Datos inicializados (read-write)
    Data = 0x02,
    /// Datos no inicializados (BSS)
    Bss = 0x03,
    /// Datos de solo lectura
    RoData = 0x04,
    /// Tabla de símbolos
    SymTab = 0x05,
    /// Tabla de strings
    StrTab = 0x06,
}

/// Flags de sección
#[derive(Debug, Clone, Copy)]
pub struct FastOSSectionFlags(pub u8);

impl FastOSSectionFlags {
    pub const READ: u8 = 0x01;
    pub const WRITE: u8 = 0x02;
    pub const EXEC: u8 = 0x04;
    pub const ALLOC: u8 = 0x08; // Ocupa memoria en runtime

    pub fn text() -> Self {
        Self(Self::READ | Self::EXEC | Self::ALLOC)
    }
    pub fn data() -> Self {
        Self(Self::READ | Self::WRITE | Self::ALLOC)
    }
    pub fn rodata() -> Self {
        Self(Self::READ | Self::ALLOC)
    }
    pub fn bss() -> Self {
        Self(Self::READ | Self::WRITE | Self::ALLOC)
    }
}

/// Entrada de sección en el header FastOS (32 bytes)
#[derive(Debug, Clone)]
pub struct FastOSSection {
    /// Tipo de sección
    pub section_type: FastOSSectionType,
    /// Flags (R/W/X/Alloc)
    pub flags: FastOSSectionFlags,
    /// Offset en el archivo
    pub file_offset: u32,
    /// Tamaño en el archivo
    pub file_size: u32,
    /// Dirección virtual en memoria
    pub vaddr: u64,
    /// Tamaño en memoria (puede ser > file_size para BSS)
    pub mem_size: u64,
    /// Alineación
    pub alignment: u32,
}

// ============================================================
// FastOS Header (64 bytes)
// ============================================================
//
// Layout:
//   [0..4]   Magic: "FsOS"
//   [4..6]   Version: u16
//   [6]      CPU Mode: u8 (0x10=16, 0x20=32, 0x40=64)
//   [7]      Binary Type: u8
//   [8..16]  Entry Point: u64
//   [16..24] Base Address: u64
//   [24..28] Section Table Offset: u32
//   [28..30] Section Count: u16
//   [30..34] Total File Size: u32
//   [34..38] Code Size: u32
//   [38..42] Data Size: u32
//   [42..46] BSS Size: u32
//   [46..50] Checksum (CRC32): u32
//   [50..58] Flags: u64
//   [58..64] Reserved: 6 bytes
//
// ============================================================

#[derive(Debug, Clone)]
pub struct FastOSHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub cpu_mode: FastOSCpuMode,
    pub binary_type: FastOSBinaryType,
    pub entry_point: u64,
    pub base_address: u64,
    pub section_table_offset: u32,
    pub section_count: u16,
    pub total_file_size: u32,
    pub code_size: u32,
    pub data_size: u32,
    pub bss_size: u32,
    pub checksum: u32,
    pub flags: u64,
}

impl FastOSHeader {
    pub fn new(cpu_mode: FastOSCpuMode, binary_type: FastOSBinaryType) -> Self {
        Self {
            magic: FASTOS_MAGIC,
            version: FASTOS_VERSION,
            cpu_mode,
            binary_type,
            entry_point: 0,
            base_address: FASTOS_KERNEL_BASE,
            section_table_offset: FASTOS_HEADER_SIZE as u32,
            section_count: 0,
            total_file_size: 0,
            code_size: 0,
            data_size: 0,
            bss_size: 0,
            checksum: 0,
            flags: 0,
        }
    }

    /// Serializar header a 64 bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(FASTOS_HEADER_SIZE);

        buf.extend_from_slice(&self.magic); // [0..4]
        buf.extend_from_slice(&self.version.to_le_bytes()); // [4..6]
        buf.push(self.cpu_mode as u8); // [6]
        buf.push(self.binary_type as u8); // [7]
        buf.extend_from_slice(&self.entry_point.to_le_bytes()); // [8..16]
        buf.extend_from_slice(&self.base_address.to_le_bytes()); // [16..24]
        buf.extend_from_slice(&self.section_table_offset.to_le_bytes()); // [24..28]
        buf.extend_from_slice(&self.section_count.to_le_bytes()); // [28..30]
        buf.extend_from_slice(&self.total_file_size.to_le_bytes()); // [30..34]
        buf.extend_from_slice(&self.code_size.to_le_bytes()); // [34..38]
        buf.extend_from_slice(&self.data_size.to_le_bytes()); // [38..42]
        buf.extend_from_slice(&self.bss_size.to_le_bytes()); // [42..46]
        buf.extend_from_slice(&self.checksum.to_le_bytes()); // [46..50]
        buf.extend_from_slice(&self.flags.to_le_bytes()); // [50..58]
        buf.extend_from_slice(&[0u8; 6]); // [58..64] reserved

        assert_eq!(buf.len(), FASTOS_HEADER_SIZE);
        buf
    }

    /// Deserializar header desde bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < FASTOS_HEADER_SIZE {
            return None;
        }
        if &data[0..4] != &FASTOS_MAGIC {
            return None;
        }

        Some(Self {
            magic: FASTOS_MAGIC,
            version: u16::from_le_bytes([data[4], data[5]]),
            cpu_mode: FastOSCpuMode::from_byte(data[6])?,
            binary_type: match data[7] {
                0x01 => FastOSBinaryType::Executable,
                0x02 => FastOSBinaryType::Kernel,
                0x03 => FastOSBinaryType::Module,
                0x04 => FastOSBinaryType::BootSector,
                0x05 => FastOSBinaryType::SharedLib,
                _ => return None,
            },
            entry_point: u64::from_le_bytes(data[8..16].try_into().ok()?),
            base_address: u64::from_le_bytes(data[16..24].try_into().ok()?),
            section_table_offset: u32::from_le_bytes(data[24..28].try_into().ok()?),
            section_count: u16::from_le_bytes(data[28..30].try_into().ok()?),
            total_file_size: u32::from_le_bytes(data[30..34].try_into().ok()?),
            code_size: u32::from_le_bytes(data[34..38].try_into().ok()?),
            data_size: u32::from_le_bytes(data[38..42].try_into().ok()?),
            bss_size: u32::from_le_bytes(data[42..46].try_into().ok()?),
            checksum: u32::from_le_bytes(data[46..50].try_into().ok()?),
            flags: u64::from_le_bytes(data[50..58].try_into().ok()?),
        })
    }
}

// ============================================================
// FastOS Generator
// ============================================================

/// Generador de binarios FastOS.
///
/// Produce archivos .fos con el formato propio de FastOS.
/// Inspirado en ELF pero más simple y directo.
pub struct FastOSGenerator {
    /// Header del binario
    header: FastOSHeader,
    /// Secciones del binario
    sections: Vec<FastOSSection>,
    /// Datos de cada sección
    section_data: Vec<Vec<u8>>,
}

impl FastOSGenerator {
    /// Crear generador para un ejecutable FastOS 64-bit (default)
    pub fn new_executable() -> Self {
        Self {
            header: FastOSHeader::new(FastOSCpuMode::Long64, FastOSBinaryType::Executable),
            sections: Vec::new(),
            section_data: Vec::new(),
        }
    }

    /// Crear generador para un kernel FastOS
    pub fn new_kernel(cpu_mode: FastOSCpuMode) -> Self {
        Self {
            header: FastOSHeader::new(cpu_mode, FastOSBinaryType::Kernel),
            sections: Vec::new(),
            section_data: Vec::new(),
        }
    }

    /// Crear generador con modo de CPU específico
    pub fn new(cpu_mode: FastOSCpuMode, binary_type: FastOSBinaryType) -> Self {
        Self {
            header: FastOSHeader::new(cpu_mode, binary_type),
            sections: Vec::new(),
            section_data: Vec::new(),
        }
    }

    /// Establecer base address
    pub fn set_base_address(&mut self, addr: u64) {
        self.header.base_address = addr;
    }

    /// Establecer entry point
    pub fn set_entry_point(&mut self, addr: u64) {
        self.header.entry_point = addr;
    }

    /// Agregar sección .text (código)
    pub fn add_text(&mut self, code: &[u8]) {
        let offset = self.calculate_next_offset();
        let vaddr = self.header.base_address + offset as u64;

        self.sections.push(FastOSSection {
            section_type: FastOSSectionType::Text,
            flags: FastOSSectionFlags::text(),
            file_offset: offset as u32,
            file_size: code.len() as u32,
            vaddr,
            mem_size: code.len() as u64,
            alignment: 16,
        });
        self.section_data.push(code.to_vec());
        self.header.code_size = code.len() as u32;

        // Entry point = start of .text by default
        if self.header.entry_point == 0 {
            self.header.entry_point = vaddr;
        }
    }

    /// Agregar sección .data (datos inicializados)
    pub fn add_data(&mut self, data: &[u8]) {
        let offset = self.calculate_next_offset();
        let vaddr = self.header.base_address + offset as u64;

        self.sections.push(FastOSSection {
            section_type: FastOSSectionType::Data,
            flags: FastOSSectionFlags::data(),
            file_offset: offset as u32,
            file_size: data.len() as u32,
            vaddr,
            mem_size: data.len() as u64,
            alignment: 8,
        });
        self.section_data.push(data.to_vec());
        self.header.data_size = data.len() as u32;
    }

    /// Agregar sección .rodata (datos de solo lectura)
    pub fn add_rodata(&mut self, data: &[u8]) {
        let offset = self.calculate_next_offset();
        let vaddr = self.header.base_address + offset as u64;

        self.sections.push(FastOSSection {
            section_type: FastOSSectionType::RoData,
            flags: FastOSSectionFlags::rodata(),
            file_offset: offset as u32,
            file_size: data.len() as u32,
            vaddr,
            mem_size: data.len() as u64,
            alignment: 8,
        });
        self.section_data.push(data.to_vec());
    }

    /// Agregar sección .bss (datos no inicializados)
    pub fn add_bss(&mut self, size: u64) {
        let offset = self.calculate_next_offset();
        let vaddr = self.header.base_address + offset as u64;

        self.sections.push(FastOSSection {
            section_type: FastOSSectionType::Bss,
            flags: FastOSSectionFlags::bss(),
            file_offset: offset as u32,
            file_size: 0, // BSS no ocupa espacio en archivo
            vaddr,
            mem_size: size,
            alignment: 16,
        });
        self.section_data.push(Vec::new());
        self.header.bss_size = size as u32;
    }

    /// Generar el binario FastOS completo
    pub fn generate(&mut self) -> Vec<u8> {
        self.header.section_count = self.sections.len() as u16;

        let mut binary = Vec::new();

        // 1. Header (64 bytes)
        let header_bytes = self.header.to_bytes();
        binary.extend_from_slice(&header_bytes);

        // 2. Section table
        for section in &self.sections {
            binary.extend_from_slice(&self.serialize_section(section));
        }

        // 3. Section data (aligned)
        for data in &self.section_data {
            if !data.is_empty() {
                // Align to 16 bytes
                while binary.len() % 16 != 0 {
                    binary.push(0x00);
                }
                binary.extend_from_slice(data);
            }
        }

        // Update total size
        self.header.total_file_size = binary.len() as u32;

        // Recalculate checksum
        self.header.checksum = self.crc32(&binary);

        // Rewrite header with final values
        let final_header = self.header.to_bytes();
        binary[..FASTOS_HEADER_SIZE].copy_from_slice(&final_header);

        binary
    }

    /// Generar y escribir a archivo
    pub fn write_to_file(&mut self, path: &Path) -> io::Result<usize> {
        let binary = self.generate();
        let size = binary.len();
        fs::write(path, &binary)?;
        Ok(size)
    }

    /// Generar un kernel FastOS completo desde código y datos
    pub fn generate_kernel(code: &[u8], data: &[u8], cpu_mode: FastOSCpuMode) -> Vec<u8> {
        let mut gen = FastOSGenerator::new_kernel(cpu_mode);
        gen.add_text(code);
        if !data.is_empty() {
            gen.add_data(data);
        }
        gen.generate()
    }

    /// Generar un ejecutable FastOS 64-bit desde código y datos
    pub fn generate_executable(code: &[u8], data: &[u8]) -> Vec<u8> {
        let mut gen = FastOSGenerator::new_executable();
        gen.add_text(code);
        if !data.is_empty() {
            gen.add_data(data);
        }
        gen.generate()
    }

    // ---- Internal helpers ----

    fn calculate_next_offset(&self) -> usize {
        let table_end = FASTOS_HEADER_SIZE + (self.sections.len() + 1) * FASTOS_SECTION_ENTRY_SIZE;

        // Find end of last section data
        let data_end: usize = self
            .sections
            .iter()
            .zip(self.section_data.iter())
            .map(|(s, d)| s.file_offset as usize + d.len())
            .max()
            .unwrap_or(0);

        let offset = table_end.max(data_end);
        // Align to 16
        (offset + 15) & !15
    }

    fn serialize_section(&self, section: &FastOSSection) -> Vec<u8> {
        let mut buf = Vec::with_capacity(FASTOS_SECTION_ENTRY_SIZE);

        buf.push(section.section_type as u8); // [0]
        buf.push(section.flags.0); // [1]
        buf.extend_from_slice(&[0u8; 2]); // [2..4] reserved
        buf.extend_from_slice(&section.file_offset.to_le_bytes()); // [4..8]
        buf.extend_from_slice(&section.file_size.to_le_bytes()); // [8..12]
        buf.extend_from_slice(&section.vaddr.to_le_bytes()); // [12..20]
        buf.extend_from_slice(&section.mem_size.to_le_bytes()); // [20..28]
        buf.extend_from_slice(&section.alignment.to_le_bytes()); // [28..32]

        assert_eq!(buf.len(), FASTOS_SECTION_ENTRY_SIZE);
        buf
    }

    /// CRC32 simple (IEEE 802.3)
    fn crc32(&self, data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFFFFFF;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }
}

// ============================================================
// FastOS Disk Image Generator
// ============================================================

/// Genera una imagen de disco booteable para FastOS.
///
/// Layout del disco:
///   Sector 0:     Boot sector (512 bytes) — carga stage2
///   Sector 1-8:   Stage2 bootloader (4KB) — modo switch + carga kernel
///   Sector 9+:    Kernel FastOS (.fos format)
pub struct FastOSDiskImage {
    /// Boot sector (512 bytes)
    boot_sector: Vec<u8>,
    /// Stage2 bootloader
    stage2: Vec<u8>,
    /// Kernel image
    kernel: Vec<u8>,
}

impl FastOSDiskImage {
    pub fn new() -> Self {
        Self {
            boot_sector: Vec::new(),
            stage2: Vec::new(),
            kernel: Vec::new(),
        }
    }

    /// Establecer boot sector (debe ser 512 bytes con 0x55AA)
    pub fn set_boot_sector(&mut self, data: &[u8]) {
        let mut sector = vec![0u8; 512];
        let len = data.len().min(510);
        sector[..len].copy_from_slice(&data[..len]);
        sector[510] = 0x55;
        sector[511] = 0xAA;
        self.boot_sector = sector;
    }

    /// Establecer stage2 bootloader
    pub fn set_stage2(&mut self, data: &[u8]) {
        self.stage2 = data.to_vec();
        // Pad to sector boundary (512 bytes)
        while self.stage2.len() % 512 != 0 {
            self.stage2.push(0x00);
        }
    }

    /// Establecer kernel image
    pub fn set_kernel(&mut self, data: &[u8]) {
        self.kernel = data.to_vec();
        // Pad to sector boundary
        while self.kernel.len() % 512 != 0 {
            self.kernel.push(0x00);
        }
    }

    /// Generar imagen de disco completa
    pub fn generate(&self) -> Vec<u8> {
        let mut image = Vec::new();
        image.extend_from_slice(&self.boot_sector);
        image.extend_from_slice(&self.stage2);
        image.extend_from_slice(&self.kernel);

        // Mínimo: 1.44MB floppy (no obligatorio para QEMU con -drive)
        // Pero asegurar al menos los sectores necesarios
        image
    }

    /// Escribir imagen de disco a archivo
    pub fn write_to_file(&self, path: &Path) -> io::Result<usize> {
        let image = self.generate();
        let size = image.len();
        fs::write(path, &image)?;
        Ok(size)
    }
}

// ============================================================
// Comparación de formatos
// ============================================================
//
// | Característica    | PE (Windows) | ELF (Linux) | FastOS (ADead-BIB) |
// |-------------------|-------------|-------------|-------------------|
// | Header size       | ~400+ bytes | 64 bytes    | 64 bytes          |
// | Magic             | "MZ"        | "\x7FELF"   | "FsOS"            |
// | Section headers   | ~40 bytes   | 64 bytes    | 32 bytes          |
// | Dynamic linking   | IAT/DLL     | PLT/GOT     | No (directo)      |
// | Multi-mode        | No          | No          | 16/32/64-bit      |
// | Boot support      | No          | No          | Nativo            |
// | Checksum          | Opcional    | No          | CRC32 siempre     |
// | Complejidad       | Alta        | Media       | Baja              |
//
// ============================================================

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fastos_header_roundtrip() {
        let header = FastOSHeader::new(FastOSCpuMode::Long64, FastOSBinaryType::Kernel);
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 64);
        assert_eq!(&bytes[0..4], b"FsOS");

        let parsed = FastOSHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.cpu_mode, FastOSCpuMode::Long64);
        assert_eq!(parsed.version, FASTOS_VERSION);
    }

    #[test]
    fn test_fastos_header_size() {
        let header = FastOSHeader::new(FastOSCpuMode::Long64, FastOSBinaryType::Executable);
        assert_eq!(header.to_bytes().len(), 64);
    }

    #[test]
    fn test_fastos_cpu_modes() {
        assert_eq!(FastOSCpuMode::Real16.operand_size(), 16);
        assert_eq!(FastOSCpuMode::Protected32.operand_size(), 32);
        assert_eq!(FastOSCpuMode::Long64.operand_size(), 64);
        assert_eq!(FastOSCpuMode::default(), FastOSCpuMode::Long64);
    }

    #[test]
    fn test_fastos_generate_executable() {
        let code = vec![0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00]; // mov rax, 1
        let binary = FastOSGenerator::generate_executable(&code, &[]);

        // Verify magic
        assert_eq!(&binary[0..4], b"FsOS");
        // Verify CPU mode = Long64
        assert_eq!(binary[6], 0x40);
        // Verify binary type = Executable
        assert_eq!(binary[7], 0x01);
        // Verify code is present
        assert!(binary.len() > 64);
    }

    #[test]
    fn test_fastos_generate_kernel() {
        let code = vec![0xFA, 0xF4]; // CLI; HLT
        let data = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello"
        let binary = FastOSGenerator::generate_kernel(&code, &data, FastOSCpuMode::Long64);

        assert_eq!(&binary[0..4], b"FsOS");
        assert_eq!(binary[7], 0x02); // Kernel type
    }

    #[test]
    fn test_fastos_section_flags() {
        let text = FastOSSectionFlags::text();
        assert_ne!(text.0 & FastOSSectionFlags::EXEC, 0);
        assert_ne!(text.0 & FastOSSectionFlags::READ, 0);
        assert_eq!(text.0 & FastOSSectionFlags::WRITE, 0);

        let data = FastOSSectionFlags::data();
        assert_ne!(data.0 & FastOSSectionFlags::WRITE, 0);
        assert_eq!(data.0 & FastOSSectionFlags::EXEC, 0);
    }

    #[test]
    fn test_fastos_disk_image() {
        let mut disk = FastOSDiskImage::new();
        disk.set_boot_sector(&[0xFA, 0xF4, 0xEB, 0xFE]); // CLI; HLT; JMP $
        disk.set_kernel(&[0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00]);

        let image = disk.generate();
        assert!(image.len() >= 512);
        assert_eq!(image[510], 0x55);
        assert_eq!(image[511], 0xAA);
    }

    #[test]
    fn test_fastos_cpu_mode_from_byte() {
        assert_eq!(FastOSCpuMode::from_byte(0x10), Some(FastOSCpuMode::Real16));
        assert_eq!(
            FastOSCpuMode::from_byte(0x20),
            Some(FastOSCpuMode::Protected32)
        );
        assert_eq!(FastOSCpuMode::from_byte(0x40), Some(FastOSCpuMode::Long64));
        assert_eq!(FastOSCpuMode::from_byte(0xFF), None);
    }

    #[test]
    fn test_fastos_multi_section() {
        let mut gen = FastOSGenerator::new_executable();
        gen.add_text(&[0x90; 32]); // 32 NOPs
        gen.add_data(&[0x41, 0x42, 0x43]); // "ABC"
        gen.add_rodata(&[0x48, 0x65, 0x6C, 0x6C, 0x6F]); // "Hello"
        gen.add_bss(1024);

        let binary = gen.generate();
        assert_eq!(&binary[0..4], b"FsOS");

        // Parse back header
        let header = FastOSHeader::from_bytes(&binary).unwrap();
        assert_eq!(header.section_count, 4);
        assert_eq!(header.code_size, 32);
        assert_eq!(header.data_size, 3);
        assert_eq!(header.bss_size, 1024);
    }
}
