// ============================================================
// PyDead-BIB Runtime 2.0 — Optimizers 💀🦈
// SGD + Adam — Determinista — Sin GC
// ============================================================

#include "optim.h"
#include <stdlib.h>
#include <string.h>
#include <math.h>

// ── SGD ──────────────────────────────────────────────────

SGD* sgd_create(float lr, float momentum, float weight_decay, int32_t num_params) {
    SGD* opt = (SGD*)malloc(sizeof(SGD));
    opt->lr = lr;
    opt->momentum = momentum;
    opt->weight_decay = weight_decay;
    opt->num_params = num_params;

    if (momentum > 0.0f) {
        opt->velocities = (Tensor**)calloc(num_params, sizeof(Tensor*));
    } else {
        opt->velocities = NULL;
    }
    return opt;
}

void sgd_step(SGD* opt, Tensor** params, Tensor** grads, int32_t n) {
    for (int32_t i = 0; i < n; i++) {
        if (!params[i] || !grads[i]) continue;
        Tensor* p = params[i];
        Tensor* g = grads[i];

        for (int64_t j = 0; j < p->size; j++) {
            float grad_val = g->data[j];

            // Weight decay (L2 regularization)
            if (opt->weight_decay > 0.0f) {
                grad_val += opt->weight_decay * p->data[j];
            }

            if (opt->momentum > 0.0f) {
                // Lazy init velocity
                if (!opt->velocities[i]) {
                    opt->velocities[i] = tensor_zeros(p->shape, p->ndim);
                }
                opt->velocities[i]->data[j] =
                    opt->momentum * opt->velocities[i]->data[j] + grad_val;
                p->data[j] -= opt->lr * opt->velocities[i]->data[j];
            } else {
                p->data[j] -= opt->lr * grad_val;
            }
        }
    }
}

void sgd_zero_grad(Tensor** grads, int32_t n) {
    for (int32_t i = 0; i < n; i++) {
        if (grads[i]) {
            memset(grads[i]->data, 0, grads[i]->size * sizeof(float));
        }
    }
}

void sgd_free(SGD* opt) {
    if (!opt) return;
    if (opt->velocities) {
        for (int32_t i = 0; i < opt->num_params; i++) {
            if (opt->velocities[i]) tensor_free(opt->velocities[i]);
        }
        free(opt->velocities);
    }
    free(opt);
}

// ── Adam ─────────────────────────────────────────────────

Adam* adam_create(float lr, float beta1, float beta2, float eps,
                  float weight_decay, int32_t num_params) {
    Adam* opt = (Adam*)malloc(sizeof(Adam));
    opt->lr = lr;
    opt->beta1 = beta1;
    opt->beta2 = beta2;
    opt->eps = eps;
    opt->weight_decay = weight_decay;
    opt->num_params = num_params;
    opt->t = 0;
    opt->m = (Tensor**)calloc(num_params, sizeof(Tensor*));
    opt->v = (Tensor**)calloc(num_params, sizeof(Tensor*));
    return opt;
}

void adam_step(Adam* opt, Tensor** params, Tensor** grads, int32_t n) {
    opt->t++;
    float bc1 = 1.0f - powf(opt->beta1, (float)opt->t);
    float bc2 = 1.0f - powf(opt->beta2, (float)opt->t);

    for (int32_t i = 0; i < n; i++) {
        if (!params[i] || !grads[i]) continue;
        Tensor* p = params[i];
        Tensor* g = grads[i];

        // Lazy init moments
        if (!opt->m[i]) {
            opt->m[i] = tensor_zeros(p->shape, p->ndim);
            opt->v[i] = tensor_zeros(p->shape, p->ndim);
        }

        for (int64_t j = 0; j < p->size; j++) {
            float grad_val = g->data[j];

            // Weight decay (AdamW style)
            if (opt->weight_decay > 0.0f) {
                p->data[j] -= opt->lr * opt->weight_decay * p->data[j];
            }

            // Update moments
            opt->m[i]->data[j] = opt->beta1 * opt->m[i]->data[j] + (1.0f - opt->beta1) * grad_val;
            opt->v[i]->data[j] = opt->beta2 * opt->v[i]->data[j] + (1.0f - opt->beta2) * grad_val * grad_val;

            // Bias-corrected estimates
            float m_hat = opt->m[i]->data[j] / bc1;
            float v_hat = opt->v[i]->data[j] / bc2;

            // Update parameters
            p->data[j] -= opt->lr * m_hat / (sqrtf(v_hat) + opt->eps);
        }
    }
}

void adam_free(Adam* opt) {
    if (!opt) return;
    for (int32_t i = 0; i < opt->num_params; i++) {
        if (opt->m[i]) tensor_free(opt->m[i]);
        if (opt->v[i]) tensor_free(opt->v[i]);
    }
    free(opt->m);
    free(opt->v);
    free(opt);
}
