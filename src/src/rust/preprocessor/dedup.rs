// ============================================================
// Symbol Deduplication — Sin simbolos duplicados
// ============================================================
// Mantiene una Symbol Table unica. Si un simbolo ya fue
// declarado, no se repite. Sin LNK2019. Sin duplicados.
// ============================================================

use std::collections::HashMap;

/// Tipo de simbolo en la tabla
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Variable,
    Type,
    Macro,
    Struct,
    Enum,
    Typedef,
}

/// Entrada en la tabla de simbolos
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: SymbolKind,
    pub source_file: String,
    pub defined: bool,
}

/// Symbol Table con deduplicacion automatica
pub struct SymbolDedup {
    /// Tabla de simbolos: nombre -> entrada
    symbols: HashMap<String, SymbolEntry>,
    /// Conteo de duplicados evitados
    dedup_count: usize,
}

impl SymbolDedup {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            dedup_count: 0,
        }
    }

    /// Registra un simbolo. Si ya existe, no duplica (retorna false).
    pub fn register(&mut self, name: &str, kind: SymbolKind, source: &str) -> bool {
        if self.symbols.contains_key(name) {
            self.dedup_count += 1;
            return false; // Ya existe, no duplicar
        }
        self.symbols.insert(
            name.to_string(),
            SymbolEntry {
                name: name.to_string(),
                kind,
                source_file: source.to_string(),
                defined: true,
            },
        );
        true
    }

    /// Busca un simbolo por nombre
    pub fn lookup(&self, name: &str) -> Option<&SymbolEntry> {
        self.symbols.get(name)
    }

    /// Retorna true si el simbolo existe
    pub fn exists(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// Retorna cuantos duplicados fueron evitados
    pub fn dedup_count(&self) -> usize {
        self.dedup_count
    }

    /// Retorna cuantos simbolos unicos hay
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Retorna todos los simbolos no definidos (para detectar errores de linkeo)
    pub fn undefined_symbols(&self) -> Vec<&SymbolEntry> {
        self.symbols.values().filter(|s| !s.defined).collect()
    }
}

impl Default for SymbolDedup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_basic() {
        let mut dedup = SymbolDedup::new();
        assert!(dedup.register("printf", SymbolKind::Function, "stdio.h"));
        assert!(!dedup.register("printf", SymbolKind::Function, "other.h")); // Duplicado
        assert_eq!(dedup.dedup_count(), 1);
        assert_eq!(dedup.symbol_count(), 1);
    }

    #[test]
    fn test_lookup() {
        let mut dedup = SymbolDedup::new();
        dedup.register("main", SymbolKind::Function, "main.c");
        assert!(dedup.lookup("main").is_some());
        assert!(dedup.lookup("nonexistent").is_none());
    }
}
