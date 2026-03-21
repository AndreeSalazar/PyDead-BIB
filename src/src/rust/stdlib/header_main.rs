// ============================================================
// header_main.h — ADead-BIB Universal Header
// ============================================================
// Un solo include. Todo disponible. Sin linker.
//
// #include <header_main.h>
//   → hereda TODAS las fastos_*.h
//   → tree shaking automático
//   → Hello World con este header → 2KB binario
// ============================================================

use std::collections::HashMap;

/// Registry of all symbols provided by header_main.h
/// Maps symbol name → (header_origin, symbol_kind)
pub struct HeaderMain {
    /// All C symbols: function name → implementation info
    pub c_symbols: HashMap<String, SymbolInfo>,
    /// All C++ symbols (only active in C++ mode)
    pub cpp_symbols: HashMap<String, SymbolInfo>,
    /// Symbols actually used by the current translation unit
    pub used_symbols: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub header: HeaderOrigin,
    pub kind: SymbolKind,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeaderOrigin {
    // C99 Standard
    FastosStdio,
    FastosStdlib,
    FastosString,
    FastosMath,
    FastosTime,
    FastosAssert,
    FastosErrno,
    FastosLimits,
    FastosTypes,
    // C++ Standard
    FastosIostream,
    FastosVector,
    FastosStringCpp,
    FastosMap,
    FastosMemory,
    FastosAlgorithm,
    FastosFunctional,
    FastosUtility,
    FastosExceptions,
    // FastOS Kernel (v7.1) — sin GCC, sin libc
    FastosKernel,    // kernel.h, fastos.h — toda la API del kernel
    FastosKernelIo,  // fastos_io.h — I/O x86-64, puertos, registros CPU
    FastosKernelAsm, // built-ins: __builtin_va_list, __attribute__, asm volatile
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Macro,
    Type,
    Constant,
    Variable,
}

impl HeaderMain {
    pub fn new() -> Self {
        let mut hm = HeaderMain {
            c_symbols: HashMap::new(),
            cpp_symbols: HashMap::new(),
            used_symbols: Vec::new(),
        };
        hm.register_c_symbols();
        hm.register_cpp_symbols();
        hm
    }

    /// Mark a symbol as used (for tree shaking)
    pub fn mark_used(&mut self, name: &str) {
        if !self.used_symbols.contains(&name.to_string()) {
            self.used_symbols.push(name.to_string());
        }
    }

    /// Get only the symbols actually used (tree shaking result)
    pub fn used_c_symbols(&self) -> Vec<&SymbolInfo> {
        self.used_symbols
            .iter()
            .filter_map(|name| self.c_symbols.get(name))
            .collect()
    }

    /// Check if a name is a known stdlib symbol
    pub fn is_known_symbol(&self, name: &str) -> bool {
        self.c_symbols.contains_key(name) || self.cpp_symbols.contains_key(name)
    }

    /// Resolve #include to internal implementation
    pub fn resolve_include(&self, header: &str) -> Option<HeaderOrigin> {
        // Normalizar: quitar path prefix ("../include/", "./", "/")
        let name = header
            .rsplit('/')
            .next()
            .unwrap_or(header);

        match name {
            // C99 standard
            "stdio.h"  | "fastos_stdio.h"  => Some(HeaderOrigin::FastosStdio),
            "stdlib.h" | "fastos_stdlib.h" => Some(HeaderOrigin::FastosStdlib),
            "string.h" | "fastos_string.h" => Some(HeaderOrigin::FastosString),
            "math.h"   | "fastos_math.h"   => Some(HeaderOrigin::FastosMath),
            "time.h"   | "fastos_time.h"   => Some(HeaderOrigin::FastosTime),
            "assert.h" | "fastos_assert.h" => Some(HeaderOrigin::FastosAssert),
            "errno.h"  | "fastos_errno.h"  => Some(HeaderOrigin::FastosErrno),
            "limits.h" | "fastos_limits.h" => Some(HeaderOrigin::FastosLimits),
            "stdint.h" | "stddef.h" | "stdbool.h" | "fastos_types.h" =>
                Some(HeaderOrigin::FastosTypes),
            // C++ standard
            "iostream" | "fastos_iostream"  => Some(HeaderOrigin::FastosIostream),
            "vector"   | "fastos_vector"    => Some(HeaderOrigin::FastosVector),
            "string"   | "fastos_string_cpp"=> Some(HeaderOrigin::FastosStringCpp),
            "map"      | "fastos_map"       => Some(HeaderOrigin::FastosMap),
            "memory"   | "fastos_memory"    => Some(HeaderOrigin::FastosMemory),
            "algorithm"| "fastos_algorithm" => Some(HeaderOrigin::FastosAlgorithm),
            "functional"| "fastos_functional"=> Some(HeaderOrigin::FastosFunctional),
            "utility"  | "fastos_utility"   => Some(HeaderOrigin::FastosUtility),
            "exception"| "stdexcept" | "fastos_exception" =>
                Some(HeaderOrigin::FastosExceptions),
            "header_main.h" => Some(HeaderOrigin::FastosStdio),
            // FastOS Kernel (v7.1) — sin GCC
            "kernel.h" | "fastos.h" | "bg_guardian.h" | "bg_hash.h" =>
                Some(HeaderOrigin::FastosKernel),
            "types.h"  | "fastos_types_kernel.h" =>
                Some(HeaderOrigin::FastosTypes),
            "fastos_io.h" | "io.h" | "ports.h" =>
                Some(HeaderOrigin::FastosKernelIo),
            "pci.h"    | "acpi.h" | "usb.h" =>
                Some(HeaderOrigin::FastosKernelIo),
            _ => None,
        }
    }

    fn register_c_symbols(&mut self) {
        let stdio_fns = [
            ("printf", "int printf(const char *fmt, ...)"),
            ("scanf", "int scanf(const char *fmt, ...)"),
            ("fprintf", "int fprintf(FILE *f, const char *fmt, ...)"),
            ("sprintf", "int sprintf(char *buf, const char *fmt, ...)"),
            ("snprintf", "int snprintf(char *buf, size_t n, const char *fmt, ...)"),
            ("fopen", "FILE *fopen(const char *path, const char *mode)"),
            ("fclose", "int fclose(FILE *f)"),
            ("fread", "size_t fread(void *buf, size_t size, size_t n, FILE *f)"),
            ("fwrite", "size_t fwrite(const void *buf, size_t size, size_t n, FILE *f)"),
            ("fgets", "char *fgets(char *buf, int n, FILE *f)"),
            ("fputs", "int fputs(const char *s, FILE *f)"),
            ("sscanf", "int sscanf(const char *s, const char *fmt, ...)"),
            ("perror", "void perror(const char *msg)"),
            ("puts", "int puts(const char *s)"),
            ("putchar", "int putchar(int c)"),
            ("getchar", "int getchar(void)"),
        ];
        for (name, sig) in &stdio_fns {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(),
                header: HeaderOrigin::FastosStdio,
                kind: SymbolKind::Function,
                signature: sig.to_string(),
            });
        }

        let stdlib_fns = [
            ("malloc", "void *malloc(size_t size)"),
            ("calloc", "void *calloc(size_t n, size_t size)"),
            ("realloc", "void *realloc(void *ptr, size_t size)"),
            ("free", "void free(void *ptr)"),
            ("exit", "void exit(int code)"),
            ("abort", "void abort(void)"),
            ("atoi", "int atoi(const char *s)"),
            ("atof", "double atof(const char *s)"),
            ("rand", "int rand(void)"),
            ("srand", "void srand(unsigned int seed)"),
            ("qsort", "void qsort(void *base, size_t n, size_t size, int (*cmp)(const void*, const void*))"),
            ("bsearch", "void *bsearch(const void *key, const void *base, size_t n, size_t size, int (*cmp)(const void*, const void*))"),
            ("abs", "int abs(int x)"),
        ];
        for (name, sig) in &stdlib_fns {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(),
                header: HeaderOrigin::FastosStdlib,
                kind: SymbolKind::Function,
                signature: sig.to_string(),
            });
        }

        let string_fns = [
            ("strlen", "size_t strlen(const char *s)"),
            ("strcpy", "char *strcpy(char *dst, const char *src)"),
            ("strncpy", "char *strncpy(char *dst, const char *src, size_t n)"),
            ("strcat", "char *strcat(char *dst, const char *src)"),
            ("strncat", "char *strncat(char *dst, const char *src, size_t n)"),
            ("strcmp", "int strcmp(const char *a, const char *b)"),
            ("strncmp", "int strncmp(const char *a, const char *b, size_t n)"),
            ("strchr", "char *strchr(const char *s, int c)"),
            ("strrchr", "char *strrchr(const char *s, int c)"),
            ("strstr", "char *strstr(const char *hay, const char *needle)"),
            ("memcpy", "void *memcpy(void *dst, const void *src, size_t n)"),
            ("memmove", "void *memmove(void *dst, const void *src, size_t n)"),
            ("memset", "void *memset(void *ptr, int val, size_t n)"),
            ("memcmp", "int memcmp(const void *a, const void *b, size_t n)"),
            ("strtok", "char *strtok(char *s, const char *delim)"),
            ("strdup", "char *strdup(const char *s)"),
        ];
        for (name, sig) in &string_fns {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(),
                header: HeaderOrigin::FastosString,
                kind: SymbolKind::Function,
                signature: sig.to_string(),
            });
        }

        let math_fns = [
            ("sin", "double sin(double x)"),
            ("cos", "double cos(double x)"),
            ("tan", "double tan(double x)"),
            ("asin", "double asin(double x)"),
            ("acos", "double acos(double x)"),
            ("atan", "double atan(double x)"),
            ("atan2", "double atan2(double y, double x)"),
            ("sqrt", "double sqrt(double x)"),
            ("cbrt", "double cbrt(double x)"),
            ("pow", "double pow(double base, double exp)"),
            ("exp", "double exp(double x)"),
            ("log", "double log(double x)"),
            ("log2", "double log2(double x)"),
            ("log10", "double log10(double x)"),
            ("floor", "double floor(double x)"),
            ("ceil", "double ceil(double x)"),
            ("round", "double round(double x)"),
            ("fabs", "double fabs(double x)"),
            ("fmod", "double fmod(double x, double y)"),
            ("hypot", "double hypot(double x, double y)"),
        ];
        for (name, sig) in &math_fns {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(),
                header: HeaderOrigin::FastosMath,
                kind: SymbolKind::Function,
                signature: sig.to_string(),
            });
        }
        // ── FastOS Kernel symbols (v7.1) — toda la API del kernel sin GCC
        use crate::stdlib::c::fastos_kernel;
        for (name, sig) in fastos_kernel::KERNEL_OUTPUT_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_kernel::KERNEL_MEMORY_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_kernel::KERNEL_SCHEDULER_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_kernel::KERNEL_INTERRUPT_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_kernel::KERNEL_PANIC_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_kernel::KERNEL_BG_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_kernel::KERNEL_HOTPLUG_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_kernel::KERNEL_USERSPACE_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, desc) in fastos_kernel::KERNEL_MACROS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Macro, signature: desc.to_string(),
            });
        }
        for (name, desc) in fastos_kernel::KERNEL_TYPES {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernel,
                kind: SymbolKind::Type, signature: desc.to_string(),
            });
        }
        // ── fastos_io: inb/outb, cli/sti, read_cr3...
        use crate::stdlib::c::fastos_io;
        for (name, sig) in fastos_io::IO_PORT_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernelIo,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_io::CPU_CONTROL_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernelIo,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_io::CPU_REGISTER_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernelIo,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, sig) in fastos_io::CPU_DESCRIPTOR_FNS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernelIo,
                kind: SymbolKind::Function, signature: sig.to_string(),
            });
        }
        for (name, desc) in fastos_io::HW_CONSTANTS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernelIo,
                kind: SymbolKind::Constant, signature: desc.to_string(),
            });
        }
        // ── fastos_asm: __builtin_*, __attribute__, compat macros
        use crate::stdlib::c::fastos_asm;
        for (name, _, desc) in fastos_asm::BUILTINS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernelAsm,
                kind: SymbolKind::Macro, signature: desc.to_string(),
            });
        }
        for (name, desc) in fastos_asm::COMPAT_MACROS {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernelAsm,
                kind: SymbolKind::Macro, signature: desc.to_string(),
            });
        }
        for (name, ty) in fastos_asm::COMPILER_TYPES {
            self.c_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(), header: HeaderOrigin::FastosKernelAsm,
                kind: SymbolKind::Type, signature: ty.to_string(),
            });
        }
    }

    fn register_cpp_symbols(&mut self) {
        // C++ stream objects
        for name in &["cout", "cin", "cerr", "clog", "endl", "flush"] {
            self.cpp_symbols.insert(name.to_string(), SymbolInfo {
                name: name.to_string(),
                header: HeaderOrigin::FastosIostream,
                kind: SymbolKind::Variable,
                signature: format!("std::{}", name),
            });
        }
    }
}

impl Default for HeaderMain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_main_creation() {
        let hm = HeaderMain::new();
        assert!(hm.c_symbols.len() > 50);
        assert!(hm.is_known_symbol("printf"));
        assert!(hm.is_known_symbol("malloc"));
        assert!(hm.is_known_symbol("strlen"));
        assert!(hm.is_known_symbol("sin"));
        assert!(!hm.is_known_symbol("nonexistent_fn"));
    }

    #[test]
    fn test_resolve_include() {
        let hm = HeaderMain::new();
        assert_eq!(hm.resolve_include("stdio.h"), Some(HeaderOrigin::FastosStdio));
        assert_eq!(hm.resolve_include("stdlib.h"), Some(HeaderOrigin::FastosStdlib));
        assert_eq!(hm.resolve_include("vector"), Some(HeaderOrigin::FastosVector));
        assert_eq!(hm.resolve_include("unknown.h"), None);
    }

    #[test]
    fn test_tree_shaking() {
        let mut hm = HeaderMain::new();
        hm.mark_used("printf");
        hm.mark_used("malloc");
        let used = hm.used_c_symbols();
        assert_eq!(used.len(), 2);
    }
}
