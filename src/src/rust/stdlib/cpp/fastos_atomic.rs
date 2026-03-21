// ============================================================
// fastos_atomic.rs — <atomic> implementation
// ============================================================
// std::atomic<T>, std::atomic_flag, memory_order
// ============================================================

pub const ATOMIC_TYPES: &[&str] = &["atomic", "atomic_flag", "memory_order"];

pub const ATOMIC_METHODS: &[&str] = &[
    "store", "load", "exchange",
    "compare_exchange_weak", "compare_exchange_strong",
    "fetch_add", "fetch_sub", "fetch_or", "fetch_and", "fetch_xor",
    "test_and_set", "clear",
    "is_lock_free", "notify_one", "notify_all", "wait",
];

pub const ATOMIC_CONSTANTS: &[&str] = &[
    "memory_order_relaxed", "memory_order_consume", "memory_order_acquire",
    "memory_order_release", "memory_order_acq_rel", "memory_order_seq_cst",
];

pub fn is_atomic_symbol(name: &str) -> bool {
    ATOMIC_TYPES.contains(&name) || ATOMIC_METHODS.contains(&name) || ATOMIC_CONSTANTS.contains(&name)
}
