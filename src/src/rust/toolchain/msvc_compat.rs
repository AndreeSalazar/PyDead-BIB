// ============================================================
// ADead-BIB — MSVC Heritage: __declspec, calling conventions, pragmas
// ============================================================
// Captures everything ADead-BIB inherits from MSVC:
//
//   • __declspec(X) extended attributes
//   • Microsoft calling conventions: __cdecl, __stdcall, __fastcall,
//     __vectorcall, __thiscall, __clrcall
//   • MSVC-specific pragmas: #pragma once, #pragma pack, #pragma comment
//   • Microsoft C/C++ language extensions: __int8/16/32/64, __w64, etc.
//
// References:
//   https://docs.microsoft.com/cpp/cpp/declspec
//   https://docs.microsoft.com/cpp/cpp/calling-conventions
//   https://docs.microsoft.com/cpp/preprocessor/pragma-directives-and-the-pragma-keyword
// ============================================================

/// `__declspec(specifier)` extended attributes recognised by ADead-BIB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsvcDeclspec {
    // ── Code-generation ────────────────────────────────────
    /// `__declspec(noinline)` — do not inline.
    NoInline,
    /// `__declspec(forceinline)` — always inline.
    ForceInline,
    /// `__declspec(noreturn)` — function never returns.
    NoReturn,
    /// `__declspec(naked)` — no prologue/epilogue emitted.
    Naked,
    /// `__declspec(nothrow)` — function does not throw.
    NoThrow,

    // ── Memory ─────────────────────────────────────────────
    /// `__declspec(align(N))` — alignment in bytes.
    Align(u64),
    /// `__declspec(restrict)` — return pointer does not alias any existing pointer.
    Restrict,
    /// `__declspec(allocator)` — function returns fresh memory.
    Allocator,

    // ── Thread-local storage ────────────────────────────────
    /// `__declspec(thread)` — thread-local variable.
    Thread,

    // ── DLL import/export ───────────────────────────────────
    /// `__declspec(dllimport)` — import symbol from DLL.
    DllImport,
    /// `__declspec(dllexport)` — export symbol from DLL.
    DllExport,

    // ── COM / COM+ ─────────────────────────────────────────
    /// `__declspec(uuid("…"))` — COM UUID.
    Uuid(String),
    /// `__declspec(novtable)` — COM pure-virtual skip vtable.
    NoVtable,
    /// `__declspec(selectany)` — pick any definition (like weak).
    Selectany,

    // ── Properties ─────────────────────────────────────────
    /// `__declspec(property(get=G, put=P))` — C++/CLI property.
    Property {
        get: Option<String>,
        put: Option<String>,
    },

    // ── SAL annotations (static analysis) ──────────────────
    /// Pointer annotated `_In_`.
    SalIn,
    /// Pointer annotated `_Out_`.
    SalOut,
    /// Pointer annotated `_Inout_`.
    SalInout,
    /// Optional return value.
    SalMaybeNull,

    // ── Deprecated / empty ─────────────────────────────────
    /// `__declspec(deprecated)`.
    Deprecated(Option<String>),
    /// `__declspec(empty_bases)` — empty base-class optimisation.
    EmptyBases,

    /// Unknown specifier stored verbatim.
    Unknown(String),
}

impl MsvcDeclspec {
    /// Parse a `__declspec(X)` specifier.
    ///
    /// `name` is the specifier keyword (lower-cased).
    /// `arg`  is the raw argument string if the specifier takes one.
    pub fn parse(name: &str, arg: Option<&str>) -> Self {
        match name.to_lowercase().as_str() {
            "noinline" => Self::NoInline,
            "forceinline" => Self::ForceInline,
            "noreturn" => Self::NoReturn,
            "naked" => Self::Naked,
            "nothrow" => Self::NoThrow,
            "align" => {
                let n = arg.and_then(|a| a.parse().ok()).unwrap_or(16);
                Self::Align(n)
            }
            "restrict" => Self::Restrict,
            "allocator" => Self::Allocator,
            "thread" => Self::Thread,
            "dllimport" => Self::DllImport,
            "dllexport" => Self::DllExport,
            "uuid" => Self::Uuid(arg.unwrap_or("").trim_matches('"').to_string()),
            "novtable" => Self::NoVtable,
            "selectany" => Self::Selectany,
            "empty_bases" => Self::EmptyBases,
            "deprecated" => Self::Deprecated(arg.map(|s| s.trim_matches('"').to_string())),
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Whether this specifier affects DLL linking.
    pub fn is_dll_linkage(&self) -> bool {
        matches!(self, Self::DllImport | Self::DllExport)
    }

    /// Whether this specifier disables inlining.
    pub fn blocks_inlining(&self) -> bool {
        matches!(self, Self::NoInline | Self::Naked)
    }
}

// ── MSVC Calling Conventions ────────────────────────────────────────────────

/// Calling conventions recognised by MSVC and represented in ADead-BIB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MsvcCallingConv {
    /// `__cdecl` — caller cleans the stack; default C convention.
    Cdecl,
    /// `__stdcall` — callee cleans the stack; used by Win32 API.
    Stdcall,
    /// `__fastcall` — first two arguments in ECX/EDX (x86 only).
    Fastcall,
    /// `__vectorcall` — XMM0–XMM5 for vector arguments.
    Vectorcall,
    /// `__thiscall` — `this` pointer in ECX; default for C++ member functions.
    Thiscall,
    /// `__clrcall` — managed (.NET) calling convention.
    Clrcall,
}

impl MsvcCallingConv {
    /// Parse from keyword string (case-insensitive, with or without underscores).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().trim_start_matches('_') {
            "cdecl" => Some(Self::Cdecl),
            "stdcall" => Some(Self::Stdcall),
            "fastcall" => Some(Self::Fastcall),
            "vectorcall" => Some(Self::Vectorcall),
            "thiscall" => Some(Self::Thiscall),
            "clrcall" => Some(Self::Clrcall),
            _ => None,
        }
    }

    /// Returns `"@N"` name-decoration suffix for x86 __stdcall/__fastcall.
    ///
    /// `param_bytes` is the total byte size of all parameters.
    pub fn decoration_suffix(self, param_bytes: u32) -> String {
        match self {
            Self::Stdcall => format!("@{}", param_bytes),
            Self::Fastcall => format!("@{}", param_bytes),
            _ => String::new(),
        }
    }

    /// Returns the MSVC name-decoration prefix character.
    pub fn decoration_prefix(self) -> &'static str {
        match self {
            Self::Stdcall => "_",
            Self::Fastcall => "@",
            Self::Cdecl => "_",
            _ => "",
        }
    }

    /// Whether the *callee* is responsible for cleaning the stack.
    pub fn callee_cleanup(self) -> bool {
        matches!(
            self,
            Self::Stdcall | Self::Fastcall | Self::Thiscall | Self::Vectorcall
        )
    }
}

// ── MSVC Pragmas ────────────────────────────────────────────────────────────

/// `#pragma` directives recognised by MSVC (and handled in the preprocessor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsvcPragma {
    /// `#pragma once` — include guard.
    Once,
    /// `#pragma pack(push)` — save current pack setting.
    PackPush(Option<u32>),
    /// `#pragma pack(pop)` — restore previous pack setting.
    PackPop,
    /// `#pragma pack(N)` — set pack alignment to N bytes.
    PackSet(u32),
    /// `#pragma comment(lib, "name")` — link library.
    CommentLib(String),
    /// `#pragma comment(linker, "flag")` — pass flag to linker.
    CommentLinker(String),
    /// `#pragma comment(compiler)` — embed compiler version.
    CommentCompiler,
    /// `#pragma comment(user, "text")` — user comment.
    CommentUser(String),
    /// `#pragma section("name", attrs)` — custom section.
    Section { name: String, attrs: Vec<String> },
    /// `#pragma warning(disable: N)` — suppress warning N.
    WarningDisable(Vec<u32>),
    /// `#pragma warning(push)`.
    WarningPush,
    /// `#pragma warning(pop)`.
    WarningPop,
    /// `#pragma warning(error: N)` — promote warning to error.
    WarningError(Vec<u32>),
    /// `#pragma optimize("flags", on/off)`.
    Optimize { flags: String, on: bool },
    /// `#pragma intrinsic(name)` — mark function as intrinsic.
    Intrinsic(String),
    /// `#pragma function(name)` — generate actual call.
    Function(String),
    /// `#pragma vtordisp(push)` / `#pragma vtordisp(...)`.
    Vtordisp,
    /// Unknown pragma stored verbatim.
    Unknown(String),
}

impl MsvcPragma {
    /// Try to parse a pragma directive line (after `#pragma `).
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        if text == "once" {
            return Self::Once;
        }
        if text.starts_with("pack") {
            let inner = text
                .trim_start_matches("pack")
                .trim()
                .trim_matches(|c| c == '(' || c == ')');
            return match inner {
                "push" => Self::PackPush(None),
                "pop" => Self::PackPop,
                s if s.starts_with("push,") => {
                    let n = s.trim_start_matches("push,").trim().parse().ok();
                    Self::PackPush(n)
                }
                s => {
                    let n = s.parse().unwrap_or(8);
                    Self::PackSet(n)
                }
            };
        }
        if text.starts_with("comment") {
            let inner = text
                .trim_start_matches("comment")
                .trim()
                .trim_matches(|c| c == '(' || c == ')');
            let parts: Vec<&str> = inner.splitn(2, ',').collect();
            let kind = parts.first().map(|s| s.trim()).unwrap_or("");
            let arg = parts
                .get(1)
                .map(|s| s.trim().trim_matches('"').to_string())
                .unwrap_or_default();
            return match kind {
                "lib" => Self::CommentLib(arg),
                "linker" => Self::CommentLinker(arg),
                "compiler" => Self::CommentCompiler,
                "user" => Self::CommentUser(arg),
                _ => Self::Unknown(text.to_string()),
            };
        }
        if text.starts_with("warning") {
            return Self::WarningPush; // simplified
        }
        Self::Unknown(text.to_string())
    }
}

// ── MSVC Language Extensions ────────────────────────────────────────────────

/// Microsoft-specific type and language extensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsvcExtension {
    // Integer types
    /// `__int8` — signed 8-bit integer.
    Int8,
    /// `__int16` — signed 16-bit integer.
    Int16,
    /// `__int32` — signed 32-bit integer.
    Int32,
    /// `__int64` — signed 64-bit integer.
    Int64,
    /// `__int128` — signed 128-bit integer (not always available).
    Int128,

    // Pointer helpers
    /// `__ptr32` — force 32-bit pointer.
    Ptr32,
    /// `__ptr64` — force 64-bit pointer (default on x64).
    Ptr64,
    /// `__w64` — indicate pointer is 64-bit-safe (legacy).
    W64,

    // Misc
    /// `__assume(cond)` — assert to the compiler that cond is true.
    Assume,
    /// `__debugbreak()` — emit a software breakpoint (`int3`).
    Debugbreak,
    /// `__noop` — no-op intrinsic.
    Noop,
    /// `__cpuid(cpuInfo, function_id)` — CPUID instruction.
    Cpuid,
    /// `__rdtsc()` — read time-stamp counter.
    Rdtsc,
    /// `__rdtscp(aux)` — RDTSCP instruction.
    Rdtscp,
    /// `wchar_t` as a built-in type (MSVC treats it as a keyword).
    WcharT,
    /// `__cdecl` as a type qualifier.
    CdeclQualifier,
    /// `__volatile` keyword.
    Volatile,
    /// `__restrict` keyword.
    Restrict,
    /// `__unaligned` keyword.
    Unaligned,
    /// Unknown extension.
    Unknown(String),
}

impl MsvcExtension {
    /// Attempt to parse an MSVC extension keyword.
    pub fn parse(kw: &str) -> Option<Self> {
        match kw {
            "__int8" | "_int8" => Some(Self::Int8),
            "__int16" | "_int16" => Some(Self::Int16),
            "__int32" | "_int32" => Some(Self::Int32),
            "__int64" | "_int64" => Some(Self::Int64),
            "__int128" => Some(Self::Int128),
            "__ptr32" => Some(Self::Ptr32),
            "__ptr64" => Some(Self::Ptr64),
            "__w64" => Some(Self::W64),
            "__assume" => Some(Self::Assume),
            "__debugbreak" => Some(Self::Debugbreak),
            "__noop" => Some(Self::Noop),
            "__cpuid" => Some(Self::Cpuid),
            "__rdtsc" => Some(Self::Rdtsc),
            "__rdtscp" => Some(Self::Rdtscp),
            "wchar_t" => Some(Self::WcharT),
            "__volatile" => Some(Self::Volatile),
            "__restrict" => Some(Self::Restrict),
            "__unaligned" => Some(Self::Unaligned),
            _ => None,
        }
    }

    /// Return the equivalent Rust/C type name for integer extensions.
    pub fn c_type_name(&self) -> Option<&'static str> {
        match self {
            Self::Int8 => Some("int8_t"),
            Self::Int16 => Some("int16_t"),
            Self::Int32 => Some("int32_t"),
            Self::Int64 => Some("int64_t"),
            Self::WcharT => Some("unsigned short"),
            _ => None,
        }
    }
}

// ── MSVC name decoration helper ─────────────────────────────────────────────

/// Apply MSVC C name decoration for a given calling convention.
///
/// `name`        — undecorated C identifier
/// `param_bytes` — total size (in bytes) of all parameters
/// `conv`        — the calling convention
pub fn msvc_decorate_c_name(name: &str, param_bytes: u32, conv: MsvcCallingConv) -> String {
    let prefix = conv.decoration_prefix();
    let suffix = conv.decoration_suffix(param_bytes);
    format!("{}{}{}", prefix, name, suffix)
}
