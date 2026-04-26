// ============================================================
// PyDead-BIB Runtime 2.0 — Full Integration Test 💀🦈
// Tests: Tensor, NN Ops, Linear/MLP, Autograd, Optimizer, I/O
// ============================================================

#include "../tensor/tensor.h"
#include "../math_ops/nn_ops.h"
#include "../nn/linear.h"
#include "../autograd/autograd.h"
#include "../optim/optim.h"
#include "../io/tensor_io.h"
#include "../memory/arena.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#define ASSERT(cond, msg) do { \
    if (!(cond)) { printf("  FAIL: %s\n", msg); (*fails)++; } \
    else { printf("  PASS: %s\n", msg); (*passes)++; } \
} while(0)

static void test_tensor_basic(int* passes, int* fails) {
    printf("\n--- Tensor Basic ---\n");

    int64_t sh[] = {2, 3};
    Tensor* t = tensor_ones(sh, 2);
    ASSERT(t != NULL, "tensor_ones [2,3]");
    ASSERT(t->size == 6, "size == 6");
    ASSERT(t->data[0] == 1.0f, "data[0] == 1");

    Tensor* z = tensor_zeros(sh, 2);
    ASSERT(z->data[0] == 0.0f, "zeros[0] == 0");

    Tensor* c = tensor_clone(t);
    ASSERT(c->data[5] == 1.0f, "clone[5] == 1");

    tensor_free(t);
    tensor_free(z);
    tensor_free(c);
}

static void test_tensor_ops(int* passes, int* fails) {
    printf("\n--- Tensor Ops (SIMD) ---\n");

    int64_t sh[] = {8};
    float a_data[] = {1, 2, 3, 4, 5, 6, 7, 8};
    float b_data[] = {8, 7, 6, 5, 4, 3, 2, 1};
    Tensor* a = tensor_create_from_data(a_data, sh, 1);
    Tensor* b = tensor_create_from_data(b_data, sh, 1);

    Tensor* sum_t = tensor_add(a, b);
    ASSERT(sum_t->data[0] == 9.0f, "add[0] == 9");
    ASSERT(sum_t->data[7] == 9.0f, "add[7] == 9");

    Tensor* sub_t = tensor_sub(a, b);
    ASSERT(sub_t->data[0] == -7.0f, "sub[0] == -7");

    Tensor* mul_t = tensor_mul(a, b);
    ASSERT(mul_t->data[0] == 8.0f, "mul[0] == 8");

    Tensor* div_t = tensor_div(a, b);
    ASSERT(fabsf(div_t->data[0] - 0.125f) < 0.001f, "div[0] == 0.125");

    Tensor* sc = tensor_scale(a, 3.0f);
    ASSERT(sc->data[2] == 9.0f, "scale 3*3 == 9");

    tensor_free(a); tensor_free(b);
    tensor_free(sum_t); tensor_free(sub_t);
    tensor_free(mul_t); tensor_free(div_t);
    tensor_free(sc);
}

static void test_matmul(int* passes, int* fails) {
    printf("\n--- Matmul ---\n");

    int64_t shA[] = {2, 3};
    int64_t shB[] = {3, 2};
    float dA[] = {1, 2, 3, 4, 5, 6};
    float dB[] = {7, 8, 9, 10, 11, 12};
    Tensor* A = tensor_create_from_data(dA, shA, 2);
    Tensor* B = tensor_create_from_data(dB, shB, 2);
    Tensor* C = tensor_matmul(A, B);

    ASSERT(C->shape[0] == 2, "matmul shape[0] == 2");
    ASSERT(C->shape[1] == 2, "matmul shape[1] == 2");
    ASSERT(fabsf(C->data[0] - 58.0f) < 0.01f, "matmul [0,0] == 58");
    ASSERT(fabsf(C->data[3] - 154.0f) < 0.01f, "matmul [1,1] == 154");

    Tensor* T = tensor_transpose(A);
    ASSERT(T->shape[0] == 3, "transpose [0] == 3");
    ASSERT(T->shape[1] == 2, "transpose [1] == 2");
    ASSERT(T->data[1] == 4.0f, "transpose [0,1] == 4");

    tensor_free(A); tensor_free(B);
    tensor_free(C); tensor_free(T);
}

static void test_activations(int* passes, int* fails) {
    printf("\n--- Activations ---\n");

    int64_t sh[] = {5};
    float d[] = {-2.0f, -1.0f, 0.0f, 1.0f, 2.0f};
    Tensor* t = tensor_create_from_data(d, sh, 1);

    Tensor* r = tensor_relu(t);
    ASSERT(r->data[0] == 0.0f, "relu(-2) == 0");
    ASSERT(r->data[4] == 2.0f, "relu(2) == 2");

    Tensor* s = tensor_sigmoid(t);
    ASSERT(fabsf(s->data[2] - 0.5f) < 0.001f, "sigmoid(0) == 0.5");

    Tensor* th = tensor_tanh_act(t);
    ASSERT(fabsf(th->data[2]) < 0.001f, "tanh(0) == 0");

    int64_t sh3[] = {3};
    float d3[] = {1.0f, 2.0f, 3.0f};
    Tensor* t3 = tensor_create_from_data(d3, sh3, 1);
    Tensor* sm = tensor_softmax(t3);
    float sm_sum = tensor_sum(sm);
    ASSERT(fabsf(sm_sum - 1.0f) < 0.001f, "softmax sum == 1.0");

    tensor_free(t); tensor_free(r);
    tensor_free(s); tensor_free(th);
    tensor_free(t3); tensor_free(sm);
}

static void test_nn_ops(int* passes, int* fails) {
    printf("\n--- NN Ops ---\n");

    int64_t sh[] = {4};
    Tensor* t = tensor_create(sh, 1);
    nn_fill_random(t, -1.0f, 1.0f, 42);
    ASSERT(t->data[0] >= -1.0f && t->data[0] <= 1.0f, "random fill in range");

    Tensor* g = nn_gelu(t);
    ASSERT(g != NULL, "gelu computed");

    Tensor* lr = nn_leaky_relu(t, 0.01f);
    ASSERT(lr != NULL, "leaky_relu computed");

    Tensor* si = nn_silu(t);
    ASSERT(si != NULL, "silu computed");

    int64_t sh2[] = {2, 4};
    Tensor* pred = tensor_ones(sh2, 2);
    Tensor* tgt = tensor_zeros(sh2, 2);
    float mse = nn_mse_loss(pred, tgt);
    ASSERT(fabsf(mse - 1.0f) < 0.01f, "mse_loss(ones, zeros) == 1.0");

    Tensor* ln = nn_layer_norm(pred, 1e-5f);
    ASSERT(ln != NULL, "layer_norm computed");

    tensor_free(t); tensor_free(g);
    tensor_free(lr); tensor_free(si);
    tensor_free(pred); tensor_free(tgt);
    tensor_free(ln);
}

static void test_linear_mlp(int* passes, int* fails) {
    printf("\n--- Linear / MLP ---\n");

    Linear* l = linear_create(4, 3, 42);
    ASSERT(l != NULL, "linear_create(4, 3)");
    ASSERT(l->weight->shape[0] == 3, "weight shape[0] == 3");
    ASSERT(l->weight->shape[1] == 4, "weight shape[1] == 4");

    int64_t sh[] = {4};
    float d[] = {1.0f, 2.0f, 3.0f, 4.0f};
    Tensor* x = tensor_create_from_data(d, sh, 1);
    Tensor* y = linear_forward(l, x);
    ASSERT(y->shape[y->ndim - 1] == 3, "linear output features == 3");

    int64_t hidden[] = {8, 4};
    MLP* mlp = mlp_create(4, hidden, 2, 2, 123);
    ASSERT(mlp != NULL, "mlp_create(4, [8,4], 2)");
    ASSERT(mlp->num_layers == 3, "mlp layers == 3");

    Tensor* out = mlp_forward(mlp, x);
    ASSERT(out->shape[out->ndim - 1] == 2, "mlp output == 2");

    tensor_free(x); tensor_free(y); tensor_free(out);
    linear_free(l);
    mlp_free(mlp);
}

static void test_autograd(int* passes, int* fails) {
    printf("\n--- Autograd ---\n");

    Tape* tape = tape_create(64);
    ASSERT(tape != NULL, "tape_create");

    // Simple: c = a + b, backward
    int64_t sh[] = {3};
    float da[] = {1.0f, 2.0f, 3.0f};
    float db[] = {4.0f, 5.0f, 6.0f};
    Tensor* a = tensor_create_from_data(da, sh, 1);
    Tensor* b = tensor_create_from_data(db, sh, 1);

    // Record a as a leaf
    Tensor* inputs_a[] = {a};
    tape_record(tape, OP_ADD, inputs_a, 0, a);  // leaf node

    Tensor* inputs_b[] = {b};
    tape_record(tape, OP_ADD, inputs_b, 0, b);  // leaf node

    Tensor* c = ag_add(tape, a, b);
    ASSERT(fabsf(c->data[0] - 5.0f) < 0.01f, "ag_add [0] == 5");

    // Test relu
    Tensor* r = ag_relu(tape, c);
    ASSERT(r->data[0] == 5.0f, "ag_relu(5) == 5");

    tensor_free(a); tensor_free(b);
    tensor_free(c); tensor_free(r);
    tape_free(tape);

    // Test cross-entropy + backward
    Tape* tape2 = tape_create(64);

    int64_t sh_logits[] = {2, 3};
    float logit_data[] = {2.0f, 1.0f, 0.1f,
                          0.5f, 2.5f, 0.3f};
    Tensor* logits = tensor_create_from_data(logit_data, sh_logits, 2);

    // Record logits as leaf
    Tensor* inp_l[] = {logits};
    tape_record(tape2, OP_ADD, inp_l, 0, logits);

    int64_t labels[] = {0, 1};
    float loss = ag_cross_entropy(tape2, logits, labels, 2);
    ASSERT(loss > 0.0f, "cross_entropy loss > 0");
    ASSERT(loss < 5.0f, "cross_entropy loss < 5 (reasonable)");

    tape_backward(tape2);

    // Check that gradient was computed for the loss node
    int has_grad = 0;
    for (int32_t i = 0; i < tape2->count; i++) {
        if (tape2->nodes[i].grad != NULL) has_grad = 1;
    }
    ASSERT(has_grad, "backward produced gradients");

    tensor_free(logits);
    tape_free(tape2);
}

static void test_optimizer(int* passes, int* fails) {
    printf("\n--- Optimizer ---\n");

    // SGD test: param = [1, 2, 3], grad = [0.1, 0.2, 0.3], lr = 0.1
    int64_t sh[] = {3};
    float p_data[] = {1.0f, 2.0f, 3.0f};
    float g_data[] = {0.1f, 0.2f, 0.3f};
    Tensor* param = tensor_create_from_data(p_data, sh, 1);
    Tensor* grad = tensor_create_from_data(g_data, sh, 1);

    SGD* sgd = sgd_create(0.1f, 0.0f, 0.0f, 1);
    Tensor* params[] = {param};
    Tensor* grads[] = {grad};
    sgd_step(sgd, params, grads, 1);

    // param[0] should be 1.0 - 0.1*0.1 = 0.99
    ASSERT(fabsf(param->data[0] - 0.99f) < 0.001f, "SGD step: param[0] == 0.99");
    ASSERT(fabsf(param->data[1] - 1.98f) < 0.001f, "SGD step: param[1] == 1.98");

    sgd_free(sgd);

    // Adam test
    param->data[0] = 1.0f; param->data[1] = 2.0f; param->data[2] = 3.0f;
    grad->data[0] = 0.1f; grad->data[1] = 0.2f; grad->data[2] = 0.3f;

    Adam* adam = adam_create(0.001f, 0.9f, 0.999f, 1e-8f, 0.0f, 1);
    adam_step(adam, params, grads, 1);
    ASSERT(param->data[0] < 1.0f, "Adam step: param[0] decreased");
    ASSERT(param->data[0] > 0.9f, "Adam step: param[0] reasonable");

    adam_free(adam);
    tensor_free(param);
    tensor_free(grad);
}

static void test_io(int* passes, int* fails) {
    printf("\n--- Tensor I/O ---\n");

    int64_t sh[] = {2, 3};
    float d[] = {1.0f, 2.0f, 3.0f, 4.0f, 5.0f, 6.0f};
    Tensor* t = tensor_create_from_data(d, sh, 2);

    // Save and load single tensor
    int ret = tensor_save(t, "test_tensor.pdb");
    ASSERT(ret == 0, "tensor_save OK");

    Tensor* loaded = tensor_load("test_tensor.pdb");
    ASSERT(loaded != NULL, "tensor_load OK");
    ASSERT(loaded->ndim == 2, "loaded ndim == 2");
    ASSERT(loaded->shape[0] == 2, "loaded shape[0] == 2");
    ASSERT(loaded->shape[1] == 3, "loaded shape[1] == 3");
    ASSERT(fabsf(loaded->data[5] - 6.0f) < 0.01f, "loaded data[5] == 6");

    // Save and load multiple tensors
    int64_t sh2[] = {4};
    float d2[] = {10.0f, 20.0f, 30.0f, 40.0f};
    Tensor* t2 = tensor_create_from_data(d2, sh2, 1);

    Tensor* tensors[] = {t, t2};
    const char* names[] = {"weight", "bias"};
    ret = tensor_save_all(tensors, names, 2, "test_model.pdb");
    ASSERT(ret == 0, "tensor_save_all OK");

    Tensor** loaded_tensors;
    char** loaded_names;
    int32_t count;
    ret = tensor_load_all("test_model.pdb", &loaded_tensors, &loaded_names, &count);
    ASSERT(ret == 0, "tensor_load_all OK");
    ASSERT(count == 2, "loaded count == 2");
    ASSERT(strcmp(loaded_names[0], "weight") == 0, "loaded name[0] == weight");
    ASSERT(loaded_tensors[0]->size == 6, "loaded[0] size == 6");
    ASSERT(loaded_tensors[1]->size == 4, "loaded[1] size == 4");

    // Cleanup loaded
    for (int32_t i = 0; i < count; i++) {
        tensor_free(loaded_tensors[i]);
        free(loaded_names[i]);
    }
    free(loaded_tensors);
    free(loaded_names);

    // Raw save/load
    ret = tensor_save_raw(t, "test_raw.bin");
    ASSERT(ret == 0, "tensor_save_raw OK");
    Tensor* raw = tensor_load_raw("test_raw.bin", sh, 2);
    ASSERT(raw != NULL, "tensor_load_raw OK");
    ASSERT(fabsf(raw->data[0] - 1.0f) < 0.01f, "raw data[0] == 1");

    tensor_free(t); tensor_free(t2);
    tensor_free(loaded); tensor_free(raw);

    // Cleanup temp files
    remove("test_tensor.pdb");
    remove("test_model.pdb");
    remove("test_raw.bin");
}

static void test_memory_arena(int* passes, int* fails) {
    printf("\n--- Memory Arena ---\n");

    Arena* arena = arena_create(1024 * 1024);  // 1 MB
    ASSERT(arena != NULL, "arena_create 1MB");
    ASSERT(arena_used(arena) == 0, "arena used == 0");

    void* p1 = arena_alloc(arena, 256, 32);
    ASSERT(p1 != NULL, "arena_alloc 256B");
    ASSERT(arena_used(arena) > 0, "arena used > 0 after alloc");

    void* p2 = arena_alloc_tensor(arena, 64);
    ASSERT(p2 != NULL, "arena_alloc_tensor 64 floats");

    size_t used = arena_used(arena);
    arena_reset(arena);
    ASSERT(arena_used(arena) == 0, "arena used == 0 after reset");

    (void)used;
    arena_free(arena);
}

static void test_training_loop(int* passes, int* fails) {
    printf("\n--- Training Loop (MLP XOR) ---\n");

    // XOR problem: 4 samples, 2 inputs, 2 outputs (one-hot)
    // [0,0]->0, [0,1]->1, [1,0]->1, [1,1]->0

    Linear* layer1 = linear_create(2, 8, 42);
    Linear* layer2 = linear_create(8, 2, 99);

    float x_data[] = {0, 0,   0, 1,   1, 0,   1, 1};
    int64_t x_shape[] = {4, 2};
    Tensor* X = tensor_create_from_data(x_data, x_shape, 2);

    int64_t labels[] = {0, 1, 1, 0};

    SGD* opt = sgd_create(0.1f, 0.0f, 0.0f, 4);

    float initial_loss = 0.0f;
    float final_loss = 0.0f;

    for (int epoch = 0; epoch < 100; epoch++) {
        // Forward
        Tensor* h1 = linear_forward(layer1, X);
        Tensor* a1 = tensor_relu(h1);
        Tensor* h2 = linear_forward(layer2, a1);

        // Softmax + cross-entropy
        Tensor* sm = tensor_softmax(h2);
        int64_t num_classes = 2;
        float loss = 0.0f;
        for (int64_t b = 0; b < 4; b++) {
            float p = sm->data[b * num_classes + labels[b]];
            if (p < 1e-7f) p = 1e-7f;
            loss -= logf(p);
        }
        loss /= 4.0f;

        if (epoch == 0) initial_loss = loss;
        if (epoch == 99) final_loss = loss;

        // Manual gradient: dLogits = softmax - one_hot
        Tensor* grad_h2 = tensor_clone(sm);
        for (int64_t b = 0; b < 4; b++) {
            grad_h2->data[b * num_classes + labels[b]] -= 1.0f;
        }
        for (int64_t i = 0; i < grad_h2->size; i++) {
            grad_h2->data[i] /= 4.0f;
        }

        // Backward through layer2: dW2 = a1^T @ grad_h2, db2 = sum(grad_h2)
        Tensor* a1T = tensor_transpose(a1);
        Tensor* grad_w2 = tensor_matmul(a1T, grad_h2);
        tensor_free(a1T);

        // grad_bias2 = column sums of grad_h2
        int64_t b2_sh[] = {2};
        Tensor* grad_b2 = tensor_zeros(b2_sh, 1);
        for (int64_t b = 0; b < 4; b++) {
            for (int64_t j = 0; j < 2; j++) {
                grad_b2->data[j] += grad_h2->data[b * 2 + j];
            }
        }

        // grad through relu: grad_a1 = grad_h2 @ W2, masked by relu
        Tensor* w2T = tensor_transpose(layer2->weight);
        Tensor* grad_a1_pre = tensor_matmul(grad_h2, layer2->weight);
        Tensor* grad_h1 = tensor_create(a1->shape, a1->ndim);
        for (int64_t i = 0; i < a1->size; i++) {
            grad_h1->data[i] = h1->data[i] > 0.0f ? grad_a1_pre->data[i] : 0.0f;
        }
        tensor_free(w2T);
        tensor_free(grad_a1_pre);

        // dW1 = X^T @ grad_h1
        Tensor* XT = tensor_transpose(X);
        Tensor* grad_w1 = tensor_matmul(XT, grad_h1);
        tensor_free(XT);

        int64_t b1_sh[] = {8};
        Tensor* grad_b1 = tensor_zeros(b1_sh, 1);
        for (int64_t b = 0; b < 4; b++) {
            for (int64_t j = 0; j < 8; j++) {
                grad_b1->data[j] += grad_h1->data[b * 8 + j];
            }
        }

        // SGD step — need to transpose grads for weight layout [out, in]
        Tensor* grad_w2T = tensor_transpose(grad_w2);
        Tensor* grad_w1T = tensor_transpose(grad_w1);

        Tensor* params[] = {layer1->weight, layer1->bias, layer2->weight, layer2->bias};
        Tensor* grads[] = {grad_w1T, grad_b1, grad_w2T, grad_b2};
        sgd_step(opt, params, grads, 4);

        // Cleanup epoch
        tensor_free(h1); tensor_free(a1); tensor_free(h2); tensor_free(sm);
        tensor_free(grad_h2); tensor_free(grad_w2); tensor_free(grad_b2);
        tensor_free(grad_h1); tensor_free(grad_w1); tensor_free(grad_b1);
        tensor_free(grad_w2T); tensor_free(grad_w1T);
    }

    printf("  Initial loss: %.4f\n", initial_loss);
    printf("  Final loss:   %.4f\n", final_loss);
    ASSERT(final_loss < initial_loss, "training: loss decreased");

    // Check predictions
    Tensor* h1 = linear_forward(layer1, X);
    Tensor* a1 = tensor_relu(h1);
    Tensor* h2 = linear_forward(layer2, a1);
    Tensor* sm = tensor_softmax(h2);

    int correct = 0;
    for (int64_t b = 0; b < 4; b++) {
        int pred = sm->data[b * 2] > sm->data[b * 2 + 1] ? 0 : 1;
        if (pred == labels[b]) correct++;
    }
    printf("  Accuracy: %d/4\n", correct);
    ASSERT(correct >= 3, "training: XOR accuracy >= 3/4");

    tensor_free(h1); tensor_free(a1); tensor_free(h2); tensor_free(sm);
    tensor_free(X);
    linear_free(layer1);
    linear_free(layer2);
    sgd_free(opt);
}

int main(void) {
    int passes = 0, fails = 0;

    printf("======================================================\n");
    printf("  PyDead-BIB Runtime 2.0 — Full Integration Tests\n");
    printf("======================================================\n");

    test_tensor_basic(&passes, &fails);
    test_tensor_ops(&passes, &fails);
    test_matmul(&passes, &fails);
    test_activations(&passes, &fails);
    test_nn_ops(&passes, &fails);
    test_linear_mlp(&passes, &fails);
    test_autograd(&passes, &fails);
    test_optimizer(&passes, &fails);
    test_io(&passes, &fails);
    test_memory_arena(&passes, &fails);
    test_training_loop(&passes, &fails);

    printf("\n======================================================\n");
    printf("  Results: %d PASS, %d FAIL\n", passes, fails);
    printf("======================================================\n");

    return fails > 0 ? 1 : 0;
}
