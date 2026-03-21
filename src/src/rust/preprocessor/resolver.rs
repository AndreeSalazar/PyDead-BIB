// ============================================================
// Header Resolver — Automatic header resolution sin CMake
// ============================================================
// Busca header_main.h, resuelve includes automaticamente.
// FastOS v7.1: kernel.h, fastos.h, types.h se sirven desde
// las librerias internas fastos_kernel, fastos_io, fastos_asm.
// Sin CMake. Sin Makefile. Sin flags raros.
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::stdlib::header_main::HeaderMain;

/// Resuelve headers automaticamente sin CMake/Makefile
pub struct HeaderResolver {
    /// Directorios de busqueda para headers
    search_paths: Vec<PathBuf>,
    /// Cache de headers ya resueltos (path -> contenido)
    resolved_cache: HashMap<String, String>,
    /// Headers ya incluidos (para evitar doble inclusion)
    included: HashMap<String, bool>,
    /// ADead-BIB v7.0 — Central stdlib registry for header_main.h
    header_main: HeaderMain,
}

impl HeaderResolver {
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            resolved_cache: HashMap::new(),
            included: HashMap::new(),
            header_main: HeaderMain::new(),
        }
    }

    /// Agrega directorio de busqueda
    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Configura paths default: directorio del archivo fuente + include/ + ~/.adead/include/
    pub fn setup_default_paths(&mut self, source_file: &Path) {
        // 1. Carpeta actual del archivo fuente
        if let Some(parent) = source_file.parent() {
            self.add_search_path(parent.to_path_buf());
            // 2. Subcarpetas comunes del proyecto (relativo al parent)
            self.add_search_path(parent.join("include"));
            self.add_search_path(parent.join("src"));
            // 3. Subir un nivel para FastOS: kernel/../include/
            if let Some(grandparent) = parent.parent() {
                self.add_search_path(grandparent.join("include"));
                self.add_search_path(grandparent.to_path_buf());
                // Soporta FastOS/kernel/../include/ y FastOS/lib/../include/
                if let Some(root) = grandparent.parent() {
                    self.add_search_path(root.join("include"));
                }
            }
        }
        // 4. ~/.adead/include/ (global headers)
        if let Some(home) = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
        {
            let global_include = PathBuf::from(home).join(".adead").join("include");
            self.add_search_path(global_include);
        }
    }

    /// Resuelve un #include — primero headers FastOS built-in, luego filesystem.
    /// Soporta paths relativos: "../include/kernel.h" → extrae "kernel.h".
    pub fn resolve(&mut self, header_name: &str) -> Result<String, ResolverError> {
        // Extraer solo el basename para headers relativos con '..':
        //   "../include/kernel.h" → "kernel.h"
        //   "./types.h"           → "types.h"
        //   "kernel.h"            → "kernel.h"
        let basename = Path::new(header_name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(header_name);

        // Skip si ya fue incluido (include guard automatico)
        if self.included.contains_key(basename) || self.included.contains_key(header_name) {
            return Ok(String::new());
        }

        // header_main.h es especial — carga todo fastos
        if basename == "header_main.h" {
            self.included.insert(basename.to_string(), true);
            return Ok(self.generate_header_main());
        }

        // ── FastOS Kernel built-in headers (v7.1) ──
        // Se sirven desde las librerias internas en vez del filesystem.
        // Esto elimina el mensaje "unknown header" para todo el kernel FastOS.
        let fastos_builtin = match basename {
            // API completa del kernel
            "kernel.h" | "fastos.h" | "bg_guardian.h" | "bg_hash.h" => {
                Some(crate::stdlib::c::fastos_kernel::generate_kernel_h())
            }
            // I/O de bajo nivel x86-64
            "fastos_io.h" | "io.h" | "ports.h" => {
                Some(crate::stdlib::c::fastos_io::generate_io_h())
            }
            // Compatibilidad de __builtin_* y __attribute__
            "fastos_asm.h" | "asm_compat.h" | "compiler.h" => {
                Some(crate::stdlib::c::fastos_asm::generate_asm_compat_h())
            }
            // Tipos basicos del kernel (sin redefinir los de stdint.h)
            "types.h" if !self.included.contains_key("stdint.h") => {
                // types.h del kernel = stdint.h + bool + NULL
                Some(concat!(
                    "/* types.h (FastOS) — ADead-BIB internal */\n",
                    "typedef unsigned char      uint8_t;\n",
                    "typedef unsigned short     uint16_t;\n",
                    "typedef unsigned int       uint32_t;\n",
                    "typedef unsigned long long uint64_t;\n",
                    "typedef signed char        int8_t;\n",
                    "typedef short              int16_t;\n",
                    "typedef int                int32_t;\n",
                    "typedef long long          int64_t;\n",
                    "typedef unsigned long long size_t;\n",
                    "typedef long long          ssize_t;\n",
                    "typedef unsigned long long uintptr_t;\n",
                    "typedef long long          intptr_t;\n",
                    "#ifndef NULL\n#define NULL ((void*)0)\n#endif\n",
                    "#ifndef bool\ntypedef _Bool bool;\n",
                    "#define true  1\n#define false 0\n#endif\n"
                ).to_string())
            }
            _ => None,
        };

        if let Some(content) = fastos_builtin {
            self.included.insert(basename.to_string(), true);
            self.included.insert(header_name.to_string(), true);
            self.resolved_cache.insert(basename.to_string(), content.clone());
            return Ok(content);
        }

        // Buscar en cache (por basename y por path completo)
        if let Some(content) = self.resolved_cache.get(basename) {
            return Ok(content.clone());
        }
        if let Some(content) = self.resolved_cache.get(header_name) {
            return Ok(content.clone());
        }

        // Buscar en filesystem — intenta el path relativo completo primero,
        // luego solo el basename en cada search_path.
        let search_names = if header_name != basename {
            vec![header_name.to_string(), basename.to_string()]
        } else {
            vec![basename.to_string()]
        };

        for name_to_try in &search_names {
            for search_path in &self.search_paths {
                let full_path = search_path.join(name_to_try);
                if full_path.exists() {
                    match std::fs::read_to_string(&full_path) {
                        Ok(content) => {
                            self.included.insert(basename.to_string(), true);
                            self.included.insert(header_name.to_string(), true);
                            self.resolved_cache
                                .insert(basename.to_string(), content.clone());
                            return Ok(content);
                        }
                        Err(_) => continue,
                    }
                }
            }
        }

        Err(ResolverError::HeaderNotFound(header_name.to_string()))
    }

    /// Genera el contenido de header_main.h — todo FastOS en 1 linea
    /// ADead-BIB v7.0: delegates to c_stdlib::get_header("header_main.h")
    /// which returns HEADER_MAIN_COMPLETE with ALL C99 declarations.
    fn generate_header_main(&self) -> String {
        // Use the real header_main.h from c_stdlib which contains
        // ALL C99 standard library declarations
        if let Some(content) = crate::frontend::c::c_stdlib::get_header("header_main.h") {
            return content.to_string();
        }
        // Fallback (should never happen)
        String::from("// header_main.h — ADead-BIB v7.0\n")
    }

    /// ADead-BIB v7.0: Check if a symbol is a known stdlib symbol.
    /// Used for tree-shaking: mark only used symbols.
    pub fn is_stdlib_symbol(&self, name: &str) -> bool {
        self.header_main.is_known_symbol(name)
    }

    /// ADead-BIB v7.0: Mark a symbol as used for tree shaking.
    pub fn mark_symbol_used(&mut self, name: &str) {
        self.header_main.mark_used(name);
    }

    /// ADead-BIB v7.0: Resolve #include to internal stdlib origin.
    /// Returns the fastos module name if the header is known.
    pub fn resolve_to_stdlib(&self, header_name: &str) -> Option<String> {
        self.header_main
            .resolve_include(header_name)
            .map(|origin| format!("{:?}", origin))
    }

    /// Retorna true si un header ya fue incluido
    pub fn is_included(&self, header_name: &str) -> bool {
        self.included.contains_key(header_name)
    }

    /// Retorna cuantos headers fueron resueltos
    pub fn resolved_count(&self) -> usize {
        self.included.len()
    }
}

impl Default for HeaderResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum ResolverError {
    HeaderNotFound(String),
    ReadError(String),
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ResolverError::HeaderNotFound(h) => write!(f, "Header not found: '{}'", h),
            ResolverError::ReadError(e) => write!(f, "Read error: {}", e),
        }
    }
}

impl std::error::Error for ResolverError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = HeaderResolver::new();
        assert_eq!(resolver.resolved_count(), 0);
    }

    #[test]
    fn test_header_main_resolution() {
        let mut resolver = HeaderResolver::new();
        let result = resolver.resolve("header_main.h");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("header_main.h"));
    }

    #[test]
    fn test_double_include_guard() {
        let mut resolver = HeaderResolver::new();
        let _ = resolver.resolve("header_main.h");
        let second = resolver.resolve("header_main.h");
        assert!(second.is_ok());
        assert!(second.unwrap().is_empty()); // No duplicado
    }
}
