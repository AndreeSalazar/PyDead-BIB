// ============================================================
// fastos_deque.rs — <deque> implementation
// ============================================================
// std::deque<T> — Double-ended queue
// O(1) push/pop at both ends, O(1) random access
// ============================================================

pub const DEQUE_METHODS: &[&str] = &[
    "push_back", "push_front", "pop_back", "pop_front",
    "operator[]", "at", "front", "back",
    "insert", "emplace", "emplace_back", "emplace_front", "erase",
    "size", "empty", "clear", "resize", "shrink_to_fit",
    "begin", "end", "rbegin", "rend", "cbegin", "cend",
    "assign", "swap",
];

pub fn is_deque_symbol(name: &str) -> bool {
    name == "deque" || DEQUE_METHODS.contains(&name)
}
