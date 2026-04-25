// ============================================================
// PyDead-BIB Runtime 2.0 — Neural Network Ops 💀🦈
// ============================================================

#include "nn_ops.h"
#include <math.h>
#include <stdlib.h>

// ── PCG random (determinista) ─────────────────────────────

static uint64_t pcg_state = 0x853c49e6748fea9bULL;
static uint64_t pcg_inc   = 0xda3e39cb94b95bdbULL;

static uint32_t pcg32(void) {
    uint64_t old = pcg_state;
    pcg_state = old * 6364136223846793005ULL + pcg_inc;
    uint32_t xorshifted = (uint32_t)(((old >> 18u) ^ old) >> 27u);
    uint32_t rot = (uint32_t)(old >> 59u);
    return (xorshifted >> rot) | (xorshifted << ((-rot) & 31));
}

static float pcg_float(void) {
    return (float)pcg32() / (float)0xFFFFFFFF;
}

// ── Activations ───────────────────────────────────────────

Tensor* nn_gelu(const Tensor* t) {
    Tensor* r = tensor_create(t->shape, t->ndim);
    const float c = 0.7978845608f;
    for (int64_t i = 0; i < t->size; i++) {
        float x = t->data[i];
        float inner = c * (x + 0.044715f * x * x * x);
        r->data[i] = 0.5f * x * (1.0f + tanhf(inner));
    }
    return r;
}

Tensor* nn_leaky_relu(const Tensor* t, float alpha) {
    Tensor* r = tensor_create(t->shape, t->ndim);
    for (int64_t i = 0; i < t->size; i++) {
        r->data[i] = t->data[i] > 0.0f ? t->data[i] : alpha * t->data[i];
    }
    return r;
}

Tensor* nn_silu(const Tensor* t) {
    Tensor* r = tensor_create(t->shape, t->ndim);
    for (int64_t i = 0; i < t->size; i++) {
        float x = t->data[i];
        r->data[i] = x / (1.0f + expf(-x));
    }
    return r;
}

// ── Loss functions ────────────────────────────────────────

float nn_mse_loss(const Tensor* pred, const Tensor* target) {
    float sum = 0.0f;
    for (int64_t i = 0; i < pred->size; i++) {
        float diff = pred->data[i] - target->data[i];
        sum += diff * diff;
    }
    return sum / (float)pred->size;
}

float nn_cross_entropy_loss(const Tensor* logits, const int64_t* labels, int64_t batch_size) {
    int64_t num_classes = logits->shape[1];
    float loss = 0.0f;
    for (int64_t b = 0; b < batch_size; b++) {
        float max_val = logits->data[b * num_classes];
        for (int64_t j = 1; j < num_classes; j++) {
            float v = logits->data[b * num_classes + j];
            if (v > max_val) max_val = v;
        }
        float log_sum_exp = 0.0f;
        for (int64_t j = 0; j < num_classes; j++) {
            log_sum_exp += expf(logits->data[b * num_classes + j] - max_val);
        }
        log_sum_exp = logf(log_sum_exp) + max_val;
        loss -= logits->data[b * num_classes + labels[b]] - log_sum_exp;
    }
    return loss / (float)batch_size;
}

// ── Normalization ─────────────────────────────────────────

Tensor* nn_layer_norm(const Tensor* t, float eps) {
    Tensor* r = tensor_create(t->shape, t->ndim);
    int64_t last_dim = t->shape[t->ndim - 1];
    int64_t outer = t->size / last_dim;

    for (int64_t i = 0; i < outer; i++) {
        float* row = &t->data[i * last_dim];
        float mean = 0.0f;
        for (int64_t j = 0; j < last_dim; j++) mean += row[j];
        mean /= (float)last_dim;

        float var = 0.0f;
        for (int64_t j = 0; j < last_dim; j++) {
            float d = row[j] - mean;
            var += d * d;
        }
        var /= (float)last_dim;

        float inv_std = 1.0f / sqrtf(var + eps);
        for (int64_t j = 0; j < last_dim; j++) {
            r->data[i * last_dim + j] = (row[j] - mean) * inv_std;
        }
    }
    return r;
}

Tensor* nn_batch_norm(const Tensor* t, const Tensor* gamma, const Tensor* beta, float eps) {
    Tensor* r = tensor_create(t->shape, t->ndim);
    int64_t batch = t->shape[0];
    int64_t features = t->shape[1];

    for (int64_t f = 0; f < features; f++) {
        float mean = 0.0f;
        for (int64_t b = 0; b < batch; b++) mean += t->data[b * features + f];
        mean /= (float)batch;

        float var = 0.0f;
        for (int64_t b = 0; b < batch; b++) {
            float d = t->data[b * features + f] - mean;
            var += d * d;
        }
        var /= (float)batch;
        float inv_std = 1.0f / sqrtf(var + eps);

        float g = gamma ? gamma->data[f] : 1.0f;
        float be = beta ? beta->data[f] : 0.0f;

        for (int64_t b = 0; b < batch; b++) {
            r->data[b * features + f] = g * (t->data[b * features + f] - mean) * inv_std + be;
        }
    }
    return r;
}

// ── Utilities ─────────────────────────────────────────────

void nn_fill_random(Tensor* t, float low, float high, uint64_t seed) {
    pcg_state = seed;
    float range = high - low;
    for (int64_t i = 0; i < t->size; i++) {
        t->data[i] = low + pcg_float() * range;
    }
}

Tensor* nn_add_bias(const Tensor* x, const Tensor* bias) {
    Tensor* r = tensor_clone(x);
    if (x->ndim == 2) {
        int64_t batch = x->shape[0];
        int64_t features = x->shape[1];
        for (int64_t b = 0; b < batch; b++) {
            for (int64_t f = 0; f < features; f++) {
                r->data[b * features + f] += bias->data[f];
            }
        }
    } else {
        for (int64_t i = 0; i < x->size; i++) {
            r->data[i] += bias->data[i];
        }
    }
    return r;
}
