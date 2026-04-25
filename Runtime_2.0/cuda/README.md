# CUDA — Runtime 2.0 💀🦈

Kernels GPU directos, sin framework (sin PyTorch, sin cuDNN).

- `gpu_matmul` — tiled matmul con shared memory (16x16 tiles)
- `gpu_relu` — ReLU paralelo
- `gpu_add` — element-wise add
- `gpu_info` — info del device

## Build
```cmd
build.cmd
```

## Requisitos
- CUDA 13.1: `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.1`

## Estado: ✅ Kernels listos
