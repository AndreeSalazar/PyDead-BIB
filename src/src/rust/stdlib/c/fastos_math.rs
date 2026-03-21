// ============================================================
// fastos_math.rs — <math.h> implementation
// ============================================================
// sin, cos, tan, sqrt, pow, log, PI, E, TAU
// Implementado con instrucciones x87 FPU y SSE2
// SIN libc — SIN linker externo
// ============================================================

pub const MATH_FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan",
    "asin", "acos", "atan", "atan2",
    "sqrt", "cbrt",
    "pow", "exp", "exp2",
    "log", "log2", "log10",
    "floor", "ceil", "round", "trunc",
    "fabs", "fabsf",
    "fmod", "remainder",
    "hypot",
    "sinf", "cosf", "tanf", "sqrtf", "powf", "logf",
    "floorf", "ceilf", "roundf", "truncf",
    "copysign", "copysignf",
    "fmin", "fmax", "fminf", "fmaxf",
    "isnan", "isinf", "isfinite", "isnormal",
    "nan", "nanf",
    "ldexp", "frexp", "modf",
    "scalbn", "scalbln",
];

pub const MATH_CONSTANTS: &[(&str, &str)] = &[
    ("M_PI", "3.14159265358979323846"),
    ("M_PI_2", "1.57079632679489661923"),
    ("M_PI_4", "0.78539816339744830962"),
    ("M_E", "2.71828182845904523536"),
    ("M_LN2", "0.69314718055994530942"),
    ("M_LN10", "2.30258509299404568402"),
    ("M_LOG2E", "1.44269504088896340736"),
    ("M_LOG10E", "0.43429448190325182765"),
    ("M_SQRT2", "1.41421356237309504880"),
    ("M_SQRT1_2", "0.70710678118654752440"),
    ("M_TAU", "6.28318530717958647692"),
    ("INFINITY", "__builtin_inf()"),
    ("NAN", "__builtin_nan(\"\")"),
    ("HUGE_VAL", "__builtin_huge_val()"),
];

pub fn is_math_symbol(name: &str) -> bool {
    MATH_FUNCTIONS.contains(&name)
        || MATH_CONSTANTS.iter().any(|(n, _)| *n == name)
}
