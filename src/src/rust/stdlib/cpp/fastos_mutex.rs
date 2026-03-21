// ============================================================
// fastos_mutex.rs — <mutex> implementation
// ============================================================
// std::mutex, std::lock_guard, std::unique_lock, std::scoped_lock
// ============================================================

pub const MUTEX_TYPES: &[&str] = &[
    "mutex", "recursive_mutex", "timed_mutex", "recursive_timed_mutex",
    "lock_guard", "unique_lock", "scoped_lock",
    "once_flag",
];

pub const MUTEX_FUNCTIONS: &[&str] = &["lock", "try_lock", "call_once"];

pub const MUTEX_METHODS: &[&str] = &[
    "lock", "unlock", "try_lock", "try_lock_for", "try_lock_until",
    "owns_lock", "release", "swap",
];

pub fn is_mutex_symbol(name: &str) -> bool {
    MUTEX_TYPES.contains(&name) || MUTEX_FUNCTIONS.contains(&name) || MUTEX_METHODS.contains(&name)
}
