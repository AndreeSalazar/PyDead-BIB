// ============================================================
// ADead-BIB — C++ Name Mangler (Itanium ABI + MSVC)
// ============================================================
// Implements C++ name mangling for two major ABIs:
//
//   Itanium ABI  (GCC / Clang / Linux / macOS)
//     prefix:  _Z
//     reference: https://itanium-cxx-abi.github.io/cxx-abi/abi.html
//
//   MSVC ABI     (Visual C++ / Windows)
//     prefix:  ?
//     reference: https://docs.microsoft.com/cpp/build/reference/decorated-names
//
// Both mangling styles are needed because:
//   • On Windows, ADead-BIB targets Win64 ABI → MSVC style names
//   • On Linux, ADead-BIB targets SysV ABI    → Itanium style names
//   • cross-compilation may need either
// ============================================================

/// Selects which ABI's mangling scheme to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManglingStyle {
    /// Itanium ABI: GCC, Clang, Linux, macOS — prefix `_Z`.
    Itanium,
    /// MSVC ABI: Visual C++, Windows — prefix `?`.
    Msvc,
}

impl ManglingStyle {
    /// Choose the style based on the target operating system.
    pub fn for_target(os: &str) -> Self {
        match os.to_lowercase().as_str() {
            "windows" | "win32" | "win64" => Self::Msvc,
            _ => Self::Itanium,
        }
    }

    /// Auto-detect from the compile-time target.
    pub fn auto_detect() -> Self {
        if cfg!(target_os = "windows") {
            Self::Msvc
        } else {
            Self::Itanium
        }
    }
}

// ── Itanium ABI type encoding ───────────────────────────────────────────────

/// Map a C/C++ base type name to its Itanium ABI one-letter encoding.
fn itanium_builtin_type(ty: &str) -> Option<&'static str> {
    match ty.trim() {
        "void" => Some("v"),
        "bool" => Some("b"),
        "char" => Some("c"),
        "signed char" => Some("a"),
        "unsigned char" => Some("h"),
        "short" | "short int" | "signed short" => Some("s"),
        "unsigned short" => Some("t"),
        "int" | "signed int" | "signed" => Some("i"),
        "unsigned int" | "unsigned" => Some("j"),
        "long" | "signed long" | "long int" => Some("l"),
        "unsigned long" | "unsigned long int" => Some("m"),
        "long long" | "long long int" | "__int64" => Some("x"),
        "unsigned long long" | "unsigned long long int" | "unsigned __int64" => Some("y"),
        "__int128" => Some("n"),
        "unsigned __int128" => Some("o"),
        "float" => Some("f"),
        "double" => Some("d"),
        "long double" => Some("e"),
        "float128" | "__float128" => Some("g"),
        "wchar_t" => Some("w"),
        "char8_t" => Some("Du"),
        "char16_t" => Some("Ds"),
        "char32_t" => Some("Di"),
        "nullptr_t" => Some("Dn"),
        "..." => Some("z"), // variadic
        _ => None,
    }
}

/// Mangle a single type in Itanium format.
fn mangle_type_itanium(ty: &str) -> String {
    let ty = ty.trim();

    // const pointer
    if ty.ends_with(" const *") || ty.ends_with("const *") {
        let inner = ty.trim_end_matches(" const *").trim_end_matches("const *");
        return format!("PK{}", mangle_type_itanium(inner));
    }
    // pointer
    if ty.ends_with(" *") || ty.ends_with('*') {
        let inner = ty.trim_end_matches('*').trim_end_matches(' ');
        return format!("P{}", mangle_type_itanium(inner));
    }
    // const ref
    if ty.ends_with(" const &") {
        let inner = ty.trim_end_matches(" const &");
        return format!("RK{}", mangle_type_itanium(inner));
    }
    // ref
    if ty.ends_with(" &") || ty.ends_with('&') {
        let inner = ty.trim_end_matches('&').trim_end_matches(' ');
        return format!("R{}", mangle_type_itanium(inner));
    }
    // const non-pointer
    if ty.starts_with("const ") {
        let inner = ty.trim_start_matches("const ");
        return format!("K{}", mangle_type_itanium(inner));
    }

    // builtin
    if let Some(code) = itanium_builtin_type(ty) {
        return code.to_string();
    }

    // user-defined type: length-prefixed name   e.g. "MyType" → "6MyType"
    let name = ty
        .trim_start_matches("struct ")
        .trim_start_matches("class ")
        .trim_start_matches("enum ");
    format!("{}{}", name.len(), name)
}

// ── MSVC type encoding ──────────────────────────────────────────────────────

/// Map a C/C++ base type to its MSVC mangled type code.
fn msvc_type_code(ty: &str) -> String {
    match ty.trim() {
        "void" => "X".to_string(),
        "bool" => "_N".to_string(),
        "char" => "D".to_string(),
        "signed char" => "C".to_string(),
        "unsigned char" => "E".to_string(),
        "short" => "F".to_string(),
        "unsigned short" => "G".to_string(),
        "int" => "H".to_string(),
        "unsigned int" => "I".to_string(),
        "long" => "J".to_string(),
        "unsigned long" => "K".to_string(),
        "long long" | "__int64" => "_J".to_string(),
        "unsigned long long" | "unsigned __int64" => "_K".to_string(),
        "__int128" => "_L".to_string(),
        "float" => "M".to_string(),
        "double" => "N".to_string(),
        "long double" => "O".to_string(),
        "wchar_t" => "_W".to_string(),
        "char8_t" => "_Q".to_string(),
        "char16_t" => "_S".to_string(),
        "char32_t" => "_U".to_string(),
        ty if ty.ends_with('*') => {
            let inner = ty.trim_end_matches('*').trim_end_matches(' ');
            format!("PAX{}", msvc_type_code(inner)) // simplified
        }
        other => {
            let name = other
                .trim_start_matches("struct ")
                .trim_start_matches("class ")
                .trim_start_matches("enum ");
            format!("V{}@@", name)
        }
    }
}

// ── Name Mangler ─────────────────────────────────────────────────────────────

/// Contextual information needed to mangle a C++ name.
#[derive(Debug, Clone, Default)]
pub struct ManglerContext {
    /// Namespace path, e.g. `["std", "chrono"]`.
    pub namespaces: Vec<String>,
    /// Class name for member functions.
    pub class_name: Option<String>,
    /// Parameter types, e.g. `["int", "const char *"]`.
    pub params: Vec<String>,
    /// Return type (for non-constructors/destructors).
    pub return_type: Option<String>,
    /// Whether this is a constructor.
    pub is_ctor: bool,
    /// Whether this is a destructor.
    pub is_dtor: bool,
    /// Whether the function is `const` (member functions).
    pub is_const: bool,
    /// Whether it's a `static` member function.
    pub is_static: bool,
    /// Whether it's a `virtual` member function.
    pub is_virtual: bool,
}

/// C++ name mangler supporting both Itanium and MSVC ABIs.
#[derive(Debug, Default)]
pub struct NameMangler {
    pub style: Option<ManglingStyle>,
}

impl NameMangler {
    /// Create a mangler using the auto-detected platform ABI.
    pub fn new() -> Self {
        Self {
            style: Some(ManglingStyle::auto_detect()),
        }
    }

    /// Create a mangler for a specific ABI.
    pub fn with_style(style: ManglingStyle) -> Self {
        Self { style: Some(style) }
    }

    /// Mangle a simple C function name (no parameters, no namespace).
    ///
    /// For C functions the name is typically unchanged (both ABIs keep it).
    pub fn mangle_c(&self, name: &str) -> String {
        name.to_string()
    }

    /// Mangle a C++ function with the given context.
    pub fn mangle_cpp(&self, name: &str, ctx: &ManglerContext) -> String {
        match self.style.unwrap_or(ManglingStyle::auto_detect()) {
            ManglingStyle::Itanium => self.mangle_itanium(name, ctx),
            ManglingStyle::Msvc => self.mangle_msvc(name, ctx),
        }
    }

    // ── Itanium mangling ──────────────────────────────────────────────────

    fn mangle_itanium(&self, name: &str, ctx: &ManglerContext) -> String {
        let mut out = String::from("_Z");

        // Nested name?
        let has_ns = !ctx.namespaces.is_empty();
        let has_class = ctx.class_name.is_some();
        let nested = has_ns || has_class;

        if nested {
            out.push('N');
            // const member qualifier
            if ctx.is_const {
                out.push('K');
            }
            for ns in &ctx.namespaces {
                out.push_str(&format!("{}{}", ns.len(), ns));
            }
            if let Some(cls) = &ctx.class_name {
                out.push_str(&format!("{}{}", cls.len(), cls));
            }
        }

        if ctx.is_ctor {
            // C1 = complete constructor
            out.push_str("C1");
        } else if ctx.is_dtor {
            // D1 = complete destructor
            out.push_str("D1");
        } else {
            if nested {
                out.push_str(&format!("{}{}", name.len(), name));
            } else {
                out.push_str(&format!("{}{}", name.len(), name));
            }
        }

        if nested {
            out.push('E'); // end nested name
        }

        // Parameter types
        if ctx.params.is_empty() {
            out.push('v'); // void params
        } else {
            for p in &ctx.params {
                out.push_str(&mangle_type_itanium(p));
            }
        }

        out
    }

    // ── MSVC mangling ─────────────────────────────────────────────────────

    fn mangle_msvc(&self, name: &str, ctx: &ManglerContext) -> String {
        // MSVC format: ?name@class@ns@@qualifier_return_params@Z
        let mut out = String::from("?");
        out.push_str(name);
        out.push('@');

        // Scope (class, namespaces)
        if let Some(cls) = &ctx.class_name {
            out.push_str(cls);
            out.push('@');
        }
        for ns in ctx.namespaces.iter().rev() {
            out.push_str(ns);
            out.push('@');
        }
        out.push('@');

        // Member/access qualifier
        if ctx.class_name.is_some() {
            if ctx.is_static {
                // static member: SA (public static)
                if ctx.is_const {
                    out.push_str("SA");
                } else {
                    out.push_str("SA");
                }
            } else {
                // non-static members: Q=public, R=protected, S=private
                if ctx.is_const {
                    out.push_str("QB");
                } else {
                    out.push_str("QA");
                }
            }
        } else {
            // free function
            out.push_str("YA");
        }

        // Return type
        let ret = ctx.return_type.as_deref().unwrap_or("void");
        out.push_str(&msvc_type_code(ret));

        // Parameter types
        if ctx.params.is_empty() {
            out.push('X'); // void
        } else {
            for p in &ctx.params {
                out.push_str(&msvc_type_code(p));
            }
            out.push('@');
        }
        out.push('Z');

        out
    }

    // ── Demangling (best effort) ──────────────────────────────────────────

    /// Try to demangle a mangled name.  Returns `None` if unrecognised.
    pub fn demangle(&self, mangled: &str) -> Option<String> {
        if mangled.starts_with("_Z") {
            // Itanium — strip prefix and return a simplified readable form
            Some(format!("<itanium> {}", &mangled[2..]))
        } else if mangled.starts_with('?') {
            // MSVC — extract name up to first @
            let name = mangled[1..].split('@').next().unwrap_or(mangled);
            Some(format!("<msvc> {}", name))
        } else {
            // Undecorated (C function)
            Some(mangled.to_string())
        }
    }
}

// ── Convenience functions ────────────────────────────────────────────────────

/// Quickly mangle a free C++ function for the current platform.
pub fn mangle_function(name: &str, params: &[&str]) -> String {
    let ctx = ManglerContext {
        params: params.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    NameMangler::new().mangle_cpp(name, &ctx)
}

/// Quickly mangle a C++ constructor.
pub fn mangle_constructor(class: &str, params: &[&str]) -> String {
    let ctx = ManglerContext {
        class_name: Some(class.to_string()),
        params: params.iter().map(|s| s.to_string()).collect(),
        is_ctor: true,
        ..Default::default()
    };
    NameMangler::new().mangle_cpp(class, &ctx)
}

/// Quickly mangle a C++ destructor.
pub fn mangle_destructor(class: &str) -> String {
    let ctx = ManglerContext {
        class_name: Some(class.to_string()),
        is_dtor: true,
        ..Default::default()
    };
    NameMangler::new().mangle_cpp(class, &ctx)
}

/// Mangle a standard-library function (std namespace).
pub fn mangle_std_function(name: &str, params: &[&str]) -> String {
    let ctx = ManglerContext {
        namespaces: vec!["std".to_string()],
        params: params.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };
    NameMangler::new().mangle_cpp(name, &ctx)
}

/// Mangle a C++ method for the current platform.
pub fn mangle_method(class: &str, name: &str, params: &[&str], is_const: bool) -> String {
    let ctx = ManglerContext {
        class_name: Some(class.to_string()),
        params: params.iter().map(|s| s.to_string()).collect(),
        is_const,
        ..Default::default()
    };
    NameMangler::new().mangle_cpp(name, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn itanium() -> NameMangler {
        NameMangler::with_style(ManglingStyle::Itanium)
    }

    #[test]
    fn test_free_function_void() {
        // void foo() → _Z3foov
        let ctx = ManglerContext { ..Default::default() };
        assert_eq!(itanium().mangle_cpp("foo", &ctx), "_Z3foov");
    }

    #[test]
    fn test_free_function_int() {
        // void foo(int) → _Z3fooi
        let ctx = ManglerContext {
            params: vec!["int".to_string()],
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("foo", &ctx), "_Z3fooi");
    }

    #[test]
    fn test_free_function_int_double() {
        // void foo(int, double) → _Z3fooid
        let ctx = ManglerContext {
            params: vec!["int".to_string(), "double".to_string()],
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("foo", &ctx), "_Z3fooid");
    }

    #[test]
    fn test_namespace_function() {
        // Math::add(int, int) → _ZN4Math3addEii
        let ctx = ManglerContext {
            namespaces: vec!["Math".to_string()],
            params: vec!["int".to_string(), "int".to_string()],
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("add", &ctx), "_ZN4Math3addEii");
    }

    #[test]
    fn test_method() {
        // Foo::bar() → _ZN3Foo3barEv
        let ctx = ManglerContext {
            class_name: Some("Foo".to_string()),
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("bar", &ctx), "_ZN3Foo3barEv");
    }

    #[test]
    fn test_method_int() {
        // Foo::bar(int) → _ZN3Foo3barEi
        let ctx = ManglerContext {
            class_name: Some("Foo".to_string()),
            params: vec!["int".to_string()],
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("bar", &ctx), "_ZN3Foo3barEi");
    }

    #[test]
    fn test_constructor() {
        // Foo::Foo() → _ZN3FooC1Ev
        let ctx = ManglerContext {
            class_name: Some("Foo".to_string()),
            is_ctor: true,
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("Foo", &ctx), "_ZN3FooC1Ev");
    }

    #[test]
    fn test_destructor() {
        // Foo::~Foo() → _ZN3FooD1Ev
        let ctx = ManglerContext {
            class_name: Some("Foo".to_string()),
            is_dtor: true,
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("Foo", &ctx), "_ZN3FooD1Ev");
    }

    #[test]
    fn test_const_method() {
        // Foo::get() const → _ZNK3Foo3getEv
        let ctx = ManglerContext {
            class_name: Some("Foo".to_string()),
            is_const: true,
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("get", &ctx), "_ZNK3Foo3getEv");
    }

    #[test]
    fn test_pointer_param() {
        // void foo(int*) → _Z3fooPi
        let ctx = ManglerContext {
            params: vec!["int *".to_string()],
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("foo", &ctx), "_Z3fooPi");
    }

    #[test]
    fn test_ref_param() {
        // void foo(int&) → _Z3fooRi
        let ctx = ManglerContext {
            params: vec!["int &".to_string()],
            ..Default::default()
        };
        assert_eq!(itanium().mangle_cpp("foo", &ctx), "_Z3fooRi");
    }

    #[test]
    fn test_std_namespace() {
        // std::sort(int*, int*) — Itanium
        let mangler = itanium();
        let ctx = ManglerContext {
            namespaces: vec!["std".to_string()],
            params: vec!["int *".to_string(), "int *".to_string()],
            ..Default::default()
        };
        let result = mangler.mangle_cpp("sort", &ctx);
        assert!(result.starts_with("_ZN"), "Should start with _ZN: {}", result);
        assert!(result.contains("3std"), "Should contain 3std: {}", result);
        assert!(result.contains("4sort"), "Should contain 4sort: {}", result);
    }

    #[test]
    fn test_itanium_mangle_function() {
        let ctx = ManglerContext {
            params: vec!["int".to_string(), "int".to_string()],
            ..Default::default()
        };
        let result = itanium().mangle_cpp("add", &ctx);
        assert_eq!(result, "_Z3addii");
    }

    #[test]
    fn test_itanium_mangle_constructor() {
        let ctx = ManglerContext {
            class_name: Some("Vector3".to_string()),
            params: vec!["float".to_string(), "float".to_string(), "float".to_string()],
            is_ctor: true,
            ..Default::default()
        };
        let result = itanium().mangle_cpp("Vector3", &ctx);
        assert!(result.contains("C1"), "Should contain C1 for ctor: {}", result);
        assert!(result.contains("7Vector3"), "Should contain 7Vector3: {}", result);
    }

    #[test]
    fn test_itanium_mangle_destructor() {
        let ctx = ManglerContext {
            class_name: Some("Vector3".to_string()),
            is_dtor: true,
            ..Default::default()
        };
        let result = itanium().mangle_cpp("Vector3", &ctx);
        assert!(result.contains("D1"), "Should contain D1 for dtor: {}", result);
        assert!(result.contains("7Vector3"), "Should contain 7Vector3: {}", result);
    }

    #[test]
    fn test_msvc_free_function() {
        let mangler = NameMangler::with_style(ManglingStyle::Msvc);
        let ctx = ManglerContext {
            params: vec!["int".to_string()],
            ..Default::default()
        };
        let result = mangler.mangle_cpp("foo", &ctx);
        assert!(result.starts_with("?foo@"), "MSVC should start with ?foo@: {}", result);
    }

    #[test]
    fn test_demangle_itanium() {
        let mangler = NameMangler::new();
        let d = mangler.demangle("_ZN3Foo3barEv");
        assert!(d.is_some());
    }

    #[test]
    fn test_demangle_msvc() {
        let mangler = NameMangler::new();
        let d = mangler.demangle("?foo@@YAHH@Z");
        assert!(d.is_some());
    }

    #[test]
    fn test_demangle_c() {
        let mangler = NameMangler::new();
        let d = mangler.demangle("printf");
        assert_eq!(d, Some("printf".to_string()));
    }
}
