// ============================================================
// fastos_set.rs — <set> <unordered_set> implementation
// ============================================================
// std::set / std::unordered_set — Associative containers
// Ordered and unordered unique/multi key collections
// ============================================================

pub const SET_TYPES: &[&str] = &["set", "multiset", "unordered_set", "unordered_multiset"];

pub const SET_METHODS: &[&str] = &[
    "insert", "emplace", "emplace_hint", "erase", "find", "count", "contains",
    "lower_bound", "upper_bound", "equal_range",
    "size", "empty", "clear", "begin", "end", "rbegin", "rend",
    "cbegin", "cend", "swap", "merge", "extract",
    "key_comp", "value_comp", "hash_function", "key_eq",
    "bucket_count", "max_bucket_count", "load_factor", "max_load_factor",
];

pub fn is_set_symbol(name: &str) -> bool {
    SET_TYPES.contains(&name) || SET_METHODS.contains(&name)
}
