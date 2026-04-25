// ============================================================
// PyDead-BIB Runtime 2.0 — Arena Allocator 💀🦈
// ============================================================

#include "arena.h"
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#else
#include <sys/mman.h>
#endif

Arena* arena_create(size_t capacity_bytes) {
    Arena* a = (Arena*)malloc(sizeof(Arena));
    if (!a) return NULL;

#ifdef _WIN32
    a->base = (uint8_t*)VirtualAlloc(NULL, capacity_bytes,
        MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
#else
    a->base = (uint8_t*)mmap(NULL, capacity_bytes,
        PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
#endif

    if (!a->base) { free(a); return NULL; }
    a->capacity = capacity_bytes;
    a->offset = 0;
    a->peak = 0;
    return a;
}

void* arena_alloc(Arena* arena, size_t size, size_t align) {
    size_t aligned_offset = (arena->offset + align - 1) & ~(align - 1);
    if (aligned_offset + size > arena->capacity) return NULL;

    void* ptr = arena->base + aligned_offset;
    arena->offset = aligned_offset + size;
    if (arena->offset > arena->peak) arena->peak = arena->offset;
    return ptr;
}

void arena_reset(Arena* arena) {
    arena->offset = 0;
}

void arena_free(Arena* arena) {
    if (!arena) return;
#ifdef _WIN32
    VirtualFree(arena->base, 0, MEM_RELEASE);
#else
    munmap(arena->base, arena->capacity);
#endif
    free(arena);
}

size_t arena_used(const Arena* arena) { return arena->offset; }
size_t arena_remaining(const Arena* arena) { return arena->capacity - arena->offset; }

void arena_print_stats(const Arena* arena) {
    printf("Arena: %zu / %zu bytes used (peak: %zu, %.1f%%)\n",
        arena->offset, arena->capacity, arena->peak,
        100.0 * (double)arena->peak / (double)arena->capacity);
}

void* arena_alloc_tensor(Arena* arena, size_t float_count) {
    return arena_alloc(arena, float_count * sizeof(float), 32);
}
