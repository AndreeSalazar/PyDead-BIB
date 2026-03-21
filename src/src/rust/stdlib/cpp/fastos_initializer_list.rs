// ============================================================
// fastos_initializer_list.rs — <initializer_list> implementation
// ============================================================
// std::initializer_list<T> — Lightweight proxy for brace-init
// ============================================================

pub const INIT_LIST_METHODS: &[&str] = &["size", "begin", "end"];

pub fn is_initializer_list_symbol(name: &str) -> bool {
    name == "initializer_list" || INIT_LIST_METHODS.contains(&name)
}
