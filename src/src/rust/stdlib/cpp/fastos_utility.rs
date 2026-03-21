// ============================================================
// fastos_utility.rs — <utility> implementation
// ============================================================
// std::pair, std::move, std::forward, make_pair, swap
// ============================================================

pub const UTILITY_TYPES: &[&str] = &[
    "pair",
    "tuple",
    "integer_sequence", "index_sequence",
    "in_place_t", "in_place_type_t", "in_place_index_t",
];

pub const UTILITY_FUNCTIONS: &[&str] = &[
    "make_pair",
    "move", "forward",
    "swap", "exchange",
    "declval",
    "as_const",
    "make_index_sequence", "make_integer_sequence",
    "get",
];

pub fn is_utility_symbol(name: &str) -> bool {
    UTILITY_TYPES.contains(&name) || UTILITY_FUNCTIONS.contains(&name)
}
