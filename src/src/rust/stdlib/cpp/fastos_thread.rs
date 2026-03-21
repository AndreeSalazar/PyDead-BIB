// ============================================================
// fastos_thread.rs — <thread> implementation
// ============================================================
// std::thread / std::jthread — OS thread management
// RAII-based thread lifecycle with join/detach
// ============================================================

pub const THREAD_TYPES: &[&str] = &["thread", "jthread"];

pub const THREAD_FUNCTIONS: &[&str] = &["sleep_for", "sleep_until", "yield_now", "get_id", "hardware_concurrency"];

pub const THREAD_METHODS: &[&str] = &["join", "detach", "joinable", "get_id", "swap"];

pub fn is_thread_symbol(name: &str) -> bool {
    THREAD_TYPES.contains(&name) || THREAD_FUNCTIONS.contains(&name) || THREAD_METHODS.contains(&name)
}
