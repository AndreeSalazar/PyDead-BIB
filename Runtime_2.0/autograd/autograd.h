// ============================================================
// PyDead-BIB Runtime 2.0 — Autograd 💀🦈
// Tape-based reverse-mode automatic differentiation
// STATUS: ✅ Implementado
// ============================================================

#ifndef PYDEAD_AUTOGRAD_H
#define PYDEAD_AUTOGRAD_H

#include "../tensor/tensor.h"

typedef enum {
    OP_ADD,
    OP_MUL,
    OP_MATMUL,
    OP_RELU,
    OP_SOFTMAX,
    OP_LOSS_CE,
    OP_LINEAR,
    OP_SCALE,
    OP_SUB,
} OpType;

typedef struct TapeNode {
    OpType  op;
    Tensor* output;
    Tensor* inputs[2];
    Tensor* grad;
    int32_t num_inputs;
    // extra context for some ops
    Tensor* saved_tensors[2];
    int64_t labels_cache[64];
    int64_t batch_size_cache;
} TapeNode;

typedef struct {
    TapeNode* nodes;
    int32_t   count;
    int32_t   capacity;
} Tape;

// ── Tape lifecycle ───────────────────────────────────────
Tape*   tape_create(int32_t capacity);
void    tape_record(Tape* tape, OpType op, Tensor** inputs, int32_t n, Tensor* output);
void    tape_record_with_saved(Tape* tape, OpType op, Tensor** inputs, int32_t n,
                                Tensor* output, Tensor** saved, int32_t n_saved);
void    tape_record_ce(Tape* tape, Tensor* logits, Tensor* softmax_out,
                        const int64_t* labels, int64_t batch_size);
void    tape_backward(Tape* tape);
void    tape_zero_grads(Tape* tape);
void    tape_free(Tape* tape);

// ── Autograd-aware ops (record + compute) ────────────────
Tensor* ag_add(Tape* tape, Tensor* a, Tensor* b);
Tensor* ag_mul(Tape* tape, Tensor* a, Tensor* b);
Tensor* ag_matmul(Tape* tape, Tensor* a, Tensor* b);
Tensor* ag_relu(Tape* tape, Tensor* input);
Tensor* ag_softmax(Tape* tape, Tensor* input);
float   ag_cross_entropy(Tape* tape, Tensor* logits, const int64_t* labels, int64_t batch_size);

#endif
