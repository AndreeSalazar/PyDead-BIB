// ============================================================
// fastos_condition_variable.rs — <condition_variable> implementation
// ============================================================
// std::condition_variable, std::condition_variable_any
// ============================================================

pub const CV_TYPES: &[&str] = &["condition_variable", "condition_variable_any", "cv_status"];

pub const CV_METHODS: &[&str] = &[
    "wait", "wait_for", "wait_until",
    "notify_one", "notify_all",
];

pub fn is_condition_variable_symbol(name: &str) -> bool {
    CV_TYPES.contains(&name) || CV_METHODS.contains(&name)
}
