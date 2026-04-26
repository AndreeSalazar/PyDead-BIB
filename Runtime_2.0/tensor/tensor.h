// ============================================================
// PyDead-BIB Runtime 2.0 - Tensor Core
// Memoria continua - Sin GC - Sin VM - Determinista
// ============================================================

#ifndef PYDEAD_TENSOR_H
#define PYDEAD_TENSOR_H

#include <stdint.h>
#include <stddef.h>

#define TENSOR_MAX_DIMS 8

typedef struct {
    float*   data;                     // memoria continua (aligned 32 para AVX2)
    int64_t  shape[TENSOR_MAX_DIMS];   // dimensiones [2, 3, 4] = 3D tensor
    int64_t  strides[TENSOR_MAX_DIMS]; // strides para indexación
    int32_t  ndim;                     // número de dimensiones
    int64_t  size;                     // total de elementos
    int32_t  owns_data;                // 1 = dueño del data (free on destroy)
} Tensor;

// ── Lifecycle ─────────────────────────────────────────────
Tensor* tensor_create(const int64_t* shape, int32_t ndim);
Tensor* tensor_create_from_data(const float* data, const int64_t* shape, int32_t ndim);
Tensor* tensor_zeros(const int64_t* shape, int32_t ndim);
Tensor* tensor_ones(const int64_t* shape, int32_t ndim);
Tensor* tensor_clone(const Tensor* src);
void    tensor_free(Tensor* t);

// ── Accessors ─────────────────────────────────────────────
float   tensor_get_2d(const Tensor* t, int64_t row, int64_t col);
void    tensor_set_2d(Tensor* t, int64_t row, int64_t col, float val);
float   tensor_get_1d(const Tensor* t, int64_t idx);
void    tensor_set_1d(Tensor* t, int64_t idx, float val);

// ── Element-wise ops (SIMD AVX2) ──────────────────────────
Tensor* tensor_add(const Tensor* a, const Tensor* b);
Tensor* tensor_sub(const Tensor* a, const Tensor* b);
Tensor* tensor_mul(const Tensor* a, const Tensor* b);
Tensor* tensor_div(const Tensor* a, const Tensor* b);
Tensor* tensor_scale(const Tensor* a, float scalar);

// ── Matrix operations ─────────────────────────────────────
Tensor* tensor_matmul(const Tensor* a, const Tensor* b);
Tensor* tensor_transpose(const Tensor* a);

// ── Reductions ────────────────────────────────────────────
float   tensor_sum(const Tensor* t);
float   tensor_max(const Tensor* t);
float   tensor_min(const Tensor* t);
float   tensor_mean(const Tensor* t);

// ── Activations (IA) ──────────────────────────────────────
Tensor* tensor_relu(const Tensor* t);
Tensor* tensor_sigmoid(const Tensor* t);
Tensor* tensor_tanh_act(const Tensor* t);
Tensor* tensor_softmax(const Tensor* t);

// ── Shape ops ─────────────────────────────────────────────
Tensor* tensor_reshape(const Tensor* t, const int64_t* new_shape, int32_t new_ndim);

// ── In-place ops (no allocation, modifies dst) ────────────
void    tensor_add_inplace(Tensor* dst, const Tensor* src);
void    tensor_sub_inplace(Tensor* dst, const Tensor* src);
void    tensor_scale_inplace(Tensor* t, float scalar);

// ── Debug ─────────────────────────────────────────────────
void    tensor_print(const Tensor* t);
void    tensor_print_shape(const Tensor* t);

#endif // PYDEAD_TENSOR_H
