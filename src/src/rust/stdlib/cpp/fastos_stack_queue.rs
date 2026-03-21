// ============================================================
// fastos_stack_queue.rs — <stack> <queue> implementation
// ============================================================
// std::stack / std::queue / std::priority_queue
// Container adaptors — LIFO and FIFO abstractions
// ============================================================

pub const STACK_QUEUE_TYPES: &[&str] = &["stack", "queue", "priority_queue"];

pub const STACK_QUEUE_METHODS: &[&str] = &[
    "push", "pop", "emplace", "top", "front", "back",
    "empty", "size", "swap",
];

pub fn is_stack_queue_symbol(name: &str) -> bool {
    STACK_QUEUE_TYPES.contains(&name) || STACK_QUEUE_METHODS.contains(&name)
}
