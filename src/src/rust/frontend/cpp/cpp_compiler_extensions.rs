// ============================================================
// ADead-BIB — C++ Compiler Extensions (GCC + MSVC + Standard C++17/20)
// ============================================================
// Handles compiler-specific attributes and extensions that appear in
// modern C++ code:
//
//   Standard C++17/20 attributes  [[nodiscard]], [[likely]], [[unlikely]],
//                                  [[noreturn]], [[deprecated]], [[fallthrough]]
//
//   GCC / Clang attributes        [[gnu::pure]], [[gnu::noinline]],
//                                  [[clang::noinline]], [[clang::vectorize]]
//
//   MSVC attributes               [[msvc::forceinline]], [[msvc::noinline]],
//                                  __declspec(dllexport), __assume
//
//   GNU C++ extensions            __typeof__, __extension__, __builtin_*,
//                                  Designated initialisers (GCC ext for C++)
//
// The C++ parser calls into this module to resolve unknown attributes.
// ============================================================

// ── Standard C++17/20 attributes ────────────────────────────────────────────

/// All standard C++ attributes defined in the C++11–C++23 standards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandardCppAttr {
    // C++11
    /// `[[noreturn]]` — function never returns.
    NoReturn,
    /// `[[carries_dependency]]` — for `memory_order_consume` dependency chains.
    CarriesDependency,

    // C++14
    /// `[[deprecated]]` / `[[deprecated("msg")]]`
    Deprecated(Option<String>),

    // C++17
    /// `[[nodiscard]]` / `[[nodiscard("msg")]]` — warn if return value is discarded.
    NoDiscard(Option<String>),
    /// `[[maybe_unused]]` — suppress unused warnings.
    MaybeUnused,
    /// `[[fallthrough]]` — intentional switch fall-through.
    Fallthrough,

    // C++20
    /// `[[likely]]` — branch likely to be taken.
    Likely,
    /// `[[unlikely]]` — branch unlikely to be taken.
    Unlikely,
    /// `[[no_unique_address]]` — empty subobject needs no storage.
    NoUniqueAddress,
    /// `[[optimize_for_synchronized]]`
    OptimizeForSynchronized,

    // C++23
    /// `[[assume(expr)]]` — assert expr is true (undefined if not).
    Assume(String),
}

impl StandardCppAttr {
    /// Parse a standard C++ attribute from the `[[X]]` content.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text == "noreturn" {
            return Some(Self::NoReturn);
        }
        if text == "carries_dependency" {
            return Some(Self::CarriesDependency);
        }
        if text == "maybe_unused" {
            return Some(Self::MaybeUnused);
        }
        if text == "fallthrough" {
            return Some(Self::Fallthrough);
        }
        if text == "likely" {
            return Some(Self::Likely);
        }
        if text == "unlikely" {
            return Some(Self::Unlikely);
        }
        if text == "no_unique_address" {
            return Some(Self::NoUniqueAddress);
        }
        if text.starts_with("deprecated") {
            let msg = text
                .trim_start_matches("deprecated")
                .trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim_matches('"');
            return Some(Self::Deprecated(if msg.is_empty() {
                None
            } else {
                Some(msg.to_string())
            }));
        }
        if text.starts_with("nodiscard") {
            let msg = text
                .trim_start_matches("nodiscard")
                .trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim_matches('"');
            return Some(Self::NoDiscard(if msg.is_empty() {
                None
            } else {
                Some(msg.to_string())
            }));
        }
        if text.starts_with("assume(") {
            let expr = text.trim_start_matches("assume(").trim_end_matches(')');
            return Some(Self::Assume(expr.to_string()));
        }
        None
    }

    /// Whether this attribute signals the function never returns.
    pub fn implies_no_return(&self) -> bool {
        matches!(self, Self::NoReturn)
    }

    /// Whether this attribute triggers a compiler warning when result is unused.
    pub fn warns_unused_result(&self) -> bool {
        matches!(self, Self::NoDiscard(_))
    }
}

// ── GCC / Clang namespace attributes ────────────────────────────────────────

/// Attributes from the `[[gnu::X]]` or `[[clang::X]]` namespaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorCppAttr {
    // Optimisation
    AlwaysInline,
    NoInline,
    Cold,
    Hot,
    Pure,
    Const,
    Flatten,

    // Memory
    Malloc,
    NoAlias,
    Aligned(u64),
    Packed,

    // Visibility
    Visibility(String),
    Weak,

    // Sanitiser hints
    NoSanitizeAddress,
    NoSanitizeThread,
    NoSanitizeMemory,

    // Clang-specific
    /// `[[clang::vectorize(enable)]]`
    Vectorize,
    /// `[[clang::loop_unroll]]`
    LoopUnroll,
    /// `[[clang::noescape]]` — pointer argument not captured.
    NoEscape,
    /// `[[clang::annotate("tag")]]`
    Annotate(String),
    /// `[[clang::diagnose_if(cond, msg, type)]]`
    DiagnoseIf,

    // MSVC-specific (in [[msvc::]] namespace)
    /// `[[msvc::forceinline]]`
    MsvcForceInline,
    /// `[[msvc::noinline]]`
    MsvcNoInline,
    /// `[[msvc::intrinsic]]`
    MsvcIntrinsic,

    Unknown(String),
}

impl VendorCppAttr {
    /// Parse a `[[ns::name]]` attribute given namespace and name.
    pub fn parse(namespace: &str, name: &str) -> Self {
        let ns = namespace.to_lowercase();
        let nm = name.to_lowercase();
        match ns.as_str() {
            "gnu" => match nm.as_str() {
                "always_inline" => Self::AlwaysInline,
                "noinline" => Self::NoInline,
                "cold" => Self::Cold,
                "hot" => Self::Hot,
                "pure" => Self::Pure,
                "const" => Self::Const,
                "flatten" => Self::Flatten,
                "malloc" => Self::Malloc,
                "packed" => Self::Packed,
                "weak" => Self::Weak,
                other => Self::Unknown(format!("gnu::{}", other)),
            },
            "clang" => match nm.as_str() {
                "always_inline" => Self::AlwaysInline,
                "noinline" => Self::NoInline,
                "vectorize" => Self::Vectorize,
                "loop_unroll" => Self::LoopUnroll,
                "noescape" => Self::NoEscape,
                "no_sanitize_address" => Self::NoSanitizeAddress,
                other => Self::Unknown(format!("clang::{}", other)),
            },
            "msvc" => match nm.as_str() {
                "forceinline" => Self::MsvcForceInline,
                "noinline" => Self::MsvcNoInline,
                "intrinsic" => Self::MsvcIntrinsic,
                other => Self::Unknown(format!("msvc::{}", other)),
            },
            _ => Self::Unknown(format!("{}::{}", namespace, name)),
        }
    }

    /// Whether this attribute forces inlining.
    pub fn forces_inlining(&self) -> bool {
        matches!(self, Self::AlwaysInline | Self::MsvcForceInline)
    }

    /// Whether this attribute prevents inlining.
    pub fn blocks_inlining(&self) -> bool {
        matches!(self, Self::NoInline | Self::MsvcNoInline)
    }
}

// ── Attribute token ──────────────────────────────────────────────────────────

/// Unified attribute representation for the C++ parser output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CppAttr {
    /// Standard `[[X]]` attribute (no namespace).
    Standard(StandardCppAttr),
    /// Vendor-prefixed `[[ns::X]]` attribute.
    Vendor(VendorCppAttr),
    /// Raw text stored for attributes the compiler doesn't interpret.
    Raw(String),
}

/// Parse a full `[[…]]` attribute content into a `CppAttr`.
///
/// `text` is the content inside the outer `[[` … `]]`.
pub fn parse_cpp_attribute(text: &str) -> CppAttr {
    let text = text.trim();

    // Check for namespace prefix: "gnu::pure", "clang::noinline", etc.
    if let Some(pos) = text.find("::") {
        let ns = &text[..pos];
        let name = text[pos + 2..]
            .split('(')
            .next()
            .unwrap_or(&text[pos + 2..]);
        return CppAttr::Vendor(VendorCppAttr::parse(ns, name));
    }

    // Standard attribute
    if let Some(std) = StandardCppAttr::parse(text) {
        return CppAttr::Standard(std);
    }

    CppAttr::Raw(text.to_string())
}

/// Test whether a `CppAttr` implies the annotated function never returns.
pub fn attr_implies_no_return(attr: &CppAttr) -> bool {
    match attr {
        CppAttr::Standard(s) => s.implies_no_return(),
        _ => false,
    }
}

/// Test whether a `CppAttr` forces function inlining.
pub fn attr_forces_inline(attr: &CppAttr) -> bool {
    match attr {
        CppAttr::Vendor(v) => v.forces_inlining(),
        _ => false,
    }
}

/// Test whether a `CppAttr` prevents function inlining.
pub fn attr_blocks_inline(attr: &CppAttr) -> bool {
    match attr {
        CppAttr::Vendor(v) => v.blocks_inlining(),
        _ => false,
    }
}
