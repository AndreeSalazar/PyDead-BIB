// ============================================================
// Cache Hasher — Hash de headers para invalidar cache
// ============================================================
// Si el header cambia, el hash cambia, cache invalido.
// Usa FNV-1a hash (rapido, buena distribucion).
// ============================================================

/// FNV-1a hash de 64 bits — rapido y con buena distribucion
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Calcula hash FNV-1a de bytes
pub fn hash_bytes(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Calcula hash de un archivo header
pub fn hash_file(path: &str) -> Result<u64, std::io::Error> {
    let content = std::fs::read(path)?;
    Ok(hash_bytes(&content))
}

/// Calcula hash combinado de multiples archivos
pub fn hash_files(paths: &[&str]) -> Result<u64, std::io::Error> {
    let mut combined = FNV_OFFSET;
    for path in paths {
        let file_hash = hash_file(path)?;
        combined ^= file_hash;
        combined = combined.wrapping_mul(FNV_PRIME);
    }
    Ok(combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let h1 = hash_bytes(b"hello world");
        let h2 = hash_bytes(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different() {
        let h1 = hash_bytes(b"hello");
        let h2 = hash_bytes(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_empty() {
        let h = hash_bytes(b"");
        assert_eq!(h, FNV_OFFSET); // Empty = offset basis
    }
}
