// ============================================================
// fastos_list.rs — <list> <forward_list> implementation
// ============================================================
// std::list / std::forward_list — Doubly/singly linked lists
// O(1) insert/erase at known positions
// ============================================================

pub const LIST_TYPES: &[&str] = &["list", "forward_list"];

pub const LIST_METHODS: &[&str] = &[
    "push_back", "push_front", "pop_back", "pop_front",
    "insert", "insert_after", "emplace", "emplace_back", "emplace_front", "emplace_after",
    "erase", "erase_after", "remove", "remove_if",
    "sort", "merge", "reverse", "unique", "splice", "splice_after",
    "size", "empty", "clear", "begin", "end", "rbegin", "rend",
    "cbegin", "cend", "front", "back", "assign", "resize", "swap",
    "before_begin", "cbefore_begin",
];

pub fn is_list_symbol(name: &str) -> bool {
    LIST_TYPES.contains(&name) || LIST_METHODS.contains(&name)
}
