// ============================================================
// BG — Binary Guardian: Binary Loader
// ============================================================
// Carga binarios PE/ELF/raw y extrae secciones de código
// para análisis pre-execution.
//
// Usa goblin para parsing de PE/ELF.
// Para raw binaries (boot sectors, firmware), trata todo como código.
//
// Autor: Eddi Andreé Salazar Matos
// ============================================================

use std::fmt;
use std::path::Path;

/// Tipo de formato binario detectado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFormat {
    PE,
    ELF,
    Raw,
}

impl fmt::Display for BinaryFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryFormat::PE => write!(f, "PE (Windows)"),
            BinaryFormat::ELF => write!(f, "ELF (Linux)"),
            BinaryFormat::Raw => write!(f, "Raw Binary"),
        }
    }
}

/// Tipo de sección.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Code,
    Data,
    ReadOnly,
    RWX,
    Unknown,
}

impl fmt::Display for SectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SectionKind::Code => write!(f, "CODE"),
            SectionKind::Data => write!(f, "DATA"),
            SectionKind::ReadOnly => write!(f, "RODATA"),
            SectionKind::RWX => write!(f, "RWX"),
            SectionKind::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Información de una sección del binario.
#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub name: String,
    pub kind: SectionKind,
    pub offset: usize,
    pub size: usize,
    pub virtual_address: usize,
    pub executable: bool,
    pub writable: bool,
    pub readable: bool,
}

impl fmt::Display for SectionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "    {:16} {:6}  offset=0x{:08X}  size={}  VA=0x{:08X}  [{}{}{}]",
            self.name,
            self.kind,
            self.offset,
            self.size,
            self.virtual_address,
            if self.readable { "R" } else { "-" },
            if self.writable { "W" } else { "-" },
            if self.executable { "X" } else { "-" },
        )
    }
}

/// Información extraída de un binario.
#[derive(Debug, Clone)]
pub struct BinaryInfo {
    pub path: String,
    pub format: BinaryFormat,
    pub sections: Vec<SectionInfo>,
    pub code_bytes: Vec<u8>,
    pub total_size: usize,
    pub entry_point: usize,
    pub rwx_count: usize,
    /// Imports extraídos del binario (nombre de función)
    pub imports: Vec<ImportEntry>,
    /// Exports extraídos del binario
    pub exports: Vec<String>,
    /// Header size (para calcular header ratio)
    pub header_size: usize,
}

/// Una entrada de import: biblioteca + nombre de función.
#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub library: String,
    pub function: String,
}

impl fmt::Display for BinaryInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "═══════════════════════════════════════════════")?;
        writeln!(f, "  BG — Binary Info")?;
        writeln!(f, "═══════════════════════════════════════════════")?;
        writeln!(f, "  Path:          {}", self.path)?;
        writeln!(f, "  Format:        {}", self.format)?;
        writeln!(f, "  Size:          {} bytes", self.total_size)?;
        writeln!(f, "  Entry point:   0x{:08X}", self.entry_point)?;
        writeln!(f, "  Sections:      {}", self.sections.len())?;
        writeln!(f, "  Code bytes:    {}", self.code_bytes.len())?;
        writeln!(f, "  RWX sections:  {}", self.rwx_count)?;
        if !self.imports.is_empty() {
            writeln!(f, "  Imports:       {}", self.imports.len())?;
        }
        if !self.exports.is_empty() {
            writeln!(f, "  Exports:       {}", self.exports.len())?;
        }
        writeln!(f)?;
        for sec in &self.sections {
            writeln!(f, "{}", sec)?;
        }
        Ok(())
    }
}

/// Binary Loader — Carga y parsea binarios PE/ELF/Raw.
pub struct BinaryLoader;

impl BinaryLoader {
    /// Carga un binario desde un archivo y extrae su información.
    pub fn load_file(path: &Path) -> Result<BinaryInfo, String> {
        let data =
            std::fs::read(path).map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;
        let name = path.to_string_lossy().to_string();
        Self::load_bytes(&data, &name)
    }

    /// Carga un binario desde bytes en memoria.
    pub fn load_bytes(data: &[u8], name: &str) -> Result<BinaryInfo, String> {
        match goblin::Object::parse(data) {
            Ok(goblin::Object::PE(pe)) => Ok(Self::load_pe(&pe, data, name)),
            Ok(goblin::Object::Elf(elf)) => Ok(Self::load_elf(&elf, data, name)),
            _ => Ok(Self::load_raw(data, name)),
        }
    }

    /// Parsea un PE (Windows executable).
    fn load_pe(pe: &goblin::pe::PE, data: &[u8], name: &str) -> BinaryInfo {
        let mut sections = Vec::new();
        let mut code_bytes = Vec::new();
        let mut rwx_count = 0;
        let header_size = pe
            .header
            .optional_header
            .map(|oh| oh.windows_fields.size_of_headers as usize)
            .unwrap_or(0);

        for section in &pe.sections {
            let sec_name = String::from_utf8_lossy(
                &section.name[..section.name.iter().position(|&b| b == 0).unwrap_or(8)],
            )
            .to_string();

            let characteristics = section.characteristics;
            let executable = characteristics & 0x20000000 != 0; // IMAGE_SCN_MEM_EXECUTE
            let writable = characteristics & 0x80000000 != 0; // IMAGE_SCN_MEM_WRITE
            let readable = characteristics & 0x40000000 != 0; // IMAGE_SCN_MEM_READ

            let kind = if executable && writable {
                rwx_count += 1;
                SectionKind::RWX
            } else if executable {
                SectionKind::Code
            } else if !writable && readable {
                SectionKind::ReadOnly
            } else if writable {
                SectionKind::Data
            } else {
                SectionKind::Unknown
            };

            let offset = section.pointer_to_raw_data as usize;
            let size = section.size_of_raw_data as usize;
            let virtual_address = section.virtual_address as usize;

            if executable && offset + size <= data.len() {
                code_bytes.extend_from_slice(&data[offset..offset + size]);
            }

            sections.push(SectionInfo {
                name: sec_name,
                kind,
                offset,
                size,
                virtual_address,
                executable,
                writable,
                readable,
            });
        }

        // Extract imports
        let mut imports = Vec::new();
        for imp in &pe.imports {
            imports.push(ImportEntry {
                library: imp.dll.to_string(),
                function: imp.name.to_string(),
            });
        }

        // Extract exports
        let exports: Vec<String> = pe
            .exports
            .iter()
            .filter_map(|e| e.name.map(|n| n.to_string()))
            .collect();

        let entry_point = pe.entry as usize;

        BinaryInfo {
            path: name.to_string(),
            format: BinaryFormat::PE,
            sections,
            code_bytes,
            total_size: data.len(),
            entry_point,
            rwx_count,
            imports,
            exports,
            header_size,
        }
    }

    /// Parsea un ELF (Linux executable).
    fn load_elf(elf: &goblin::elf::Elf, data: &[u8], name: &str) -> BinaryInfo {
        let mut sections = Vec::new();
        let mut code_bytes = Vec::new();
        let mut rwx_count = 0;

        // Estimate header size from first section offset
        let header_size = elf
            .section_headers
            .first()
            .map(|s| s.sh_offset as usize)
            .unwrap_or(64);

        for sh in &elf.section_headers {
            let sec_name = elf.shdr_strtab.get_at(sh.sh_name).unwrap_or("").to_string();

            let executable = sh.sh_flags & 0x4 != 0; // SHF_EXECINSTR
            let writable = sh.sh_flags & 0x1 != 0; // SHF_WRITE
            let allocatable = sh.sh_flags & 0x2 != 0; // SHF_ALLOC

            let kind = if executable && writable {
                rwx_count += 1;
                SectionKind::RWX
            } else if executable {
                SectionKind::Code
            } else if !writable && allocatable {
                SectionKind::ReadOnly
            } else if writable {
                SectionKind::Data
            } else {
                SectionKind::Unknown
            };

            let offset = sh.sh_offset as usize;
            let size = sh.sh_size as usize;
            let virtual_address = sh.sh_addr as usize;

            if executable && sh.sh_type == goblin::elf::section_header::SHT_PROGBITS {
                if offset + size <= data.len() {
                    code_bytes.extend_from_slice(&data[offset..offset + size]);
                }
            }

            if allocatable {
                sections.push(SectionInfo {
                    name: sec_name,
                    kind,
                    offset,
                    size,
                    virtual_address,
                    executable,
                    writable,
                    readable: allocatable,
                });
            }
        }

        // Extract imports from dynamic symbols
        let mut imports = Vec::new();
        for sym in &elf.dynsyms {
            if sym.is_import() {
                let name_str = elf.dynstrtab.get_at(sym.st_name).unwrap_or("");
                if !name_str.is_empty() {
                    imports.push(ImportEntry {
                        library: String::new(), // ELF doesn't tie imports to specific libs directly
                        function: name_str.to_string(),
                    });
                }
            }
        }

        // Extract exports from dynamic symbols
        let exports: Vec<String> = elf
            .dynsyms
            .iter()
            .filter(|s| !s.is_import() && s.st_bind() == goblin::elf::sym::STB_GLOBAL)
            .filter_map(|s| {
                let n = elf.dynstrtab.get_at(s.st_name).unwrap_or("");
                if n.is_empty() {
                    None
                } else {
                    Some(n.to_string())
                }
            })
            .collect();

        let entry_point = elf.entry as usize;

        BinaryInfo {
            path: name.to_string(),
            format: BinaryFormat::ELF,
            sections,
            code_bytes,
            total_size: data.len(),
            entry_point,
            rwx_count,
            imports,
            exports,
            header_size,
        }
    }

    /// Trata el binario como flat raw binary (boot sector, firmware).
    fn load_raw(data: &[u8], name: &str) -> BinaryInfo {
        BinaryInfo {
            path: name.to_string(),
            format: BinaryFormat::Raw,
            sections: vec![SectionInfo {
                name: ".text".to_string(),
                kind: SectionKind::Code,
                offset: 0,
                size: data.len(),
                virtual_address: 0,
                executable: true,
                writable: false,
                readable: true,
            }],
            code_bytes: data.to_vec(),
            total_size: data.len(),
            entry_point: 0,
            rwx_count: 0,
            imports: Vec::new(),
            exports: Vec::new(),
            header_size: 0,
        }
    }

    /// Valida integridad estructural del binario.
    /// Retorna (entry_valid, overlapping_sections, anomalous_permissions).
    pub fn validate_structure(info: &BinaryInfo) -> (bool, bool, usize) {
        // 1. Validar entry point
        let entry_valid = if info.format == BinaryFormat::Raw {
            true // Raw binaries always start at 0
        } else {
            info.sections.iter().any(|s| {
                s.executable
                    && info.entry_point >= s.virtual_address
                    && info.entry_point < s.virtual_address + s.size
            })
        };

        // 2. Detectar secciones solapadas
        let mut overlapping = false;
        for i in 0..info.sections.len() {
            for j in (i + 1)..info.sections.len() {
                let a = &info.sections[i];
                let b = &info.sections[j];
                if a.offset < b.offset + b.size && b.offset < a.offset + a.size {
                    overlapping = true;
                }
            }
        }

        // 3. Contar permisos anómalos (data sections con execute flag)
        let anomalous = info
            .sections
            .iter()
            .filter(|s| s.kind == SectionKind::RWX || (s.executable && s.writable))
            .count();

        (entry_valid, overlapping, anomalous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_binary() {
        let data = vec![0x55, 0x48, 0x89, 0xE5, 0xC3];
        let info = BinaryLoader::load_bytes(&data, "test.bin").unwrap();
        assert_eq!(info.format, BinaryFormat::Raw);
        assert_eq!(info.code_bytes.len(), 5);
        assert_eq!(info.sections.len(), 1);
        assert_eq!(info.entry_point, 0);
    }

    #[test]
    fn test_empty_binary() {
        let data = Vec::new();
        let info = BinaryLoader::load_bytes(&data, "empty").unwrap();
        assert_eq!(info.format, BinaryFormat::Raw);
        assert_eq!(info.code_bytes.len(), 0);
    }

    #[test]
    fn test_display() {
        let data = vec![0x55, 0x48, 0x89, 0xE5, 0xC3];
        let info = BinaryLoader::load_bytes(&data, "test.bin").unwrap();
        let s = format!("{}", info);
        assert!(s.contains("Raw Binary"));
        assert!(s.contains("test.bin"));
    }

    #[test]
    fn test_validate_raw() {
        let data = vec![0x55, 0x48, 0x89, 0xE5, 0xC3];
        let info = BinaryLoader::load_bytes(&data, "test.bin").unwrap();
        let (valid, overlapping, anomalous) = BinaryLoader::validate_structure(&info);
        assert!(valid);
        assert!(!overlapping);
        assert_eq!(anomalous, 0);
    }
}
