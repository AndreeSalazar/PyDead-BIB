// ============================================================
// fastos_tuple.rs — <tuple> implementation
// ============================================================
// std::tuple — Heterogeneous fixed-size collection
// Compile-time indexed access via std::get
// ============================================================

pub const TUPLE_TYPES: &[&str] = &["tuple", "tuple_size", "tuple_element"];

pub const TUPLE_FUNCTIONS: &[&str] = &[
    "make_tuple", "tie", "forward_as_tuple", "tuple_cat", "get", "apply",
];

pub fn is_tuple_symbol(name: &str) -> bool {
    TUPLE_TYPES.contains(&name) || TUPLE_FUNCTIONS.contains(&name)
}
