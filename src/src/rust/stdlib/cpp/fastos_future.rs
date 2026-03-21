// ============================================================
// fastos_future.rs — <future> implementation
// ============================================================
// std::future / std::promise — Asynchronous result passing
// Task-based parallelism with std::async
// ============================================================

pub const FUTURE_TYPES: &[&str] = &["future", "shared_future", "promise", "packaged_task", "launch", "future_status"];

pub const FUTURE_FUNCTIONS: &[&str] = &["async"];

pub const FUTURE_METHODS: &[&str] = &[
    "get", "wait", "wait_for", "wait_until", "valid", "share",
    "set_value", "set_exception", "get_future",
];

pub fn is_future_symbol(name: &str) -> bool {
    FUTURE_TYPES.contains(&name) || FUTURE_FUNCTIONS.contains(&name) || FUTURE_METHODS.contains(&name)
}
