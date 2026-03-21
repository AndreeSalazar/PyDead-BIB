// ============================================================
// fastos_stdlib.rs — <stdlib.h> implementation
// ============================================================
// malloc, free, calloc, realloc, exit, atoi, rand, qsort
// Implementado sobre syscall mmap/munmap directo
// SIN libc — SIN linker externo
// ============================================================

pub const STDLIB_FUNCTIONS: &[&str] = &[
    "malloc", "calloc", "realloc", "free",
    "exit", "abort", "_Exit",
    "atoi", "atol", "atoll", "atof",
    "strtol", "strtoll", "strtoul", "strtoull", "strtod", "strtof",
    "rand", "srand",
    "qsort", "bsearch",
    "abs", "labs", "llabs",
    "div", "ldiv", "lldiv",
    "getenv", "system",
    "atexit",
];

pub const STDLIB_MACROS: &[(&str, &str)] = &[
    ("NULL", "((void*)0)"),
    ("EXIT_SUCCESS", "0"),
    ("EXIT_FAILURE", "1"),
    ("RAND_MAX", "2147483647"),
    ("MB_CUR_MAX", "4"),
];

pub const STDLIB_TYPES: &[&str] = &["div_t", "ldiv_t", "lldiv_t"];

pub fn is_stdlib_symbol(name: &str) -> bool {
    STDLIB_FUNCTIONS.contains(&name)
        || STDLIB_MACROS.iter().any(|(n, _)| *n == name)
        || STDLIB_TYPES.contains(&name)
}
