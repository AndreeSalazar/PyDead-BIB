# Tests — Runtime 2.0 💀🦈

Suite de tests de integración completa.

## test_full.c
Cubre todos los módulos del runtime:
- Tensor basic (create, zeros, ones, clone)
- Tensor ops SIMD (add, sub, mul, div, scale)
- Matmul + transpose
- Activaciones (relu, sigmoid, tanh, softmax)
- NN ops (gelu, leaky_relu, silu, mse_loss, layer_norm)
- Linear / MLP forward
- Autograd (tape, backward, cross-entropy)
- Optimizers (SGD, Adam)
- Tensor I/O (save/load .pdb, raw)
- Memory arena
- **Training loop completo** (XOR con MLP, forward + backward + SGD)

## Build
```cmd
cd Runtime_2.0\c_abi
build.cmd
..\build\test_full.exe
```

## Estado: ✅ Implementado
