// ============================================================
// fastos_optional.rs — <optional> implementation
// ============================================================
// std::optional<T> — Nullable value wrapper (C++17)
// Monadic operations for safe value access
// ============================================================

pub const OPTIONAL_TYPES: &[&str] = &["optional", "nullopt_t", "bad_optional_access"];

pub const OPTIONAL_METHODS: &[&str] = &[
    "has_value", "value", "value_or", "reset", "emplace", "swap",
];

pub const OPTIONAL_FUNCTIONS: &[&str] = &["make_optional"];

pub fn is_optional_symbol(name: &str) -> bool {
    OPTIONAL_TYPES.contains(&name) || OPTIONAL_METHODS.contains(&name) || OPTIONAL_FUNCTIONS.contains(&name)
}
