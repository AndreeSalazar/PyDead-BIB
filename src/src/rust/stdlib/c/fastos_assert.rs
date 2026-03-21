// ============================================================
// fastos_assert.rs — <assert.h> implementation
// ============================================================
// assert, static_assert, NDEBUG
// ============================================================

pub const ASSERT_MACROS: &[(&str, &str)] = &[
    ("assert", "assert(cond) → void | abort()"),
    ("static_assert", "static_assert(cond, msg) → compiletime"),
];

pub fn is_assert_symbol(name: &str) -> bool {
    name == "assert" || name == "static_assert" || name == "NDEBUG"
}
