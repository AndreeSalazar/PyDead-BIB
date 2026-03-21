// ============================================================
// Cache Validator — Verifica si fastos.bib es vigente
// ============================================================
// Flujo:
//   fastos.bib existe?
//     SI: hash(header) == cache.hash? -> CACHE HIT (nanosegundos)
//     NO: CACHE MISS -> recompila -> genera cache
// ============================================================

use super::hasher;
use super::{ADeadCache, CACHE_MAGIC, CACHE_VERSION};

/// Resultado de validacion del cache
#[derive(Debug, PartialEq)]
pub enum CacheStatus {
    /// Cache valido, hash coincide — carga instantanea
    Hit,
    /// Cache existe pero hash no coincide — recompilar
    Stale,
    /// Cache no existe — compilar y generar
    Miss,
    /// Cache corrupto — eliminar y recompilar
    Corrupt,
}

/// Valida si un cache es vigente para los headers actuales
pub fn validate(cache: &ADeadCache, current_hash: u64) -> CacheStatus {
    // Verificar magic bytes
    if cache.magic != CACHE_MAGIC {
        return CacheStatus::Corrupt;
    }

    // Verificar version
    if cache.version != CACHE_VERSION {
        return CacheStatus::Stale;
    }

    // Verificar hash del header
    if cache.hash != current_hash {
        return CacheStatus::Stale;
    }

    CacheStatus::Hit
}

/// Verifica un archivo fastos.bib contra un header
pub fn validate_file(cache_path: &str, header_path: &str) -> CacheStatus {
    // Leer hash actual del header
    let current_hash = match hasher::hash_file(header_path) {
        Ok(h) => h,
        Err(_) => return CacheStatus::Miss,
    };

    // Leer cache
    let cache = match super::deserializer::read_from_file(cache_path) {
        Ok(c) => c,
        Err(_) => return CacheStatus::Miss,
    };

    validate(&cache, current_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let cache = ADeadCache::new(0x12345678);
        assert_eq!(validate(&cache, 0x12345678), CacheStatus::Hit);
    }

    #[test]
    fn test_cache_stale() {
        let cache = ADeadCache::new(0x12345678);
        assert_eq!(validate(&cache, 0xDEADBEEF), CacheStatus::Stale);
    }

    #[test]
    fn test_cache_corrupt() {
        let mut cache = ADeadCache::new(0x12345678);
        cache.magic = [0; 8]; // Corrupt magic
        assert_eq!(validate(&cache, 0x12345678), CacheStatus::Corrupt);
    }
}
