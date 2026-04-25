# Tensor — Runtime 2.0 💀🦈

Tipo fundamental para IA. Memoria continua, aligned 32 bytes para AVX2.

## API
- `tensor_create`, `tensor_zeros`, `tensor_ones`, `tensor_clone`, `tensor_free`
- `tensor_add/sub/mul/div` — element-wise con SIMD AVX2
- `tensor_matmul` — cache-friendly i-k-j con FMA
- `tensor_relu/sigmoid/tanh_act/softmax` — activaciones IA
- `tensor_sum/max/min/mean` — reducciones

## Build
```cmd
cd Runtime_2.0\c_abi
build.cmd
```

## Estado: ✅ Implementado
