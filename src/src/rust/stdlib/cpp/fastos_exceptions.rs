// ============================================================
// fastos_exceptions.rs — <exception> + <stdexcept> implementation
// ============================================================
// try/catch/throw C++98
// std::exception, std::runtime_error, std::logic_error, etc.
// ============================================================

pub const EXCEPTION_CLASSES: &[&str] = &[
    "exception",
    "bad_exception",
    "bad_alloc",
    "bad_cast",
    "bad_typeid",
    "runtime_error",
    "range_error",
    "overflow_error",
    "underflow_error",
    "logic_error",
    "domain_error",
    "invalid_argument",
    "length_error",
    "out_of_range",
];

pub const EXCEPTION_FUNCTIONS: &[&str] = &[
    "what",
    "current_exception",
    "rethrow_exception",
    "make_exception_ptr",
    "throw_with_nested",
    "rethrow_if_nested",
    "terminate",
    "set_terminate",
    "get_terminate",
    "uncaught_exception",
    "uncaught_exceptions",
];

pub fn is_exception_symbol(name: &str) -> bool {
    EXCEPTION_CLASSES.contains(&name) || EXCEPTION_FUNCTIONS.contains(&name)
}

/// C inline implementation of exception → error code system.
/// ADead-BIB eliminates stack unwinding; throw becomes __adb_set_error + return,
/// try/catch becomes body + if(__adb_has_error()) { handler }.
pub const EXCEPTION_IMPL: &str = r#"
static char __adb_error_msg[256] = {0};
static int  __adb_error_flag = 0;

static void __adb_set_error(const char* msg) {
    __adb_error_flag = 1;
    int i = 0;
    while (msg[i] && i < 255) { __adb_error_msg[i] = msg[i]; i++; }
    __adb_error_msg[i] = 0;
}

static int __adb_has_error(void) {
    return __adb_error_flag;
}

static const char* __adb_get_error(void) {
    return __adb_error_msg;
}

static void __adb_clear_error(void) {
    __adb_error_flag = 0;
    __adb_error_msg[0] = 0;
}
"#;
