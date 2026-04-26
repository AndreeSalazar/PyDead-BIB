// ============================================================
// PyDead-BIB Runtime 2.0 — Optimizers 💀🦈
// SGD + Adam — Determinista — Sin GC
// ============================================================

#ifndef PYDEAD_OPTIM_H
#define PYDEAD_OPTIM_H

#include "../tensor/tensor.h"

// ── SGD ──────────────────────────────────────────────────
typedef struct {
    float lr;
    float momentum;
    float weight_decay;
    Tensor** velocities;
    int32_t  num_params;
} SGD;

SGD*  sgd_create(float lr, float momentum, float weight_decay, int32_t num_params);
void  sgd_step(SGD* opt, Tensor** params, Tensor** grads, int32_t n);
void  sgd_zero_grad(Tensor** grads, int32_t n);
void  sgd_free(SGD* opt);

// ── Adam ─────────────────────────────────────────────────
typedef struct {
    float lr;
    float beta1;
    float beta2;
    float eps;
    float weight_decay;
    Tensor** m;     // first moment
    Tensor** v;     // second moment
    int32_t  num_params;
    int32_t  t;     // step counter
} Adam;

Adam* adam_create(float lr, float beta1, float beta2, float eps,
                  float weight_decay, int32_t num_params);
void  adam_step(Adam* opt, Tensor** params, Tensor** grads, int32_t n);
void  adam_free(Adam* opt);

#endif
