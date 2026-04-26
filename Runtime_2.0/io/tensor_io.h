// ============================================================
// PyDead-BIB Runtime 2.0 — Tensor I/O 💀🦈
// Save/Load tensores — formato binario simple + safetensors
// ============================================================

#ifndef PYDEAD_TENSOR_IO_H
#define PYDEAD_TENSOR_IO_H

#include "../tensor/tensor.h"

// ── PyDead-BIB binary format (.pdb) ──────────────────────
// Simple: [magic 4B][ndim 4B][shape ndim*8B][data size*4B]
int     tensor_save(const Tensor* t, const char* path);
Tensor* tensor_load(const char* path);

// ── Save/Load multiple tensors (model weights) ───────────
int     tensor_save_all(Tensor** tensors, const char** names,
                         int32_t count, const char* path);
int     tensor_load_all(const char* path, Tensor*** tensors,
                         char*** names, int32_t* count);

// ── Raw binary (for interop) ────────────────────────────
int     tensor_save_raw(const Tensor* t, const char* path);
Tensor* tensor_load_raw(const char* path, const int64_t* shape, int32_t ndim);

#endif
