// ============================================================
// fastos_iterator.rs — <iterator> implementation
// ============================================================
// std::advance, std::distance, std::next, std::prev, inserters
// ============================================================

pub const ITERATOR_TYPES: &[&str] = &[
    "iterator_traits",
    "back_insert_iterator", "front_insert_iterator", "insert_iterator",
    "reverse_iterator", "move_iterator",
    "istream_iterator", "ostream_iterator",
    "istreambuf_iterator", "ostreambuf_iterator",
];

pub const ITERATOR_FUNCTIONS: &[&str] = &[
    "advance", "distance", "next", "prev",
    "begin", "end", "cbegin", "cend", "rbegin", "rend",
    "back_inserter", "front_inserter", "inserter",
    "make_reverse_iterator", "make_move_iterator",
];

pub fn is_iterator_symbol(name: &str) -> bool {
    ITERATOR_TYPES.contains(&name) || ITERATOR_FUNCTIONS.contains(&name)
}
