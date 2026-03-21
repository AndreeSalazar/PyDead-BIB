// ============================================================
// fastos_time.rs — <time.h> implementation
// ============================================================
// clock, time, sleep, gettimeofday, localtime, gmtime
// Implementado sobre syscalls del OS
// ============================================================

pub const TIME_FUNCTIONS: &[&str] = &[
    "time", "clock", "difftime", "mktime",
    "localtime", "gmtime", "asctime", "ctime",
    "strftime",
    "clock_gettime", "clock_getres",
    "nanosleep", "sleep", "usleep",
    "gettimeofday", "settimeofday",
];

pub const TIME_MACROS: &[(&str, &str)] = &[
    ("CLOCKS_PER_SEC", "1000000"),
    ("CLOCK_REALTIME", "0"),
    ("CLOCK_MONOTONIC", "1"),
];

pub const TIME_TYPES: &[&str] = &["time_t", "clock_t", "tm", "timespec", "timeval"];

pub fn is_time_symbol(name: &str) -> bool {
    TIME_FUNCTIONS.contains(&name)
        || TIME_MACROS.iter().any(|(n, _)| *n == name)
        || TIME_TYPES.contains(&name)
}
