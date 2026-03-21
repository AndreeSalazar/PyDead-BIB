// ============================================================
// fastos_types.rs — <stdint.h> + <stddef.h> + <stdbool.h>
// ============================================================
// int8_t, uint64_t, size_t, bool, NULL, offsetof
// ============================================================

pub const STDINT_TYPES: &[(&str, &str)] = &[
    ("int8_t", "signed char"),
    ("int16_t", "short"),
    ("int32_t", "int"),
    ("int64_t", "long long"),
    ("uint8_t", "unsigned char"),
    ("uint16_t", "unsigned short"),
    ("uint32_t", "unsigned int"),
    ("uint64_t", "unsigned long long"),
    ("intptr_t", "long long"),
    ("uintptr_t", "unsigned long long"),
    ("intmax_t", "long long"),
    ("uintmax_t", "unsigned long long"),
    ("int_least8_t", "signed char"),
    ("int_least16_t", "short"),
    ("int_least32_t", "int"),
    ("int_least64_t", "long long"),
    ("uint_least8_t", "unsigned char"),
    ("uint_least16_t", "unsigned short"),
    ("uint_least32_t", "unsigned int"),
    ("uint_least64_t", "unsigned long long"),
    ("int_fast8_t", "signed char"),
    ("int_fast16_t", "long long"),
    ("int_fast32_t", "long long"),
    ("int_fast64_t", "long long"),
    ("uint_fast8_t", "unsigned char"),
    ("uint_fast16_t", "unsigned long long"),
    ("uint_fast32_t", "unsigned long long"),
    ("uint_fast64_t", "unsigned long long"),
];

pub const STDDEF_TYPES: &[(&str, &str)] = &[
    ("size_t", "unsigned long long"),
    ("ssize_t", "long long"),
    ("ptrdiff_t", "long long"),
    ("off_t", "long long"),
    ("wchar_t", "int"),
    ("wint_t", "unsigned int"),
    ("max_align_t", "long double"),
];

pub const STDBOOL_DEFS: &[(&str, &str)] = &[
    ("bool", "_Bool"),
    ("true", "1"),
    ("false", "0"),
    ("__bool_true_false_are_defined", "1"),
];

pub const COMMON_MACROS: &[(&str, &str)] = &[
    ("NULL", "((void*)0)"),
    ("offsetof", "__builtin_offsetof"),
];

pub fn is_types_symbol(name: &str) -> bool {
    STDINT_TYPES.iter().any(|(n, _)| *n == name)
        || STDDEF_TYPES.iter().any(|(n, _)| *n == name)
        || STDBOOL_DEFS.iter().any(|(n, _)| *n == name)
        || COMMON_MACROS.iter().any(|(n, _)| *n == name)
}
