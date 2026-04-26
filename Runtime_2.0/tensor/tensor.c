// ============================================================
// PyDead-BIB Runtime 2.0 - Tensor Implementation
// SIMD AVX2 - Sin GC - Memoria Continua - Determinista
// ============================================================

#include "tensor.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <math.h>

#ifdef __AVX2__
#include <immintrin.h>
#endif

// ── Aligned allocation (32-byte for AVX2) ─────────────────

static void* aligned_alloc_32(size_t size) {
#ifdef _WIN32
    return _aligned_malloc(size, 32);
#else
    void* ptr = NULL;
    posix_memalign(&ptr, 32, size);
    return ptr;
#endif
}

static void aligned_free_32(void* ptr) {
#ifdef _WIN32
    _aligned_free(ptr);
#else
    free(ptr);
#endif
}

// ── Compute strides from shape ────────────────────────────

static void compute_strides(Tensor* t) {
    t->strides[t->ndim - 1] = 1;
    for (int i = t->ndim - 2; i >= 0; i--) {
        t->strides[i] = t->strides[i + 1] * t->shape[i + 1];
    }
}

static int64_t compute_size(const int64_t* shape, int32_t ndim) {
    int64_t s = 1;
    for (int i = 0; i < ndim; i++) s *= shape[i];
    return s;
}

// ── Lifecycle ─────────────────────────────────────────────

Tensor* tensor_create(const int64_t* shape, int32_t ndim) {
    Tensor* t = (Tensor*)malloc(sizeof(Tensor));
    if (!t) return NULL;
    t->ndim = ndim;
    t->size = compute_size(shape, ndim);
    memcpy(t->shape, shape, ndim * sizeof(int64_t));
    compute_strides(t);
    t->data = (float*)aligned_alloc_32(t->size * sizeof(float));
    t->owns_data = 1;
    return t;
}

Tensor* tensor_create_from_data(const float* data, const int64_t* shape, int32_t ndim) {
    Tensor* t = tensor_create(shape, ndim);
    if (!t) return NULL;
    memcpy(t->data, data, t->size * sizeof(float));
    return t;
}

Tensor* tensor_zeros(const int64_t* shape, int32_t ndim) {
    Tensor* t = tensor_create(shape, ndim);
    if (!t) return NULL;
    memset(t->data, 0, t->size * sizeof(float));
    return t;
}

Tensor* tensor_ones(const int64_t* shape, int32_t ndim) {
    Tensor* t = tensor_create(shape, ndim);
    if (!t) return NULL;
    for (int64_t i = 0; i < t->size; i++) t->data[i] = 1.0f;
    return t;
}

Tensor* tensor_clone(const Tensor* src) {
    return tensor_create_from_data(src->data, src->shape, src->ndim);
}

void tensor_free(Tensor* t) {
    if (!t) return;
    if (t->owns_data && t->data) aligned_free_32(t->data);
    free(t);
}

// ── Accessors ─────────────────────────────────────────────

float tensor_get_2d(const Tensor* t, int64_t row, int64_t col) {
    return t->data[row * t->strides[0] + col * t->strides[1]];
}

void tensor_set_2d(Tensor* t, int64_t row, int64_t col, float val) {
    t->data[row * t->strides[0] + col * t->strides[1]] = val;
}

float tensor_get_1d(const Tensor* t, int64_t idx) {
    return t->data[idx];
}

void tensor_set_1d(Tensor* t, int64_t idx, float val) {
    t->data[idx] = val;
}

// ── Element-wise (SIMD AVX2) ──────────────────────────────

Tensor* tensor_add(const Tensor* a, const Tensor* b) {
    Tensor* r = tensor_create(a->shape, a->ndim);
    int64_t i = 0;
#ifdef __AVX2__
    for (; i + 8 <= a->size; i += 8) {
        __m256 va = _mm256_load_ps(&a->data[i]);
        __m256 vb = _mm256_load_ps(&b->data[i]);
        _mm256_store_ps(&r->data[i], _mm256_add_ps(va, vb));
    }
#endif
    for (; i < a->size; i++) {
        r->data[i] = a->data[i] + b->data[i];
    }
    return r;
}

Tensor* tensor_sub(const Tensor* a, const Tensor* b) {
    Tensor* r = tensor_create(a->shape, a->ndim);
    int64_t i = 0;
#ifdef __AVX2__
    for (; i + 8 <= a->size; i += 8) {
        __m256 va = _mm256_load_ps(&a->data[i]);
        __m256 vb = _mm256_load_ps(&b->data[i]);
        _mm256_store_ps(&r->data[i], _mm256_sub_ps(va, vb));
    }
#endif
    for (; i < a->size; i++) {
        r->data[i] = a->data[i] - b->data[i];
    }
    return r;
}

Tensor* tensor_mul(const Tensor* a, const Tensor* b) {
    Tensor* r = tensor_create(a->shape, a->ndim);
    int64_t i = 0;
#ifdef __AVX2__
    for (; i + 8 <= a->size; i += 8) {
        __m256 va = _mm256_load_ps(&a->data[i]);
        __m256 vb = _mm256_load_ps(&b->data[i]);
        _mm256_store_ps(&r->data[i], _mm256_mul_ps(va, vb));
    }
#endif
    for (; i < a->size; i++) {
        r->data[i] = a->data[i] * b->data[i];
    }
    return r;
}

Tensor* tensor_div(const Tensor* a, const Tensor* b) {
    Tensor* r = tensor_create(a->shape, a->ndim);
    int64_t i = 0;
#ifdef __AVX2__
    for (; i + 8 <= a->size; i += 8) {
        __m256 va = _mm256_load_ps(&a->data[i]);
        __m256 vb = _mm256_load_ps(&b->data[i]);
        _mm256_store_ps(&r->data[i], _mm256_div_ps(va, vb));
    }
#endif
    for (; i < a->size; i++) {
        r->data[i] = a->data[i] / b->data[i];
    }
    return r;
}

Tensor* tensor_scale(const Tensor* a, float scalar) {
    Tensor* r = tensor_create(a->shape, a->ndim);
    int64_t i = 0;
#ifdef __AVX2__
    __m256 vs = _mm256_set1_ps(scalar);
    for (; i + 8 <= a->size; i += 8) {
        __m256 va = _mm256_load_ps(&a->data[i]);
        _mm256_store_ps(&r->data[i], _mm256_mul_ps(va, vs));
    }
#endif
    for (; i < a->size; i++) {
        r->data[i] = a->data[i] * scalar;
    }
    return r;
}

// ── Matrix multiply (cache-friendly i-k-j + AVX2 FMA) ────

Tensor* tensor_matmul(const Tensor* a, const Tensor* b) {
    int64_t M = a->shape[0];
    int64_t K = a->shape[1];
    int64_t N = b->shape[1];
    int64_t shape[2] = {M, N};
    Tensor* r = tensor_zeros(shape, 2);

    for (int64_t i = 0; i < M; i++) {
        for (int64_t k = 0; k < K; k++) {
            float a_ik = a->data[i * K + k];
            int64_t j = 0;
#ifdef __AVX2__
            __m256 va = _mm256_set1_ps(a_ik);
            for (; j + 8 <= N; j += 8) {
                __m256 vb = _mm256_loadu_ps(&b->data[k * N + j]);
                __m256 vr = _mm256_loadu_ps(&r->data[i * N + j]);
                vr = _mm256_fmadd_ps(va, vb, vr);
                _mm256_storeu_ps(&r->data[i * N + j], vr);
            }
#endif
            for (; j < N; j++) {
                r->data[i * N + j] += a_ik * b->data[k * N + j];
            }
        }
    }
    return r;
}

Tensor* tensor_transpose(const Tensor* a) {
    int64_t shape[2] = {a->shape[1], a->shape[0]};
    Tensor* r = tensor_create(shape, 2);
    int64_t rows = a->shape[0];
    int64_t cols = a->shape[1];
    for (int64_t i = 0; i < rows; i++) {
        for (int64_t j = 0; j < cols; j++) {
            r->data[j * rows + i] = a->data[i * cols + j];
        }
    }
    return r;
}

// ── Reductions ────────────────────────────────────────────

float tensor_sum(const Tensor* t) {
    float s = 0.0f;
    int64_t i = 0;
#ifdef __AVX2__
    __m256 vsum = _mm256_setzero_ps();
    for (; i + 8 <= t->size; i += 8) {
        vsum = _mm256_add_ps(vsum, _mm256_load_ps(&t->data[i]));
    }
#ifdef _MSC_VER
    __declspec(align(32)) float tmp[8];
#else
    float tmp[8] __attribute__((aligned(32)));
#endif
    _mm256_store_ps(tmp, vsum);
    s = tmp[0] + tmp[1] + tmp[2] + tmp[3] + tmp[4] + tmp[5] + tmp[6] + tmp[7];
#endif
    for (; i < t->size; i++) s += t->data[i];
    return s;
}

float tensor_max(const Tensor* t) {
    float m = t->data[0];
    for (int64_t i = 1; i < t->size; i++) {
        if (t->data[i] > m) m = t->data[i];
    }
    return m;
}

float tensor_min(const Tensor* t) {
    float m = t->data[0];
    for (int64_t i = 1; i < t->size; i++) {
        if (t->data[i] < m) m = t->data[i];
    }
    return m;
}

float tensor_mean(const Tensor* t) {
    return tensor_sum(t) / (float)t->size;
}

// ── Activations ───────────────────────────────────────────

Tensor* tensor_relu(const Tensor* t) {
    Tensor* r = tensor_create(t->shape, t->ndim);
    int64_t i = 0;
#ifdef __AVX2__
    __m256 vzero = _mm256_setzero_ps();
    for (; i + 8 <= t->size; i += 8) {
        __m256 v = _mm256_load_ps(&t->data[i]);
        _mm256_store_ps(&r->data[i], _mm256_max_ps(v, vzero));
    }
#endif
    for (; i < t->size; i++) {
        r->data[i] = t->data[i] > 0.0f ? t->data[i] : 0.0f;
    }
    return r;
}

Tensor* tensor_sigmoid(const Tensor* t) {
    Tensor* r = tensor_create(t->shape, t->ndim);
    for (int64_t i = 0; i < t->size; i++) {
        r->data[i] = 1.0f / (1.0f + expf(-t->data[i]));
    }
    return r;
}

Tensor* tensor_tanh_act(const Tensor* t) {
    Tensor* r = tensor_create(t->shape, t->ndim);
    for (int64_t i = 0; i < t->size; i++) {
        r->data[i] = tanhf(t->data[i]);
    }
    return r;
}

Tensor* tensor_softmax(const Tensor* t) {
    Tensor* r = tensor_create(t->shape, t->ndim);

    if (t->ndim == 1) {
        float max_val = t->data[0];
        for (int64_t i = 1; i < t->size; i++) {
            if (t->data[i] > max_val) max_val = t->data[i];
        }
        float sum = 0.0f;
        for (int64_t i = 0; i < t->size; i++) {
            r->data[i] = expf(t->data[i] - max_val);
            sum += r->data[i];
        }
        for (int64_t i = 0; i < t->size; i++) {
            r->data[i] /= sum;
        }
    } else if (t->ndim == 2) {
        int64_t rows = t->shape[0];
        int64_t cols = t->shape[1];
        for (int64_t row = 0; row < rows; row++) {
            float max_val = t->data[row * cols];
            for (int64_t j = 1; j < cols; j++) {
                float v = t->data[row * cols + j];
                if (v > max_val) max_val = v;
            }
            float sum = 0.0f;
            for (int64_t j = 0; j < cols; j++) {
                r->data[row * cols + j] = expf(t->data[row * cols + j] - max_val);
                sum += r->data[row * cols + j];
            }
            for (int64_t j = 0; j < cols; j++) {
                r->data[row * cols + j] /= sum;
            }
        }
    }
    return r;
}

// ── Shape ops ─────────────────────────────────────────────

Tensor* tensor_reshape(const Tensor* t, const int64_t* new_shape, int32_t new_ndim) {
    Tensor* r = (Tensor*)malloc(sizeof(Tensor));
    r->ndim = new_ndim;
    r->size = t->size;
    memcpy(r->shape, new_shape, new_ndim * sizeof(int64_t));
    compute_strides(r);
    r->data = t->data;  // shared data — no copy
    r->owns_data = 0;   // does NOT own data
    return r;
}

// ── Debug ─────────────────────────────────────────────────

void tensor_print_shape(const Tensor* t) {
    printf("Tensor(shape=[");
    for (int i = 0; i < t->ndim; i++) {
        printf("%lld", (long long)t->shape[i]);
        if (i < t->ndim - 1) printf(", ");
    }
    printf("], size=%lld)\n", (long long)t->size);
}

void tensor_print(const Tensor* t) {
    tensor_print_shape(t);
    if (t->ndim == 1) {
        printf("[");
        for (int64_t i = 0; i < t->size && i < 20; i++) {
            printf("%.4f", t->data[i]);
            if (i < t->size - 1) printf(", ");
        }
        if (t->size > 20) printf(", ...");
        printf("]\n");
    } else if (t->ndim == 2) {
        int64_t rows = t->shape[0];
        int64_t cols = t->shape[1];
        printf("[\n");
        for (int64_t i = 0; i < rows && i < 10; i++) {
            printf("  [");
            for (int64_t j = 0; j < cols && j < 10; j++) {
                printf("%.4f", t->data[i * cols + j]);
                if (j < cols - 1) printf(", ");
            }
            if (cols > 10) printf(", ...");
            printf("]\n");
        }
        if (rows > 10) printf("  ...\n");
        printf("]\n");
    }
}

// ── In-place ops (no allocation, modifies dst) ────────────

void tensor_add_inplace(Tensor* dst, const Tensor* src) {
    int64_t i = 0;
#ifdef __AVX2__
    for (; i + 8 <= dst->size; i += 8) {
        __m256 vd = _mm256_load_ps(&dst->data[i]);
        __m256 vs = _mm256_load_ps(&src->data[i]);
        _mm256_store_ps(&dst->data[i], _mm256_add_ps(vd, vs));
    }
#endif
    for (; i < dst->size; i++) {
        dst->data[i] += src->data[i];
    }
}

void tensor_sub_inplace(Tensor* dst, const Tensor* src) {
    int64_t i = 0;
#ifdef __AVX2__
    for (; i + 8 <= dst->size; i += 8) {
        __m256 vd = _mm256_load_ps(&dst->data[i]);
        __m256 vs = _mm256_load_ps(&src->data[i]);
        _mm256_store_ps(&dst->data[i], _mm256_sub_ps(vd, vs));
    }
#endif
    for (; i < dst->size; i++) {
        dst->data[i] -= src->data[i];
    }
}

void tensor_scale_inplace(Tensor* t, float scalar) {
    int64_t i = 0;
#ifdef __AVX2__
    __m256 vs = _mm256_set1_ps(scalar);
    for (; i + 8 <= t->size; i += 8) {
        __m256 vd = _mm256_load_ps(&t->data[i]);
        _mm256_store_ps(&t->data[i], _mm256_mul_ps(vd, vs));
    }
#endif
    for (; i < t->size; i++) {
        t->data[i] *= scalar;
    }
}
