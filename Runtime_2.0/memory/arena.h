// ============================================================
// PyDead-BIB Runtime 2.0 — Arena Allocator 💀🦈
// Sin GC — Determinista — Memoria Continua
// ============================================================

#ifndef PYDEAD_ARENA_H
#define PYDEAD_ARENA_H

#include <stdint.h>
#include <stddef.h>

typedef struct {
    uint8_t* base;
    size_t   capacity;
    size_t   offset;
    size_t   peak;
} Arena;

Arena*  arena_create(size_t capacity_bytes);
void*   arena_alloc(Arena* arena, size_t size, size_t align);
void    arena_reset(Arena* arena);
void    arena_free(Arena* arena);
size_t  arena_used(const Arena* arena);
size_t  arena_remaining(const Arena* arena);
void    arena_print_stats(const Arena* arena);
void*   arena_alloc_tensor(Arena* arena, size_t float_count);

#endif
