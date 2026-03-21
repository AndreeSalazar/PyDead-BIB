// ============================================================
// fastos_array.rs — <array> implementation
// ============================================================
// std::array<T,N> — Fixed-size aggregate container
// Zero-overhead wrapper over C-style arrays
// ============================================================

pub const ARRAY_METHODS: &[&str] = &[
    "operator[]", "at", "front", "back", "data",
    "size", "max_size", "empty", "fill", "swap",
    "begin", "end", "rbegin", "rend", "cbegin", "cend",
];

pub fn is_array_symbol(name: &str) -> bool {
    name == "array" || ARRAY_METHODS.contains(&name)
}
