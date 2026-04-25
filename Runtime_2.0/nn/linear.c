// ============================================================
// PyDead-BIB Runtime 2.0 — Neural Network Layers 💀🦈
// ============================================================

#include "linear.h"
#include "../math_ops/nn_ops.h"
#include <stdlib.h>
#include <math.h>

Linear* linear_create(int64_t in_features, int64_t out_features, uint64_t seed) {
    Linear* l = (Linear*)malloc(sizeof(Linear));
    l->in_features = in_features;
    l->out_features = out_features;

    int64_t w_shape[] = {out_features, in_features};
    l->weight = tensor_create(w_shape, 2);

    int64_t b_shape[] = {out_features};
    l->bias = tensor_zeros(b_shape, 1);

    float scale = sqrtf(2.0f / (float)in_features);
    nn_fill_random(l->weight, -scale, scale, seed);

    return l;
}

Tensor* linear_forward(const Linear* layer, const Tensor* input) {
    Tensor* wT = tensor_transpose(layer->weight);
    Tensor* out;

    if (input->ndim == 1) {
        int64_t batch_shape[] = {1, input->size};
        Tensor* x = tensor_reshape(input, batch_shape, 2);
        out = tensor_matmul(x, wT);
        for (int64_t j = 0; j < layer->out_features; j++) {
            out->data[j] += layer->bias->data[j];
        }
        tensor_free(x);
    } else {
        out = tensor_matmul(input, wT);
        int64_t batch = input->shape[0];
        for (int64_t b = 0; b < batch; b++) {
            for (int64_t j = 0; j < layer->out_features; j++) {
                out->data[b * layer->out_features + j] += layer->bias->data[j];
            }
        }
    }

    tensor_free(wT);
    return out;
}

void linear_free(Linear* layer) {
    if (!layer) return;
    tensor_free(layer->weight);
    tensor_free(layer->bias);
    free(layer);
}

MLP* mlp_create(int64_t input_size, const int64_t* hidden_sizes, int32_t num_hidden, int64_t output_size, uint64_t seed) {
    MLP* model = (MLP*)malloc(sizeof(MLP));
    model->num_layers = num_hidden + 1;
    model->layers = (Linear*)malloc(model->num_layers * sizeof(Linear));

    int64_t prev_size = input_size;
    for (int32_t i = 0; i < num_hidden; i++) {
        Linear* l = linear_create(prev_size, hidden_sizes[i], seed + i);
        model->layers[i] = *l;
        free(l);
        prev_size = hidden_sizes[i];
    }
    Linear* out = linear_create(prev_size, output_size, seed + num_hidden);
    model->layers[num_hidden] = *out;
    free(out);

    return model;
}

Tensor* mlp_forward(const MLP* model, const Tensor* input) {
    Tensor* x = tensor_clone(input);

    for (int32_t i = 0; i < model->num_layers - 1; i++) {
        Tensor* y = linear_forward(&model->layers[i], x);
        tensor_free(x);
        x = tensor_relu(y);
        tensor_free(y);
    }

    Tensor* out = linear_forward(&model->layers[model->num_layers - 1], x);
    tensor_free(x);
    return out;
}

void mlp_free(MLP* model) {
    if (!model) return;
    for (int32_t i = 0; i < model->num_layers; i++) {
        tensor_free(model->layers[i].weight);
        tensor_free(model->layers[i].bias);
    }
    free(model->layers);
    free(model);
}
