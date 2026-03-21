// ============================================================
// fastos_string_view.rs — <string_view> implementation
// ============================================================
// std::string_view — Non-owning reference to string data
// ============================================================

pub const STRING_VIEW_TYPES: &[&str] = &["string_view", "basic_string_view", "wstring_view", "u8string_view", "u16string_view", "u32string_view"];

pub const STRING_VIEW_METHODS: &[&str] = &[
    "size", "length", "max_size", "empty",
    "data", "begin", "end", "cbegin", "cend", "rbegin", "rend",
    "operator[]", "at", "front", "back",
    "substr", "find", "rfind",
    "find_first_of", "find_last_of", "find_first_not_of", "find_last_not_of",
    "starts_with", "ends_with", "contains",
    "compare", "copy",
    "remove_prefix", "remove_suffix", "swap",
];

pub fn is_string_view_symbol(name: &str) -> bool {
    STRING_VIEW_TYPES.contains(&name) || STRING_VIEW_METHODS.contains(&name)
}
