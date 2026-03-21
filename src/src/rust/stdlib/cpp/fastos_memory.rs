// ============================================================
// fastos_memory.rs — <memory> implementation
// ============================================================
// std::unique_ptr, std::shared_ptr, std::weak_ptr
// make_unique, make_shared
// ============================================================

pub const MEMORY_TYPES: &[&str] = &[
    "unique_ptr", "shared_ptr", "weak_ptr",
    "allocator", "allocator_traits",
    "default_delete",
];

pub const MEMORY_FUNCTIONS: &[&str] = &[
    "make_unique", "make_shared",
    "dynamic_pointer_cast", "static_pointer_cast", "const_pointer_cast",
    "addressof",
    "uninitialized_copy", "uninitialized_fill",
];

pub fn is_memory_symbol(name: &str) -> bool {
    MEMORY_TYPES.contains(&name) || MEMORY_FUNCTIONS.contains(&name)
}
