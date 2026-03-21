// ============================================================
// fastos_functional.rs — <functional> implementation
// ============================================================
// std::function, std::bind, std::hash
// Type erasure via closure + invoke + destroy pointers
// ============================================================

pub const FUNCTIONAL_TYPES: &[&str] = &[
    "function",
    "hash",
    "less", "greater", "less_equal", "greater_equal",
    "equal_to", "not_equal_to",
    "plus", "minus", "multiplies", "divides", "modulus", "negate",
    "logical_and", "logical_or", "logical_not",
    "bit_and", "bit_or", "bit_xor", "bit_not",
    "reference_wrapper",
];

pub const FUNCTIONAL_FUNCTIONS: &[&str] = &[
    "bind", "ref", "cref",
    "mem_fn", "not_fn",
    "invoke",
];

pub fn is_functional_symbol(name: &str) -> bool {
    FUNCTIONAL_TYPES.contains(&name) || FUNCTIONAL_FUNCTIONS.contains(&name)
}

/// C inline implementation of std::function with type erasure
/// Uses void* closure + function pointers for invoke/destroy
pub const FUNCTIONAL_IMPL: &str = r#"
typedef struct {
    void* _closure;
    void* _invoke;
    void* _destroy;
} __adb_function;

static void __func_init(__adb_function* f) {
    f->_closure = 0;
    f->_invoke = 0;
    f->_destroy = 0;
}

static void __func_destroy(__adb_function* f) {
    if (f->_destroy && f->_closure) {
        ((void(*)(void*))f->_destroy)(f->_closure);
    }
    f->_closure = 0;
    f->_invoke = 0;
    f->_destroy = 0;
}

static void __func_assign_fn(__adb_function* f, void* fn_ptr) {
    __func_destroy(f);
    f->_closure = fn_ptr;
    f->_invoke = fn_ptr;
    f->_destroy = 0;
}

static long long __func_call_ii(__adb_function* f, long long arg) {
    if (!f->_invoke) return 0;
    return ((long long(*)(long long))f->_invoke)(arg);
}

static long long __func_call_void(__adb_function* f) {
    if (!f->_invoke) return 0;
    return ((long long(*)())f->_invoke)();
}

static int __func_valid(__adb_function* f) {
    return f->_invoke != 0;
}
"#;
