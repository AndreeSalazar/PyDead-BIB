// ============================================================
// fastos_numeric.rs — <numeric> implementation
// ============================================================
// std::accumulate, iota, gcd, lcm, etc.
// ============================================================

pub const NUMERIC_FUNCTIONS: &[&str] = &[
    "accumulate", "reduce", "inner_product", "transform_reduce",
    "partial_sum", "inclusive_scan", "exclusive_scan",
    "adjacent_difference",
    "iota",
    "gcd", "lcm",
    "midpoint",
];

pub fn is_numeric_symbol(name: &str) -> bool {
    NUMERIC_FUNCTIONS.contains(&name)
}
