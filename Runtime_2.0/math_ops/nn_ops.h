// ============================================================
// PyDead-BIB Runtime 2.0 — Neural Network Ops 💀🦈
// Activaciones, Loss, Normalization — SIMD AVX2
// ============================================================

#ifndef PYDEAD_NN_OPS_H
#define PYDEAD_NN_OPS_H

#include "../tensor/tensor.h"

// ── Activations ───────────────────────────────────────────
Tensor* nn_gelu(const Tensor* t);
Tensor* nn_leaky_relu(const Tensor* t, float alpha);
Tensor* nn_silu(const Tensor* t);

// ── Loss functions ────────────────────────────────────────
float   nn_mse_loss(const Tensor* pred, const Tensor* target);
float   nn_cross_entropy_loss(const Tensor* logits, const int64_t* labels, int64_t batch_size);

// ── Normalization ─────────────────────────────────────────
Tensor* nn_layer_norm(const Tensor* t, float eps);
Tensor* nn_batch_norm(const Tensor* t, const Tensor* gamma, const Tensor* beta, float eps);

// ── Utilities ─────────────────────────────────────────────
void    nn_fill_random(Tensor* t, float low, float high, uint64_t seed);
Tensor* nn_add_bias(const Tensor* x, const Tensor* bias);

#endif
