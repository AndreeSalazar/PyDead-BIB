// ============================================================
// Python Import Resolver for PyDead-BIB
// ============================================================
// Static import resolution — Sin .pyc intermedios — NUNCA
// Sin site-packages en runtime — NUNCA
// "ModuleNotFoundError" en compile time — no en runtime
// Dead import elimination
// ============================================================

/// Resolved import information
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub module: String,
    pub names: Vec<String>,
    pub is_stdlib: bool,
    pub compile_action: ImportAction,
}

/// What to do with an import at compile time
#[derive(Debug, Clone)]
pub enum ImportAction {
    /// Module maps to native ISA instructions (e.g., math.sqrt → VSQRTSS)
    InlineNative,
    /// Module maps to a syscall (e.g., sys.exit → syscall)
    Syscall,
    /// Module compiles to static code included in binary
    StaticLink,
    /// Third-party pip package — PyDead-BIB compiles natively, no pip needed
    NativeReplacement(String),
    /// Module not found — compile error
    NotFound,
}

/// Static import resolver — all imports resolved at compile time
pub struct PyImportResolver {
    stdlib_modules: Vec<&'static str>,
    pip_packages: Vec<(&'static str, &'static str)>,
}

impl PyImportResolver {
    pub fn new() -> Self {
        Self {
            stdlib_modules: vec![
                // Math → inline ISA
                "math", "cmath",
                // System → syscalls
                "sys", "os", "os.path",
                // I/O → native
                "io", "builtins",
                // Data structures → native types
                "collections", "itertools", "functools",
                // String → .data section
                "string", "re",
                // Types
                "typing", "types", "abc",
                // Numbers
                "decimal", "fractions",
                // File system
                "pathlib", "glob", "shutil",
                // Time → native
                "time", "datetime",
                // JSON/serialization
                "json", "pickle", "struct",
                // Concurrency (sin GIL → real threads)
                "threading", "multiprocessing", "asyncio",
                // Network
                "socket", "http", "urllib",
                // Hashing
                "hashlib", "hmac",
                // Random
                "random",
                // Compression
                "zlib", "gzip", "bz2", "lzma",
                // Error handling
                "traceback", "warnings",
                // Enum
                "enum",
                // Dataclasses
                "dataclasses",
                // Copy
                "copy",
                // Logging
                "logging",
                // Testing
                "unittest",
                // Context
                "contextlib",
                // CSV
                "csv",
                // Subprocess
                "subprocess",
                // Signal
                "signal",
                // Temp files
                "tempfile",
                // Args
                "argparse",
                // Config
                "configparser",
                // UUID
                "uuid",
                // Base64
                "base64",
                // Secrets
                "secrets",
                // sqlite
                "sqlite3",
                // Email
                "email",
                // XML
                "xml",
                // HTML
                "html",
                // Inspect
                "inspect",
                // Operator
                "operator",
                // Textwrap
                "textwrap",
                // Weakref
                "weakref",
                // Array
                "array",
                // Bisect / heapq
                "bisect", "heapq",
                // Queue
                "queue",
                // Statistics
                "statistics",
                // Pprint
                "pprint",
                // Dis
                "dis",
                // Platform
                "platform",
                // Sysconfig
                "sysconfig",
            ],
            pip_packages: vec![
                ("numpy", "SIMD nativo — VMULPS/VADDPS directo — sin numpy C extension"),
                ("requests", "HTTP nativo — syscall directo — sin requests/urllib3 stack"),
                ("flask", "HTTP server nativo — sin WSGI — sin Werkzeug"),
                ("django", "sin Django — usar framework nativo PyDead-BIB"),
                ("pandas", "datos tabulares nativos — sin pandas C extension"),
                ("scipy", "math nativo — VSQRTSS/VFMADD directo"),
                ("matplotlib", "sin matplotlib — output directo a framebuffer"),
                ("pytest", "testing nativo PyDead-BIB — sin pytest runtime"),
                ("pillow", "imagen nativa — sin PIL C extension"),
                ("PIL", "imagen nativa — sin PIL C extension"),
                ("sqlalchemy", "SQL directo — sin ORM runtime"),
                ("fastapi", "HTTP server nativo — sin Starlette/Pydantic stack"),
                ("pydantic", "validación en compile time — sin pydantic runtime"),
                ("cryptography", "crypto nativo — AES-NI instrucciones directas"),
                ("aiohttp", "async HTTP nativo — sin event loop Python"),
                ("celery", "tasks nativos — sin broker Python"),
                ("redis", "redis protocol nativo — sin redis-py"),
                ("boto3", "AWS API nativo — sin botocore stack"),
                ("botocore", "AWS API nativo — sin botocore stack"),
                ("setuptools", "sin necesidad — PyDead-BIB compila directo"),
                ("pip", "sin necesidad — PyDead-BIB compila directo"),
                ("wheel", "sin necesidad — PyDead-BIB compila directo"),
            ],
        }
    }

    /// Resolve imports from source — returns list of import names found
    pub fn resolve(&self, source: &str) -> Vec<String> {
        let mut imports = Vec::new();
        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("import ") {
                let module = trimmed["import ".len()..].trim();
                // Handle "import a, b, c"
                for m in module.split(',') {
                    let name = m.trim().split(" as ").next().unwrap_or("").trim();
                    if !name.is_empty() {
                        imports.push(name.to_string());
                    }
                }
            } else if trimmed.starts_with("from ") && trimmed.contains("import") {
                if let Some(module_part) = trimmed.strip_prefix("from ") {
                    let module = module_part.split("import").next().unwrap_or("").trim();
                    if !module.is_empty() && module != "__future__" {
                        imports.push(module.to_string());
                    }
                }
            }
        }
        imports
    }

    /// Check if a module name is a known pip package
    pub fn is_pip_package(&self, name: &str) -> bool {
        let base = name.split('.').next().unwrap_or(name);
        self.pip_packages.iter().any(|(pkg, _)| *pkg == base)
    }

    /// Get the native replacement message for a pip package
    pub fn pip_replacement_message(&self, name: &str) -> Option<&str> {
        let base = name.split('.').next().unwrap_or(name);
        self.pip_packages.iter()
            .find(|(pkg, _)| *pkg == base)
            .map(|(_, msg)| *msg)
    }

    /// Resolve a module name to a compile action
    pub fn resolve_module(&self, name: &str) -> ResolvedImport {
        let base = name.split('.').next().unwrap_or(name);
        let is_stdlib = self.stdlib_modules.contains(&base);

        let action = if is_stdlib {
            match base {
                "math" | "cmath" => ImportAction::InlineNative,
                "sys" | "os" => ImportAction::Syscall,
                _ => ImportAction::StaticLink,
            }
        } else if let Some(msg) = self.pip_replacement_message(name) {
            ImportAction::NativeReplacement(msg.to_string())
        } else {
            ImportAction::NotFound
        };

        ResolvedImport {
            module: name.to_string(),
            names: Vec::new(),
            is_stdlib,
            compile_action: action,
        }
    }
}

impl std::fmt::Display for ResolvedImport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.module)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_imports() {
        let resolver = PyImportResolver::new();
        let imports = resolver.resolve("import os\nimport math\nfrom sys import exit\n");
        assert_eq!(imports.len(), 3);
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"math".to_string()));
        assert!(imports.contains(&"sys".to_string()));
    }

    #[test]
    fn test_stdlib_detection() {
        let resolver = PyImportResolver::new();
        let resolved = resolver.resolve_module("math");
        assert!(resolved.is_stdlib);
    }

    #[test]
    fn test_pip_package_detection() {
        let resolver = PyImportResolver::new();
        assert!(resolver.is_pip_package("numpy"));
        assert!(resolver.is_pip_package("requests"));
        assert!(resolver.is_pip_package("flask"));
        assert!(resolver.is_pip_package("django"));
        assert!(resolver.is_pip_package("pandas"));
        assert!(resolver.is_pip_package("scipy"));
        assert!(resolver.is_pip_package("matplotlib"));
        assert!(resolver.is_pip_package("pytest"));
        assert!(resolver.is_pip_package("pillow"));
        assert!(resolver.is_pip_package("PIL"));
        assert!(resolver.is_pip_package("sqlalchemy"));
        assert!(resolver.is_pip_package("fastapi"));
        assert!(resolver.is_pip_package("pydantic"));
        assert!(resolver.is_pip_package("cryptography"));
        assert!(resolver.is_pip_package("aiohttp"));
        assert!(resolver.is_pip_package("celery"));
        assert!(resolver.is_pip_package("redis"));
        assert!(resolver.is_pip_package("boto3"));
        assert!(resolver.is_pip_package("setuptools"));
        assert!(resolver.is_pip_package("pip"));
        assert!(resolver.is_pip_package("wheel"));
    }

    #[test]
    fn test_pip_package_not_stdlib() {
        let resolver = PyImportResolver::new();
        assert!(!resolver.is_pip_package("math"));
        assert!(!resolver.is_pip_package("os"));
        assert!(!resolver.is_pip_package("sys"));
    }

    #[test]
    fn test_pip_unknown_package() {
        let resolver = PyImportResolver::new();
        assert!(!resolver.is_pip_package("some_unknown_pkg"));
    }

    #[test]
    fn test_pip_replacement_message() {
        let resolver = PyImportResolver::new();
        assert_eq!(
            resolver.pip_replacement_message("numpy"),
            Some("SIMD nativo — VMULPS/VADDPS directo — sin numpy C extension")
        );
        assert_eq!(
            resolver.pip_replacement_message("requests"),
            Some("HTTP nativo — syscall directo — sin requests/urllib3 stack")
        );
        assert_eq!(resolver.pip_replacement_message("unknown_pkg"), None);
    }

    #[test]
    fn test_pip_submodule_detection() {
        let resolver = PyImportResolver::new();
        assert!(resolver.is_pip_package("numpy.linalg"));
        assert!(resolver.is_pip_package("flask.app"));
        assert!(resolver.is_pip_package("boto3.session"));
    }

    #[test]
    fn test_resolve_module_pip_returns_native_replacement() {
        let resolver = PyImportResolver::new();
        let resolved = resolver.resolve_module("numpy");
        assert!(!resolved.is_stdlib);
        match &resolved.compile_action {
            ImportAction::NativeReplacement(msg) => {
                assert!(msg.contains("SIMD nativo"));
            }
            other => panic!("expected NativeReplacement, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_module_unknown_returns_not_found() {
        let resolver = PyImportResolver::new();
        let resolved = resolver.resolve_module("totally_unknown");
        assert!(!resolved.is_stdlib);
        assert!(matches!(resolved.compile_action, ImportAction::NotFound));
    }

    #[test]
    fn test_new_stdlib_modules() {
        let resolver = PyImportResolver::new();
        for module in &["logging", "unittest", "contextlib", "csv", "subprocess",
                        "signal", "tempfile", "argparse", "configparser", "uuid",
                        "base64", "secrets", "sqlite3", "email", "xml", "html",
                        "inspect", "operator", "textwrap", "weakref", "array",
                        "bisect", "heapq", "queue", "statistics", "pprint",
                        "dis", "platform", "sysconfig"] {
            let resolved = resolver.resolve_module(module);
            assert!(resolved.is_stdlib, "{} should be stdlib", module);
        }
    }
}
