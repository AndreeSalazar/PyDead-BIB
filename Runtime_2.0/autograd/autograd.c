// ============================================================
// PyDead-BIB Runtime 2.0 — Autograd Implementation 💀🦈
// Tape-based reverse-mode automatic differentiation
// ============================================================

#include "autograd.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <math.h>

// ── Tape lifecycle ───────────────────────────────────────

Tape* tape_create(int32_t capacity) {
    Tape* tape = (Tape*)malloc(sizeof(Tape));
    if (!tape) return NULL;
    tape->nodes = (TapeNode*)calloc(capacity, sizeof(TapeNode));
    if (!tape->nodes) { free(tape); return NULL; }
    tape->count = 0;
    tape->capacity = capacity;
    return tape;
}

void tape_record(Tape* tape, OpType op, Tensor** inputs, int32_t n, Tensor* output) {
    if (tape->count >= tape->capacity) return;
    TapeNode* node = &tape->nodes[tape->count++];
    node->op = op;
    node->output = output;
    node->num_inputs = n;
    node->grad = NULL;
    node->saved_tensors[0] = NULL;
    node->saved_tensors[1] = NULL;
    node->batch_size_cache = 0;
    for (int32_t i = 0; i < n && i < 2; i++) {
        node->inputs[i] = inputs[i];
    }
}

void tape_record_with_saved(Tape* tape, OpType op, Tensor** inputs, int32_t n,
                             Tensor* output, Tensor** saved, int32_t n_saved) {
    if (tape->count >= tape->capacity) return;
    TapeNode* node = &tape->nodes[tape->count++];
    node->op = op;
    node->output = output;
    node->num_inputs = n;
    node->grad = NULL;
    node->batch_size_cache = 0;
    for (int32_t i = 0; i < n && i < 2; i++) {
        node->inputs[i] = inputs[i];
    }
    node->saved_tensors[0] = NULL;
    node->saved_tensors[1] = NULL;
    for (int32_t i = 0; i < n_saved && i < 2; i++) {
        node->saved_tensors[i] = saved[i];
    }
}

void tape_record_ce(Tape* tape, Tensor* logits, Tensor* softmax_out,
                     const int64_t* labels, int64_t batch_size) {
    if (tape->count >= tape->capacity) return;
    TapeNode* node = &tape->nodes[tape->count++];
    node->op = OP_LOSS_CE;
    node->output = softmax_out;
    node->inputs[0] = logits;
    node->inputs[1] = NULL;
    node->num_inputs = 1;
    node->grad = NULL;
    node->saved_tensors[0] = softmax_out;
    node->saved_tensors[1] = NULL;
    node->batch_size_cache = batch_size;
    int64_t copy_n = batch_size < 64 ? batch_size : 64;
    memcpy(node->labels_cache, labels, copy_n * sizeof(int64_t));
}

// ── Backward pass ────────────────────────────────────────

// backward for: C = A + B
// dA = dC, dB = dC (gradient flows through unchanged)
static void backward_add(TapeNode* node) {
    Tensor* grad_out = node->grad;
    if (!grad_out) return;

    // Accumulate gradient to input 0
    Tensor* a = node->inputs[0];
    (void)a;
    // For add, grad_input = grad_output (identity)
    // We find the tape node whose output == inputs[i] and accumulate
    // This is handled by the main backward loop via grad propagation
}

// backward for: C = A * B (element-wise)
// dA = dC * B, dB = dC * A
static void backward_mul(TapeNode* node) {
    // Saved: inputs[0] = A, inputs[1] = B at forward time
    (void)node;
}

// backward for: C = A @ B (matmul)
// dA = dC @ B^T, dB = A^T @ dC
static void backward_matmul(TapeNode* node) {
    (void)node;
}

// backward for: Y = relu(X)
// dX = dY * (X > 0)
static void backward_relu(TapeNode* node) {
    (void)node;
}

// backward for: cross-entropy loss with softmax
// dLogits = softmax_output - one_hot(labels)
static void backward_cross_entropy(TapeNode* node) {
    (void)node;
}

void tape_backward(Tape* tape) {
    if (tape->count == 0) return;

    // The last node is the loss — its gradient is 1.0
    TapeNode* loss_node = &tape->nodes[tape->count - 1];

    // For cross-entropy: grad = (softmax - one_hot) / batch_size
    if (loss_node->op == OP_LOSS_CE) {
        Tensor* softmax_out = loss_node->saved_tensors[0];
        int64_t batch_size = loss_node->batch_size_cache;
        int64_t num_classes = softmax_out->shape[softmax_out->ndim - 1];

        // grad_logits = softmax_output - one_hot(labels)
        Tensor* grad = tensor_clone(softmax_out);
        for (int64_t b = 0; b < batch_size; b++) {
            int64_t label = loss_node->labels_cache[b];
            if (softmax_out->ndim == 2) {
                grad->data[b * num_classes + label] -= 1.0f;
            } else {
                grad->data[label] -= 1.0f;
            }
        }
        // Average over batch
        for (int64_t i = 0; i < grad->size; i++) {
            grad->data[i] /= (float)batch_size;
        }
        loss_node->grad = grad;

        // Propagate to the logits input node
        Tensor* logits = loss_node->inputs[0];
        for (int32_t i = tape->count - 2; i >= 0; i--) {
            if (tape->nodes[i].output == logits) {
                if (tape->nodes[i].grad) {
                    Tensor* acc = tensor_add(tape->nodes[i].grad, grad);
                    tensor_free(tape->nodes[i].grad);
                    tape->nodes[i].grad = acc;
                } else {
                    tape->nodes[i].grad = tensor_clone(grad);
                }
                break;
            }
        }
    }

    // Traverse tape in reverse (skip the loss node we already handled)
    int32_t start = (loss_node->op == OP_LOSS_CE) ? tape->count - 2 : tape->count - 1;

    for (int32_t idx = start; idx >= 0; idx--) {
        TapeNode* node = &tape->nodes[idx];
        Tensor* grad_out = node->grad;
        if (!grad_out) continue;

        switch (node->op) {
            case OP_ADD: {
                // dA = grad_out, dB = grad_out
                for (int inp = 0; inp < node->num_inputs; inp++) {
                    Tensor* input_tensor = node->inputs[inp];
                    for (int32_t j = idx - 1; j >= 0; j--) {
                        if (tape->nodes[j].output == input_tensor) {
                            if (tape->nodes[j].grad) {
                                Tensor* acc = tensor_add(tape->nodes[j].grad, grad_out);
                                tensor_free(tape->nodes[j].grad);
                                tape->nodes[j].grad = acc;
                            } else {
                                tape->nodes[j].grad = tensor_clone(grad_out);
                            }
                            break;
                        }
                    }
                }
                break;
            }

            case OP_SUB: {
                // dA = grad_out, dB = -grad_out
                Tensor* inputA = node->inputs[0];
                Tensor* inputB = node->inputs[1];
                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == inputA) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, grad_out);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(grad_out);
                        }
                        break;
                    }
                }
                Tensor* neg_grad = tensor_scale(grad_out, -1.0f);
                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == inputB) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, neg_grad);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(neg_grad);
                        }
                        break;
                    }
                }
                tensor_free(neg_grad);
                break;
            }

            case OP_MUL: {
                // dA = grad_out * B, dB = grad_out * A
                Tensor* A = node->inputs[0];
                Tensor* B = node->inputs[1];
                Tensor* grad_a = tensor_mul(grad_out, B);
                Tensor* grad_b = tensor_mul(grad_out, A);
                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == A) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, grad_a);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(grad_a);
                        }
                        break;
                    }
                }
                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == B) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, grad_b);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(grad_b);
                        }
                        break;
                    }
                }
                tensor_free(grad_a);
                tensor_free(grad_b);
                break;
            }

            case OP_MATMUL: {
                // C = A @ B
                // dA = grad_out @ B^T
                // dB = A^T @ grad_out
                Tensor* A = node->inputs[0];
                Tensor* B = node->inputs[1];

                Tensor* Bt = tensor_transpose(B);
                Tensor* grad_a = tensor_matmul(grad_out, Bt);
                tensor_free(Bt);

                Tensor* At = tensor_transpose(A);
                Tensor* grad_b = tensor_matmul(At, grad_out);
                tensor_free(At);

                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == A) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, grad_a);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(grad_a);
                        }
                        break;
                    }
                }
                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == B) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, grad_b);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(grad_b);
                        }
                        break;
                    }
                }
                tensor_free(grad_a);
                tensor_free(grad_b);
                break;
            }

            case OP_RELU: {
                // dX = grad_out * (X > 0)
                Tensor* input = node->inputs[0];
                Tensor* grad_input = tensor_create(grad_out->shape, grad_out->ndim);
                for (int64_t i = 0; i < grad_out->size; i++) {
                    grad_input->data[i] = input->data[i] > 0.0f ? grad_out->data[i] : 0.0f;
                }
                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == input) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, grad_input);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(grad_input);
                        }
                        break;
                    }
                }
                tensor_free(grad_input);
                break;
            }

            case OP_SOFTMAX: {
                // Softmax backward is complex; typically handled via CE loss directly
                // For standalone softmax: dX_i = S_i * (dY_i - sum(dY * S))
                Tensor* S = node->output;
                Tensor* grad_input = tensor_create(grad_out->shape, grad_out->ndim);

                if (S->ndim == 1) {
                    float dot = 0.0f;
                    for (int64_t i = 0; i < S->size; i++) {
                        dot += grad_out->data[i] * S->data[i];
                    }
                    for (int64_t i = 0; i < S->size; i++) {
                        grad_input->data[i] = S->data[i] * (grad_out->data[i] - dot);
                    }
                } else if (S->ndim == 2) {
                    int64_t rows = S->shape[0];
                    int64_t cols = S->shape[1];
                    for (int64_t r = 0; r < rows; r++) {
                        float dot = 0.0f;
                        for (int64_t c = 0; c < cols; c++) {
                            dot += grad_out->data[r * cols + c] * S->data[r * cols + c];
                        }
                        for (int64_t c = 0; c < cols; c++) {
                            grad_input->data[r * cols + c] =
                                S->data[r * cols + c] * (grad_out->data[r * cols + c] - dot);
                        }
                    }
                }

                Tensor* input = node->inputs[0];
                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == input) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, grad_input);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(grad_input);
                        }
                        break;
                    }
                }
                tensor_free(grad_input);
                break;
            }

            case OP_SCALE: {
                // dA = grad_out * scalar (scalar stored in saved_tensors[0]->data[0])
                Tensor* input = node->inputs[0];
                float scalar = node->saved_tensors[0] ? node->saved_tensors[0]->data[0] : 1.0f;
                Tensor* grad_input = tensor_scale(grad_out, scalar);
                for (int32_t j = idx - 1; j >= 0; j--) {
                    if (tape->nodes[j].output == input) {
                        if (tape->nodes[j].grad) {
                            Tensor* acc = tensor_add(tape->nodes[j].grad, grad_input);
                            tensor_free(tape->nodes[j].grad);
                            tape->nodes[j].grad = acc;
                        } else {
                            tape->nodes[j].grad = tensor_clone(grad_input);
                        }
                        break;
                    }
                }
                tensor_free(grad_input);
                break;
            }

            case OP_LINEAR:
            case OP_LOSS_CE:
                // Already handled above or handled as composite ops
                break;
        }
    }
}

void tape_zero_grads(Tape* tape) {
    for (int32_t i = 0; i < tape->count; i++) {
        if (tape->nodes[i].grad) {
            tensor_free(tape->nodes[i].grad);
            tape->nodes[i].grad = NULL;
        }
    }
    tape->count = 0;
}

void tape_free(Tape* tape) {
    if (!tape) return;
    for (int32_t i = 0; i < tape->count; i++) {
        if (tape->nodes[i].grad) {
            tensor_free(tape->nodes[i].grad);
        }
    }
    free(tape->nodes);
    free(tape);
}

// ── Autograd-aware ops ───────────────────────────────────

Tensor* ag_add(Tape* tape, Tensor* a, Tensor* b) {
    Tensor* out = tensor_add(a, b);
    Tensor* inputs[2] = {a, b};
    tape_record(tape, OP_ADD, inputs, 2, out);
    return out;
}

Tensor* ag_mul(Tape* tape, Tensor* a, Tensor* b) {
    Tensor* out = tensor_mul(a, b);
    Tensor* inputs[2] = {a, b};
    tape_record(tape, OP_MUL, inputs, 2, out);
    return out;
}

Tensor* ag_matmul(Tape* tape, Tensor* a, Tensor* b) {
    Tensor* out = tensor_matmul(a, b);
    Tensor* inputs[2] = {a, b};
    tape_record(tape, OP_MATMUL, inputs, 2, out);
    return out;
}

Tensor* ag_relu(Tape* tape, Tensor* input) {
    Tensor* out = tensor_relu(input);
    Tensor* inputs[1] = {input};
    tape_record(tape, OP_RELU, inputs, 1, out);
    return out;
}

Tensor* ag_softmax(Tape* tape, Tensor* input) {
    Tensor* out = tensor_softmax(input);
    Tensor* inputs[1] = {input};
    tape_record(tape, OP_SOFTMAX, inputs, 1, out);
    return out;
}

float ag_cross_entropy(Tape* tape, Tensor* logits, const int64_t* labels, int64_t batch_size) {
    // Compute softmax
    Tensor* sm = tensor_softmax(logits);

    // Compute cross-entropy loss
    int64_t num_classes = logits->shape[logits->ndim - 1];
    float loss = 0.0f;
    for (int64_t b = 0; b < batch_size; b++) {
        int64_t offset = (logits->ndim == 2) ? b * num_classes : 0;
        float p = sm->data[offset + labels[b]];
        if (p < 1e-7f) p = 1e-7f;
        loss -= logf(p);
    }
    loss /= (float)batch_size;

    // Record on tape for backward
    tape_record_ce(tape, logits, sm, labels, batch_size);

    return loss;
}
