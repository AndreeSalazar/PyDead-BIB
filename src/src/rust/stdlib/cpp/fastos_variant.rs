// ============================================================
// fastos_variant.rs — <variant> implementation
// ============================================================
// std::variant — Type-safe discriminated union (C++17)
// Pattern matching via std::visit
// ============================================================

pub const VARIANT_TYPES: &[&str] = &["variant", "monostate", "bad_variant_access"];

pub const VARIANT_FUNCTIONS: &[&str] = &[
    "get", "get_if", "holds_alternative", "visit",
];

pub const VARIANT_METHODS: &[&str] = &[
    "index", "valueless_by_exception", "emplace", "swap",
];

pub fn is_variant_symbol(name: &str) -> bool {
    VARIANT_TYPES.contains(&name) || VARIANT_FUNCTIONS.contains(&name) || VARIANT_METHODS.contains(&name)
}
