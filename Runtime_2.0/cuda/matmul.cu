// ============================================================
// PyDead-BIB Runtime 2.0 — CUDA Matmul Kernel 💀🦈
// GPU compute — Sin framework — Kernel directo
// ============================================================

#include <cuda_runtime.h>
#include <stdio.h>

#define BLOCK_SIZE 16

__global__ void matmul_kernel(const float* A, const float* B, float* C,
                               int M, int K, int N) {
    __shared__ float sA[BLOCK_SIZE][BLOCK_SIZE];
    __shared__ float sB[BLOCK_SIZE][BLOCK_SIZE];

    int row = blockIdx.y * BLOCK_SIZE + threadIdx.y;
    int col = blockIdx.x * BLOCK_SIZE + threadIdx.x;
    float sum = 0.0f;

    for (int tile = 0; tile < (K + BLOCK_SIZE - 1) / BLOCK_SIZE; tile++) {
        int tileK = tile * BLOCK_SIZE;

        if (row < M && tileK + threadIdx.x < K)
            sA[threadIdx.y][threadIdx.x] = A[row * K + tileK + threadIdx.x];
        else
            sA[threadIdx.y][threadIdx.x] = 0.0f;

        if (tileK + threadIdx.y < K && col < N)
            sB[threadIdx.y][threadIdx.x] = B[(tileK + threadIdx.y) * N + col];
        else
            sB[threadIdx.y][threadIdx.x] = 0.0f;

        __syncthreads();

        for (int k = 0; k < BLOCK_SIZE; k++) {
            sum += sA[threadIdx.y][k] * sB[k][threadIdx.x];
        }
        __syncthreads();
    }

    if (row < M && col < N) {
        C[row * N + col] = sum;
    }
}

__global__ void relu_kernel(const float* input, float* output, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        output[idx] = input[idx] > 0.0f ? input[idx] : 0.0f;
    }
}

__global__ void add_kernel(const float* a, const float* b, float* c, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        c[idx] = a[idx] + b[idx];
    }
}

// ── Host API (extern "C" for linking) ─────────────────────

extern "C" {

int gpu_matmul(const float* h_A, const float* h_B, float* h_C,
               int M, int K, int N) {
    float *d_A, *d_B, *d_C;
    size_t sA = M * K * sizeof(float);
    size_t sB = K * N * sizeof(float);
    size_t sC = M * N * sizeof(float);

    cudaMalloc(&d_A, sA);
    cudaMalloc(&d_B, sB);
    cudaMalloc(&d_C, sC);

    cudaMemcpy(d_A, h_A, sA, cudaMemcpyHostToDevice);
    cudaMemcpy(d_B, h_B, sB, cudaMemcpyHostToDevice);

    dim3 block(BLOCK_SIZE, BLOCK_SIZE);
    dim3 grid((N + BLOCK_SIZE - 1) / BLOCK_SIZE,
              (M + BLOCK_SIZE - 1) / BLOCK_SIZE);

    matmul_kernel<<<grid, block>>>(d_A, d_B, d_C, M, K, N);
    cudaDeviceSynchronize();

    cudaMemcpy(h_C, d_C, sC, cudaMemcpyDeviceToHost);

    cudaFree(d_A);
    cudaFree(d_B);
    cudaFree(d_C);
    return 0;
}

int gpu_relu(const float* h_input, float* h_output, int size) {
    float *d_in, *d_out;
    size_t bytes = size * sizeof(float);

    cudaMalloc(&d_in, bytes);
    cudaMalloc(&d_out, bytes);
    cudaMemcpy(d_in, h_input, bytes, cudaMemcpyHostToDevice);

    int threads = 256;
    int blocks = (size + threads - 1) / threads;
    relu_kernel<<<blocks, threads>>>(d_in, d_out, size);
    cudaDeviceSynchronize();

    cudaMemcpy(h_output, d_out, bytes, cudaMemcpyDeviceToHost);
    cudaFree(d_in);
    cudaFree(d_out);
    return 0;
}

int gpu_add(const float* h_a, const float* h_b, float* h_c, int size) {
    float *d_a, *d_b, *d_c;
    size_t bytes = size * sizeof(float);

    cudaMalloc(&d_a, bytes);
    cudaMalloc(&d_b, bytes);
    cudaMalloc(&d_c, bytes);

    cudaMemcpy(d_a, h_a, bytes, cudaMemcpyHostToDevice);
    cudaMemcpy(d_b, h_b, bytes, cudaMemcpyHostToDevice);

    int threads = 256;
    int blocks = (size + threads - 1) / threads;
    add_kernel<<<blocks, threads>>>(d_a, d_b, d_c, size);
    cudaDeviceSynchronize();

    cudaMemcpy(h_c, d_c, bytes, cudaMemcpyDeviceToHost);
    cudaFree(d_a);
    cudaFree(d_b);
    cudaFree(d_c);
    return 0;
}

void gpu_info(void) {
    int count = 0;
    cudaGetDeviceCount(&count);
    printf("[PyDead-BIB GPU] Devices: %d\n", count);
    for (int i = 0; i < count; i++) {
        cudaDeviceProp prop;
        cudaGetDeviceProperties(&prop, i);
        printf("  [%d] %s — %.0f MB — SM %d.%d — %d SMs\n",
            i, prop.name,
            (float)prop.totalGlobalMem / (1024*1024),
            prop.major, prop.minor,
            prop.multiProcessorCount);
    }
}

} // extern "C"
