// ============================================================
// fastos_string.rs — <string.h> implementation
// ============================================================
// strlen, strcpy, strcat, strcmp, memcpy, memset
// Implementado con instrucciones rep movsb/stosb
// SIN libc — SIN linker externo
// ============================================================

pub const STRING_FUNCTIONS: &[&str] = &[
    "strlen", "strcpy", "strncpy",
    "strcat", "strncat",
    "strcmp", "strncmp",
    "strchr", "strrchr", "strstr",
    "memcpy", "memmove", "memset", "memcmp", "memchr",
    "strtok", "strdup", "strndup",
    "strerror", "strcoll", "strxfrm",
    "strpbrk", "strspn", "strcspn",
];

pub fn is_string_symbol(name: &str) -> bool {
    STRING_FUNCTIONS.contains(&name)
}
