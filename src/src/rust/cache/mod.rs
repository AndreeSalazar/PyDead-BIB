// ============================================================
// ADead-BIB Cache — fastos.bib System
// ============================================================
// Precompiled cache that eliminates repeated header parsing.
// fastos.bib stores: AST, TypeTable, SymbolTable, UB reports.
// Cache hit = nanosegundos. Cache miss = compile + generate cache.
// ============================================================

pub mod deserializer;
pub mod hasher;
pub mod serializer;
pub mod validator;

use std::collections::HashMap;

/// Magic bytes for fastos.bib: "ADEAD.BI"
pub const CACHE_MAGIC: [u8; 8] = *b"ADEAD.BI";

/// Version del formato de cache
pub const CACHE_VERSION: u32 = 2;

/// Estructura principal del cache — fastos.bib
#[derive(Debug, Clone)]
pub struct ADeadCache {
    /// Magic bytes: "ADEAD.BI"
    pub magic: [u8; 8],
    /// Version del cache format
    pub version: u32,
    /// Timestamp de cuando fue generado (epoch seconds)
    pub timestamp: u64,
    /// Hash del header source — si cambia, cache invalido
    pub hash: u64,
    /// AST serializado (tipos resueltos)
    pub ast_data: Vec<u8>,
    /// Tabla de tipos resueltos
    pub types: TypeTable,
    /// Tabla de simbolos indexados (funciones, variables, macros)
    pub symbols: SymbolTable,
    /// UB reports pre-analizados — UNICO en el mundo
    pub ub_reports: Vec<CachedUBReport>,
    /// Tabla de implementaciones
    pub impls: ImplTable,
}

impl ADeadCache {
    pub fn new(hash: u64) -> Self {
        Self {
            magic: CACHE_MAGIC,
            version: CACHE_VERSION,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            hash,
            ast_data: Vec::new(),
            types: TypeTable::new(),
            symbols: SymbolTable::new(),
            ub_reports: Vec::new(),
            impls: ImplTable::new(),
        }
    }

    /// Verifica si el cache es valido (magic + version correctos)
    pub fn is_valid(&self) -> bool {
        self.magic == CACHE_MAGIC && self.version == CACHE_VERSION
    }
}

/// Tabla de tipos resueltos
#[derive(Debug, Clone)]
pub struct TypeTable {
    pub entries: HashMap<String, CachedType>,
}

impl TypeTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, ty: CachedType) {
        self.entries.insert(name, ty);
    }

    pub fn get(&self, name: &str) -> Option<&CachedType> {
        self.entries.get(name)
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Tipo cacheado
#[derive(Debug, Clone)]
pub struct CachedType {
    pub name: String,
    pub size: usize,
    pub alignment: usize,
    pub kind: CachedTypeKind,
}

#[derive(Debug, Clone)]
pub enum CachedTypeKind {
    Primitive,
    Struct { fields: Vec<(String, String)> },
    Pointer { pointee: String },
    Array { element: String, size: usize },
    Function { params: Vec<String>, ret: String },
}

/// Tabla de simbolos
#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub entries: HashMap<String, CachedSymbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, sym: CachedSymbol) {
        self.entries.insert(name, sym);
    }

    pub fn get(&self, name: &str) -> Option<&CachedSymbol> {
        self.entries.get(name)
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Simbolo cacheado
#[derive(Debug, Clone)]
pub struct CachedSymbol {
    pub name: String,
    pub kind: CachedSymbolKind,
    pub source_file: String,
}

#[derive(Debug, Clone)]
pub enum CachedSymbolKind {
    Function { params: Vec<String>, ret: String },
    Variable { ty: String },
    Macro { value: String },
}

/// UB report cacheado (para headers pre-analizados)
#[derive(Debug, Clone)]
pub struct CachedUBReport {
    pub kind: String,
    pub severity: String,
    pub message: String,
    pub location: String,
}

/// Tabla de implementaciones
#[derive(Debug, Clone)]
pub struct ImplTable {
    pub entries: HashMap<String, ImplEntry>,
}

impl ImplTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, entry: ImplEntry) {
        self.entries.insert(name, entry);
    }
}

impl Default for ImplTable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ImplEntry {
    pub function_name: String,
    pub source_file: String,
    pub offset: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = ADeadCache::new(0x12345678);
        assert!(cache.is_valid());
        assert_eq!(cache.hash, 0x12345678);
        assert_eq!(cache.magic, CACHE_MAGIC);
    }

    #[test]
    fn test_type_table() {
        let mut table = TypeTable::new();
        table.insert(
            "int".to_string(),
            CachedType {
                name: "int".to_string(),
                size: 4,
                alignment: 4,
                kind: CachedTypeKind::Primitive,
            },
        );
        assert!(table.get("int").is_some());
        assert!(table.get("float").is_none());
    }
}
