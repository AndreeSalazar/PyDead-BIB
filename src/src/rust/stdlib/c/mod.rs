// ============================================================
// ADead-BIB C99 Standard Library
// ============================================================
// Implementaciones propias — Sin libc externa
// Cada módulo implementa un header C99 estándar
// usando syscalls directos o instrucciones x87/SSE2
// ============================================================
// FastOS Kernel Modules (Nuevos v7.1)
//   fastos_kernel → <kernel.h> <fastos.h> — API del kernel
//   fastos_io     → <fastos_io.h>          — I/O x86-64
//   fastos_asm    → __builtin_*, asm volatile, __attribute__
// ============================================================

pub mod fastos_stdio;
pub mod fastos_stdlib;
pub mod fastos_string;
pub mod fastos_math;
pub mod fastos_time;
pub mod fastos_assert;
pub mod fastos_errno;
pub mod fastos_limits;
pub mod fastos_types;

// ── FastOS Kernel (ADead-BIB v7.1) ──────────────────────────
pub mod fastos_kernel;  // kernel.h, fastos.h → kprintf, kmalloc, process_t, KERNEL_PANIC...
pub mod fastos_io;      // fastos_io.h        → inb/outb, cli/sti, read_cr3, PIC/PIT
pub mod fastos_asm;     // built-ins          → __builtin_va_list, __attribute__, asm volatile
