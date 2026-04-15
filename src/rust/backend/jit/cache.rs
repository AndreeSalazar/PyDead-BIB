use std::collections::HashMap;
use std::sync::Mutex;






// ── MEJORA 3: Thermal Cache ────────────────────────────────
// Cache compiled bytes across runs within same session
// Key = hash of source, Value = pre-patched text + data
pub struct CacheEntry {
    text: Vec<u8>,
    data: Vec<u8>,
    entry_offset: u32,
}

// Simple static cache using Mutex
static THERMAL_CACHE: std::sync::LazyLock<Mutex<HashMap<u64, CacheEntry>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Hash source code for thermal cache key
pub fn hash_source(source: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for b in source.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV prime
    }
    h
}

/// Check thermal cache for pre-compiled bytes
pub fn cache_lookup(hash: u64) -> Option<bool> {
    THERMAL_CACHE.lock().ok().map(|c| c.contains_key(&hash))
}


