// ============================================================
// PyDead-BIB Runtime 2.0 — Autograd (Futuro) 💀🦈
// Tape-based reverse-mode automatic differentiation
// STATUS: Diseño — Implementación en fase posterior
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
} OpType;

typedef struct TapeNode {
    OpType op;
    Tensor* output;
    Tensor* inputs[2];
    Tensor* grad;
    int32_t num_inputs;
} TapeNode;

typedef struct {
    TapeNode* nodes;
    int32_t   count;
    int32_t   capacity;
} Tape;

// TODO: Implementar en fase posterior
// Tape*   tape_create(int32_t capacity);
// void    tape_record(Tape* tape, OpType op, Tensor** inputs, int n, Tensor* output);
// void    tape_backward(Tape* tape);
// void    tape_free(Tape* tape);

#endif
