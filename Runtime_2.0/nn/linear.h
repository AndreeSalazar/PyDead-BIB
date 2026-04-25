// ============================================================
// PyDead-BIB Runtime 2.0 — Neural Network Layers 💀🦈
// ============================================================

#ifndef PYDEAD_LINEAR_H
#define PYDEAD_LINEAR_H

#include "../tensor/tensor.h"

typedef struct {
    Tensor* weight;
    Tensor* bias;
    int64_t in_features;
    int64_t out_features;
} Linear;

Linear* linear_create(int64_t in_features, int64_t out_features, uint64_t seed);
Tensor* linear_forward(const Linear* layer, const Tensor* input);
void    linear_free(Linear* layer);

typedef struct {
    Linear* layers;
    int32_t num_layers;
} MLP;

MLP*    mlp_create(int64_t input_size, const int64_t* hidden_sizes, int32_t num_hidden, int64_t output_size, uint64_t seed);
Tensor* mlp_forward(const MLP* model, const Tensor* input);
void    mlp_free(MLP* model);

#endif
