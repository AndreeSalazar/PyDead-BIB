// ============================================================
// fastos_chrono.rs — <chrono> implementation
// ============================================================
// std::chrono — Time utilities and clocks
// Duration arithmetic and time point conversions
// ============================================================

pub const CHRONO_TYPES: &[&str] = &[
    "duration", "time_point",
    "high_resolution_clock", "steady_clock", "system_clock",
    "milliseconds", "microseconds", "nanoseconds", "seconds", "minutes", "hours",
    "chrono",
];

pub const CHRONO_FUNCTIONS: &[&str] = &["duration_cast", "time_point_cast"];

pub const CHRONO_METHODS: &[&str] = &["now", "count", "time_since_epoch"];

pub fn is_chrono_symbol(name: &str) -> bool {
    CHRONO_TYPES.contains(&name) || CHRONO_FUNCTIONS.contains(&name) || CHRONO_METHODS.contains(&name)
}
