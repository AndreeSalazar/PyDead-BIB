# Memory — Runtime 2.0 💀🦈

Arena allocator determinista. Sin GC — free todo de golpe.

- `arena_create(size)` → VirtualAlloc/mmap un bloque grande
- `arena_alloc(size, align)` → O(1) bump allocator
- `arena_reset()` → reutiliza sin free (inferencia por batch)
- `arena_free()` → libera todo

## Estado: ✅ Implementado
