// ============================================================
// clang_compat.rs — Clang/LLVM Flag Compatibility Layer
// ============================================================
// Emula flags Clang comunes para que usuarios de Clang
// puedan migrar sin dolor a ADead-BIB
// ============================================================

/// Clang-specific flags that differ from GCC
#[derive(Debug, Clone, PartialEq)]
pub enum ClangFlag {
    // Sanitizers (ADead-BIB has UB detector instead)
    Fsanitize(Vec<String>),      // -fsanitize=address,undefined
    FsanitizeRecover(Vec<String>),

    // Clang-specific warnings
    WeverythingFlag,             // -Weverything
    WnoEverythingFlag,

    // Target triple
    Target(String),              // --target=x86_64-pc-linux-gnu

    // Clang-specific options
    Fcolor,                      // -fcolor-diagnostics
    Fno(String),                 // -fno-exceptions, -fno-rtti
    Stdlib(String),              // -stdlib=libc++

    // Module support
    Fmodules,                    // -fmodules
    FmoduleCachePath(String),
}

/// Parse a Clang-specific flag
pub fn parse_clang_flag(flag: &str) -> Option<ClangFlagResult> {
    match flag {
        "-Weverything" => Some(ClangFlagResult::Flag(ClangFlag::WeverythingFlag)),
        "-fcolor-diagnostics" => Some(ClangFlagResult::Flag(ClangFlag::Fcolor)),
        "-fmodules" => Some(ClangFlagResult::Flag(ClangFlag::Fmodules)),
        _ if flag.starts_with("-fsanitize=") => {
            let sanitizers: Vec<String> = flag[11..].split(',').map(|s| s.to_string()).collect();
            // ADead-BIB: sanitizers replaced by UB detector
            Some(ClangFlagResult::UBDetectorReplacement(sanitizers))
        }
        _ if flag.starts_with("--target=") => {
            let target = flag[9..].to_string();
            Some(ClangFlagResult::Flag(ClangFlag::Target(target)))
        }
        _ if flag.starts_with("-stdlib=") => {
            // ADead-BIB: stdlib is internal, ignore
            Some(ClangFlagResult::Ignored(flag.to_string()))
        }
        _ if flag.starts_with("-fno-") => {
            let feature = flag[5..].to_string();
            Some(ClangFlagResult::Flag(ClangFlag::Fno(feature)))
        }
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClangFlagResult {
    Flag(ClangFlag),
    UBDetectorReplacement(Vec<String>),
    Ignored(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clang_flags() {
        assert!(parse_clang_flag("-Weverything").is_some());
        assert!(parse_clang_flag("-fcolor-diagnostics").is_some());

        // Sanitizers → UB detector
        if let Some(ClangFlagResult::UBDetectorReplacement(sans)) =
            parse_clang_flag("-fsanitize=address,undefined")
        {
            assert_eq!(sans.len(), 2);
            assert_eq!(sans[0], "address");
            assert_eq!(sans[1], "undefined");
        } else {
            panic!("Expected UBDetectorReplacement");
        }

        // Target
        if let Some(ClangFlagResult::Flag(ClangFlag::Target(t))) =
            parse_clang_flag("--target=x86_64-pc-linux-gnu")
        {
            assert_eq!(t, "x86_64-pc-linux-gnu");
        } else {
            panic!("Expected Target");
        }
    }
}
