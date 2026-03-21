// ============================================================
// gcc_compat.rs — GCC Flag Compatibility Layer
// ============================================================
// Emula flags GCC comunes para que usuarios de GCC
// puedan migrar sin dolor a ADead-BIB
// ============================================================

/// GCC optimization levels mapped to ADead-BIB optimizer settings
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GccOptLevel {
    O0,  // No optimization
    O1,  // Basic optimizations
    O2,  // Standard optimizations (default)
    O3,  // Aggressive optimizations
    Os,  // Optimize for size
    Og,  // Optimize for debugging
}

/// GCC warning flags
#[derive(Debug, Clone, PartialEq)]
pub enum GccWarning {
    Wall,           // Enable most warnings
    Wextra,         // Enable extra warnings
    Werror,         // Treat warnings as errors
    Wpedantic,      // ISO C strict conformance
    Wno(String),    // Disable specific warning
}

/// GCC standard flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GccStandard {
    C89,
    C99,
    C11,
    C17,
    Cpp98,
    Cpp11,
    Cpp14,
    Cpp17,
    Cpp20,
    GnuC99,    // C99 + GNU extensions
    GnuCpp17,  // C++17 + GNU extensions
}

/// Parse a GCC-style flag and return its ADead-BIB equivalent
pub fn parse_gcc_flag(flag: &str) -> Option<GccFlagResult> {
    match flag {
        "-O0" => Some(GccFlagResult::OptLevel(GccOptLevel::O0)),
        "-O1" | "-O" => Some(GccFlagResult::OptLevel(GccOptLevel::O1)),
        "-O2" => Some(GccFlagResult::OptLevel(GccOptLevel::O2)),
        "-O3" => Some(GccFlagResult::OptLevel(GccOptLevel::O3)),
        "-Os" => Some(GccFlagResult::OptLevel(GccOptLevel::Os)),
        "-Og" => Some(GccFlagResult::OptLevel(GccOptLevel::Og)),
        "-Wall" => Some(GccFlagResult::Warning(GccWarning::Wall)),
        "-Wextra" => Some(GccFlagResult::Warning(GccWarning::Wextra)),
        "-Werror" => Some(GccFlagResult::Warning(GccWarning::Werror)),
        "-Wpedantic" | "-pedantic" => Some(GccFlagResult::Warning(GccWarning::Wpedantic)),
        "-g" => Some(GccFlagResult::DebugInfo),
        "-c" => Some(GccFlagResult::CompileOnly),
        "-S" => Some(GccFlagResult::AsmOutput),
        "-E" => Some(GccFlagResult::PreprocessOnly),
        "-std=c89" | "-std=c90" => Some(GccFlagResult::Standard(GccStandard::C89)),
        "-std=c99" => Some(GccFlagResult::Standard(GccStandard::C99)),
        "-std=c11" => Some(GccFlagResult::Standard(GccStandard::C11)),
        "-std=c17" | "-std=c18" => Some(GccFlagResult::Standard(GccStandard::C17)),
        "-std=c++98" | "-std=c++03" => Some(GccFlagResult::Standard(GccStandard::Cpp98)),
        "-std=c++11" => Some(GccFlagResult::Standard(GccStandard::Cpp11)),
        "-std=c++14" => Some(GccFlagResult::Standard(GccStandard::Cpp14)),
        "-std=c++17" => Some(GccFlagResult::Standard(GccStandard::Cpp17)),
        "-std=c++20" => Some(GccFlagResult::Standard(GccStandard::Cpp20)),
        "-std=gnu99" => Some(GccFlagResult::Standard(GccStandard::GnuC99)),
        "-std=gnu++17" => Some(GccFlagResult::Standard(GccStandard::GnuCpp17)),
        _ if flag.starts_with("-Wno-") => {
            let warning = flag[5..].to_string();
            Some(GccFlagResult::Warning(GccWarning::Wno(warning)))
        }
        _ if flag.starts_with("-I") => {
            let path = flag[2..].to_string();
            Some(GccFlagResult::IncludePath(path))
        }
        _ if flag.starts_with("-D") => {
            let macro_def = flag[2..].to_string();
            Some(GccFlagResult::Define(macro_def))
        }
        _ if flag.starts_with("-l") => {
            // ADead-BIB: linker flags ignored — we handle everything internally
            Some(GccFlagResult::LinkerIgnored(flag.to_string()))
        }
        _ if flag.starts_with("-L") => {
            // ADead-BIB: library paths ignored — internal stdlib
            Some(GccFlagResult::LinkerIgnored(flag.to_string()))
        }
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GccFlagResult {
    OptLevel(GccOptLevel),
    Warning(GccWarning),
    Standard(GccStandard),
    DebugInfo,
    CompileOnly,
    AsmOutput,
    PreprocessOnly,
    IncludePath(String),
    Define(String),
    LinkerIgnored(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcc_flags() {
        assert_eq!(parse_gcc_flag("-O2"), Some(GccFlagResult::OptLevel(GccOptLevel::O2)));
        assert_eq!(parse_gcc_flag("-Wall"), Some(GccFlagResult::Warning(GccWarning::Wall)));
        assert_eq!(parse_gcc_flag("-std=c99"), Some(GccFlagResult::Standard(GccStandard::C99)));
        assert_eq!(parse_gcc_flag("-std=c++17"), Some(GccFlagResult::Standard(GccStandard::Cpp17)));
        assert_eq!(parse_gcc_flag("-g"), Some(GccFlagResult::DebugInfo));
        assert!(parse_gcc_flag("-lm").is_some()); // linker flag ignored
        assert!(parse_gcc_flag("-lstdc++").is_some()); // linker flag ignored
    }
}
