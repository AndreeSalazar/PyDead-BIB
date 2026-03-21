// ============================================================
// Cache Deserializer — bytes → AST desde fastos.bib
// ============================================================

use super::{
    ADeadCache, CachedSymbol, CachedSymbolKind, CachedType, CachedTypeKind, CachedUBReport,
    ImplTable, SymbolTable, TypeTable, CACHE_MAGIC, CACHE_VERSION,
};

/// Error de deserializacion
#[derive(Debug)]
pub enum DeserializeError {
    InvalidMagic,
    VersionMismatch { expected: u32, got: u32 },
    TruncatedData,
    IoError(std::io::Error),
}

impl std::fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DeserializeError::InvalidMagic => {
                write!(f, "Invalid cache magic bytes (not fastos.bib)")
            }
            DeserializeError::VersionMismatch { expected, got } => write!(
                f,
                "Cache version mismatch: expected {}, got {}",
                expected, got
            ),
            DeserializeError::TruncatedData => write!(f, "Cache file truncated"),
            DeserializeError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for DeserializeError {}

impl From<std::io::Error> for DeserializeError {
    fn from(e: std::io::Error) -> Self {
        DeserializeError::IoError(e)
    }
}

fn read_string(bytes: &[u8], pos: &mut usize) -> Result<String, DeserializeError> {
    if *pos + 4 > bytes.len() {
        return Err(DeserializeError::TruncatedData);
    }
    let len = u32::from_le_bytes([bytes[*pos], bytes[*pos + 1], bytes[*pos + 2], bytes[*pos + 3]])
        as usize;
    *pos += 4;
    if *pos + len > bytes.len() {
        return Err(DeserializeError::TruncatedData);
    }
    let s = String::from_utf8_lossy(&bytes[*pos..*pos + len]).to_string();
    *pos += len;
    Ok(s)
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, DeserializeError> {
    if *pos + 4 > bytes.len() {
        return Err(DeserializeError::TruncatedData);
    }
    let val = u32::from_le_bytes([bytes[*pos], bytes[*pos + 1], bytes[*pos + 2], bytes[*pos + 3]]);
    *pos += 4;
    Ok(val)
}

fn read_u64(bytes: &[u8], pos: &mut usize) -> Result<u64, DeserializeError> {
    if *pos + 8 > bytes.len() {
        return Err(DeserializeError::TruncatedData);
    }
    let val = u64::from_le_bytes([
        bytes[*pos],
        bytes[*pos + 1],
        bytes[*pos + 2],
        bytes[*pos + 3],
        bytes[*pos + 4],
        bytes[*pos + 5],
        bytes[*pos + 6],
        bytes[*pos + 7],
    ]);
    *pos += 8;
    Ok(val)
}

fn read_u8(bytes: &[u8], pos: &mut usize) -> Result<u8, DeserializeError> {
    if *pos >= bytes.len() {
        return Err(DeserializeError::TruncatedData);
    }
    let val = bytes[*pos];
    *pos += 1;
    Ok(val)
}

/// Deserializa bytes a ADeadCache
pub fn deserialize(bytes: &[u8]) -> Result<ADeadCache, DeserializeError> {
    if bytes.len() < 32 {
        return Err(DeserializeError::TruncatedData);
    }

    let mut pos = 0;

    // Magic (8 bytes)
    let mut magic = [0u8; 8];
    magic.copy_from_slice(&bytes[pos..pos + 8]);
    pos += 8;
    if magic != CACHE_MAGIC {
        return Err(DeserializeError::InvalidMagic);
    }

    // Version
    let version = read_u32(bytes, &mut pos)?;
    if version != CACHE_VERSION {
        return Err(DeserializeError::VersionMismatch {
            expected: CACHE_VERSION,
            got: version,
        });
    }

    // Timestamp
    let timestamp = read_u64(bytes, &mut pos)?;

    // Hash
    let hash = read_u64(bytes, &mut pos)?;

    // AST data
    let ast_len = read_u32(bytes, &mut pos)? as usize;
    if pos + ast_len > bytes.len() {
        return Err(DeserializeError::TruncatedData);
    }
    let ast_data = bytes[pos..pos + ast_len].to_vec();
    pos += ast_len;

    // Types
    let types_count = read_u32(bytes, &mut pos)? as usize;
    let mut types = TypeTable::new();
    for _ in 0..types_count {
        let key = read_string(bytes, &mut pos)?;
        let name = read_string(bytes, &mut pos)?;
        let size = read_u32(bytes, &mut pos)? as usize;
        let alignment = read_u32(bytes, &mut pos)? as usize;
        let kind_tag = read_u8(bytes, &mut pos)?;
        let kind = match kind_tag {
            0 => CachedTypeKind::Primitive,
            1 => {
                let field_count = read_u32(bytes, &mut pos)? as usize;
                let mut fields = Vec::with_capacity(field_count);
                for _ in 0..field_count {
                    let fname = read_string(bytes, &mut pos)?;
                    let ftype = read_string(bytes, &mut pos)?;
                    fields.push((fname, ftype));
                }
                CachedTypeKind::Struct { fields }
            }
            2 => {
                let pointee = read_string(bytes, &mut pos)?;
                CachedTypeKind::Pointer { pointee }
            }
            3 => {
                let element = read_string(bytes, &mut pos)?;
                let size_val = read_u64(bytes, &mut pos)? as usize;
                CachedTypeKind::Array {
                    element,
                    size: size_val,
                }
            }
            4 => {
                let param_count = read_u32(bytes, &mut pos)? as usize;
                let mut params = Vec::with_capacity(param_count);
                for _ in 0..param_count {
                    params.push(read_string(bytes, &mut pos)?);
                }
                let ret = read_string(bytes, &mut pos)?;
                CachedTypeKind::Function { params, ret }
            }
            _ => return Err(DeserializeError::TruncatedData),
        };
        types.insert(
            key,
            CachedType {
                name,
                size,
                alignment,
                kind,
            },
        );
    }

    // Symbols
    let symbols_count = read_u32(bytes, &mut pos)? as usize;
    let mut symbols = SymbolTable::new();
    for _ in 0..symbols_count {
        let key = read_string(bytes, &mut pos)?;
        let name = read_string(bytes, &mut pos)?;
        let source_file = read_string(bytes, &mut pos)?;
        let kind_tag = read_u8(bytes, &mut pos)?;
        let kind = match kind_tag {
            0 => {
                let param_count = read_u32(bytes, &mut pos)? as usize;
                let mut params = Vec::with_capacity(param_count);
                for _ in 0..param_count {
                    params.push(read_string(bytes, &mut pos)?);
                }
                let ret = read_string(bytes, &mut pos)?;
                CachedSymbolKind::Function { params, ret }
            }
            1 => {
                let ty = read_string(bytes, &mut pos)?;
                CachedSymbolKind::Variable { ty }
            }
            2 => {
                let value = read_string(bytes, &mut pos)?;
                CachedSymbolKind::Macro { value }
            }
            _ => return Err(DeserializeError::TruncatedData),
        };
        symbols.insert(
            key,
            CachedSymbol {
                name,
                kind,
                source_file,
            },
        );
    }

    // UB reports
    let ub_count = read_u32(bytes, &mut pos)? as usize;
    let mut ub_reports = Vec::with_capacity(ub_count);
    for _ in 0..ub_count {
        let kind = read_string(bytes, &mut pos)?;
        let severity = read_string(bytes, &mut pos)?;
        let message = read_string(bytes, &mut pos)?;
        let location = read_string(bytes, &mut pos)?;
        ub_reports.push(CachedUBReport {
            kind,
            severity,
            message,
            location,
        });
    }

    Ok(ADeadCache {
        magic,
        version,
        timestamp,
        hash,
        ast_data,
        types,
        symbols,
        ub_reports,
        impls: ImplTable::new(),
    })
}

/// Lee y deserializa un archivo fastos.bib
pub fn read_from_file(path: &str) -> Result<ADeadCache, DeserializeError> {
    let bytes = std::fs::read(path)?;
    deserialize(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::serializer;

    #[test]
    fn test_roundtrip() {
        let cache = ADeadCache::new(0xCAFEBABE);
        let bytes = serializer::serialize(&cache);
        let result = deserialize(&bytes);
        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.hash, 0xCAFEBABE);
        assert!(loaded.is_valid());
    }

    #[test]
    fn test_invalid_magic() {
        let bytes = vec![0u8; 44];
        let result = deserialize(&bytes);
        assert!(matches!(result, Err(DeserializeError::InvalidMagic)));
    }
}
