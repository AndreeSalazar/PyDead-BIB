// ============================================================
// Cache Serializer — AST → bytes para fastos.bib
// ============================================================

use super::ADeadCache;

/// Write a length-prefixed string to the byte buffer
fn write_string(bytes: &mut Vec<u8>, s: &str) {
    let len = s.len() as u32;
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(s.as_bytes());
}

/// Serializa un ADeadCache a bytes para escribir a fastos.bib
pub fn serialize(cache: &ADeadCache) -> Vec<u8> {
    let mut bytes = Vec::new();

    // Header: magic (8 bytes)
    bytes.extend_from_slice(&cache.magic);

    // Version (4 bytes, little-endian)
    bytes.extend_from_slice(&cache.version.to_le_bytes());

    // Timestamp (8 bytes)
    bytes.extend_from_slice(&cache.timestamp.to_le_bytes());

    // Hash (8 bytes)
    bytes.extend_from_slice(&cache.hash.to_le_bytes());

    // AST data length + data
    let ast_len = cache.ast_data.len() as u32;
    bytes.extend_from_slice(&ast_len.to_le_bytes());
    bytes.extend_from_slice(&cache.ast_data);

    // Types: count + entries
    let types_count = cache.types.entries.len() as u32;
    bytes.extend_from_slice(&types_count.to_le_bytes());
    for (name, ty) in &cache.types.entries {
        write_string(&mut bytes, name);
        write_string(&mut bytes, &ty.name);
        bytes.extend_from_slice(&(ty.size as u32).to_le_bytes());
        bytes.extend_from_slice(&(ty.alignment as u32).to_le_bytes());
        // Kind tag
        match &ty.kind {
            super::CachedTypeKind::Primitive => bytes.push(0),
            super::CachedTypeKind::Struct { fields } => {
                bytes.push(1);
                bytes.extend_from_slice(&(fields.len() as u32).to_le_bytes());
                for (fname, ftype) in fields {
                    write_string(&mut bytes, fname);
                    write_string(&mut bytes, ftype);
                }
            }
            super::CachedTypeKind::Pointer { pointee } => {
                bytes.push(2);
                write_string(&mut bytes, pointee);
            }
            super::CachedTypeKind::Array { element, size } => {
                bytes.push(3);
                write_string(&mut bytes, element);
                bytes.extend_from_slice(&(*size as u64).to_le_bytes());
            }
            super::CachedTypeKind::Function { params, ret } => {
                bytes.push(4);
                bytes.extend_from_slice(&(params.len() as u32).to_le_bytes());
                for p in params {
                    write_string(&mut bytes, p);
                }
                write_string(&mut bytes, ret);
            }
        }
    }

    // Symbols: count + entries
    let symbols_count = cache.symbols.entries.len() as u32;
    bytes.extend_from_slice(&symbols_count.to_le_bytes());
    for (name, sym) in &cache.symbols.entries {
        write_string(&mut bytes, name);
        write_string(&mut bytes, &sym.name);
        write_string(&mut bytes, &sym.source_file);
        match &sym.kind {
            super::CachedSymbolKind::Function { params, ret } => {
                bytes.push(0);
                bytes.extend_from_slice(&(params.len() as u32).to_le_bytes());
                for p in params {
                    write_string(&mut bytes, p);
                }
                write_string(&mut bytes, ret);
            }
            super::CachedSymbolKind::Variable { ty } => {
                bytes.push(1);
                write_string(&mut bytes, ty);
            }
            super::CachedSymbolKind::Macro { value } => {
                bytes.push(2);
                write_string(&mut bytes, value);
            }
        }
    }

    // UB reports: count + entries
    let ub_count = cache.ub_reports.len() as u32;
    bytes.extend_from_slice(&ub_count.to_le_bytes());
    for ub in &cache.ub_reports {
        write_string(&mut bytes, &ub.kind);
        write_string(&mut bytes, &ub.severity);
        write_string(&mut bytes, &ub.message);
        write_string(&mut bytes, &ub.location);
    }

    bytes
}

/// Escribe el cache a un archivo fastos.bib
pub fn write_to_file(cache: &ADeadCache, path: &str) -> std::io::Result<usize> {
    let bytes = serialize(cache);
    std::fs::write(path, &bytes)?;
    Ok(bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_header() {
        let cache = ADeadCache::new(0xDEADBEEF);
        let bytes = serialize(&cache);
        // Minimum: 8 (magic) + 4 (version) + 8 (timestamp) + 8 (hash) + 4 (ast_len) + 4+4+4 (counts)
        assert!(bytes.len() >= 44);
        assert_eq!(&bytes[0..8], b"ADEAD.BI");
    }
}
