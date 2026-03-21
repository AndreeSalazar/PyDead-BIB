// ============================================================
// fastos_span.rs — <span> implementation
// ============================================================
// std::span<T> — Non-owning view over contiguous data (C++20)
// ============================================================

pub const SPAN_METHODS: &[&str] = &[
    "size", "size_bytes", "empty",
    "data", "front", "back",
    "operator[]",
    "first", "last", "subspan",
    "begin", "end", "rbegin", "rend",
];

pub const SPAN_CONSTANTS: &[&str] = &["dynamic_extent"];

pub fn is_span_symbol(name: &str) -> bool {
    name == "span" || SPAN_METHODS.contains(&name) || SPAN_CONSTANTS.contains(&name)
}
