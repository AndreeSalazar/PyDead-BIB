// ============================================================
// fastos_limits.rs — <limits.h> implementation
// ============================================================
// INT_MAX, INT_MIN, CHAR_MAX, SIZE_MAX, etc.
// ============================================================

pub const LIMITS_CONSTANTS: &[(&str, &str)] = &[
    ("CHAR_BIT", "8"),
    ("CHAR_MIN", "-128"),
    ("CHAR_MAX", "127"),
    ("SCHAR_MIN", "-128"),
    ("SCHAR_MAX", "127"),
    ("UCHAR_MAX", "255"),
    ("SHRT_MIN", "-32768"),
    ("SHRT_MAX", "32767"),
    ("USHRT_MAX", "65535"),
    ("INT_MIN", "-2147483648"),
    ("INT_MAX", "2147483647"),
    ("UINT_MAX", "4294967295"),
    ("LONG_MIN", "-9223372036854775808"),
    ("LONG_MAX", "9223372036854775807"),
    ("ULONG_MAX", "18446744073709551615"),
    ("LLONG_MIN", "-9223372036854775808"),
    ("LLONG_MAX", "9223372036854775807"),
    ("ULLONG_MAX", "18446744073709551615"),
    ("SIZE_MAX", "18446744073709551615"),
    ("PTRDIFF_MIN", "-9223372036854775808"),
    ("PTRDIFF_MAX", "9223372036854775807"),
    ("MB_LEN_MAX", "16"),
];

pub fn is_limits_symbol(name: &str) -> bool {
    LIMITS_CONSTANTS.iter().any(|(n, _)| *n == name)
}
