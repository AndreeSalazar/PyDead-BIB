// ============================================================
// PyDead-BIB Runtime 2.0 — Tensor Tests 💀🦈
// ============================================================

#include "tensor.h"
#include <stdio.h>
#include <math.h>

#define ASSERT(cond, msg) do { \
    if (!(cond)) { printf("  FAIL: %s\n", msg); fails++; } \
    else { printf("  PASS: %s\n", msg); passes++; } \
} while(0)

int main(void) {
    int passes = 0, fails = 0;

    printf("======================================================\n");
    printf("  PyDead-BIB Runtime 2.0 — Tensor Tests\n");
    printf("======================================================\n\n");

    // Test 1: Create tensor
    int64_t shape1[] = {3};
    float data1[] = {1.0f, 2.0f, 3.0f};
    Tensor* t1 = tensor_create_from_data(data1, shape1, 1);
    ASSERT(t1 != NULL, "tensor_create_from_data");
    ASSERT(t1->size == 3, "tensor size == 3");
    ASSERT(t1->ndim == 1, "tensor ndim == 1");

    // Test 2: Zeros
    int64_t shape2[] = {2, 3};
    Tensor* t2 = tensor_zeros(shape2, 2);
    ASSERT(t2 != NULL, "tensor_zeros [2,3]");
    ASSERT(t2->size == 6, "zeros size == 6");
    ASSERT(t2->data[0] == 0.0f, "zeros data[0] == 0");

    // Test 3: Ones
    Tensor* t3 = tensor_ones(shape2, 2);
    ASSERT(t3->data[5] == 1.0f, "ones data[5] == 1");

    // Test 4: Element-wise add
    int64_t shape_v[] = {4};
    float a_data[] = {1.0f, 2.0f, 3.0f, 4.0f};
    float b_data[] = {5.0f, 6.0f, 7.0f, 8.0f};
    Tensor* ta = tensor_create_from_data(a_data, shape_v, 1);
    Tensor* tb = tensor_create_from_data(b_data, shape_v, 1);
    Tensor* tc = tensor_add(ta, tb);
    ASSERT(tc->data[0] == 6.0f, "add [0] == 6");
    ASSERT(tc->data[3] == 12.0f, "add [3] == 12");

    // Test 5: Element-wise mul
    Tensor* tm = tensor_mul(ta, tb);
    ASSERT(tm->data[0] == 5.0f, "mul [0] == 5");
    ASSERT(tm->data[1] == 12.0f, "mul [1] == 12");

    // Test 6: Matmul [2,3] x [3,2] = [2,2]
    int64_t shA[] = {2, 3};
    int64_t shB[] = {3, 2};
    float dA[] = {1, 2, 3, 4, 5, 6};
    float dB[] = {7, 8, 9, 10, 11, 12};
    Tensor* mA = tensor_create_from_data(dA, shA, 2);
    Tensor* mB = tensor_create_from_data(dB, shB, 2);
    Tensor* mC = tensor_matmul(mA, mB);
    ASSERT(mC->shape[0] == 2, "matmul shape[0] == 2");
    ASSERT(mC->shape[1] == 2, "matmul shape[1] == 2");
    ASSERT(fabsf(mC->data[0] - 58.0f) < 0.01f, "matmul [0,0] == 58");
    ASSERT(fabsf(mC->data[1] - 64.0f) < 0.01f, "matmul [0,1] == 64");
    ASSERT(fabsf(mC->data[2] - 139.0f) < 0.01f, "matmul [1,0] == 139");
    ASSERT(fabsf(mC->data[3] - 154.0f) < 0.01f, "matmul [1,1] == 154");

    // Test 7: ReLU
    int64_t shR[] = {5};
    float dR[] = {-2.0f, -1.0f, 0.0f, 1.0f, 2.0f};
    Tensor* tR = tensor_create_from_data(dR, shR, 1);
    Tensor* tRelu = tensor_relu(tR);
    ASSERT(tRelu->data[0] == 0.0f, "relu(-2) == 0");
    ASSERT(tRelu->data[2] == 0.0f, "relu(0) == 0");
    ASSERT(tRelu->data[4] == 2.0f, "relu(2) == 2");

    // Test 8: Softmax
    int64_t shS[] = {3};
    float dS[] = {1.0f, 2.0f, 3.0f};
    Tensor* tS = tensor_create_from_data(dS, shS, 1);
    Tensor* tSoft = tensor_softmax(tS);
    float soft_sum = tensor_sum(tSoft);
    ASSERT(fabsf(soft_sum - 1.0f) < 0.001f, "softmax sum == 1.0");

    // Test 9: Sum/Mean
    float sum1 = tensor_sum(t1);
    ASSERT(fabsf(sum1 - 6.0f) < 0.01f, "sum [1,2,3] == 6");
    float mean1 = tensor_mean(t1);
    ASSERT(fabsf(mean1 - 2.0f) < 0.01f, "mean [1,2,3] == 2");

    // Test 10: Transpose
    Tensor* mT = tensor_transpose(mA);
    ASSERT(mT->shape[0] == 3, "transpose shape[0] == 3");
    ASSERT(mT->shape[1] == 2, "transpose shape[1] == 2");
    ASSERT(mT->data[0] == 1.0f, "transpose [0,0] == 1");
    ASSERT(mT->data[1] == 4.0f, "transpose [0,1] == 4");

    // Test 11: Scale
    Tensor* tScaled = tensor_scale(ta, 2.0f);
    ASSERT(tScaled->data[0] == 2.0f, "scale 1*2 == 2");
    ASSERT(tScaled->data[3] == 8.0f, "scale 4*2 == 8");

    // Test 12: SIMD matmul 16x16
    int64_t shBig[] = {16, 16};
    Tensor* bigA = tensor_ones(shBig, 2);
    Tensor* bigB = tensor_ones(shBig, 2);
    Tensor* bigC = tensor_matmul(bigA, bigB);
    ASSERT(fabsf(bigC->data[0] - 16.0f) < 0.01f, "matmul 16x16 ones = 16");

    // Cleanup
    tensor_free(t1); tensor_free(t2); tensor_free(t3);
    tensor_free(ta); tensor_free(tb); tensor_free(tc); tensor_free(tm);
    tensor_free(mA); tensor_free(mB); tensor_free(mC); tensor_free(mT);
    tensor_free(tR); tensor_free(tRelu);
    tensor_free(tS); tensor_free(tSoft);
    tensor_free(tScaled);
    tensor_free(bigA); tensor_free(bigB); tensor_free(bigC);

    printf("\n======================================================\n");
    printf("  Results: %d PASS, %d FAIL\n", passes, fails);
    printf("======================================================\n");

    return fails > 0 ? 1 : 0;
}
