// ============================================================
// fastos_any.rs — <any> implementation
// ============================================================
// std::any — Type-erased value container (C++17)
// Runtime type identification via any_cast
// ============================================================

pub const ANY_TYPES: &[&str] = &["any", "bad_any_cast"];

pub const ANY_FUNCTIONS: &[&str] = &["any_cast", "make_any"];

pub const ANY_METHODS: &[&str] = &["has_value", "reset", "swap", "type"];

pub fn is_any_symbol(name: &str) -> bool {
    ANY_TYPES.contains(&name) || ANY_FUNCTIONS.contains(&name) || ANY_METHODS.contains(&name)
}
