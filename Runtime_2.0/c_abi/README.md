# C ABI — Runtime 2.0 💀🦈

Compilador C temporal hasta que el C propio esté completo.

## Herramientas

| Herramienta | Ruta | Versión |
|-------------|------|---------|
| GCC (MSYS2) | `C:\msys64\mingw64\bin\gcc.exe` | MinGW-w64 |
| NVCC | `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.1\bin\nvcc.exe` | CUDA 13.1 |

## Build
```cmd
cd Runtime_2.0\c_abi
build.cmd
```

Genera `.o` en `Runtime_2.0\build\`:
- `tensor.o` — Tensor core
- `arena.o` — Arena allocator
- `nn_ops.o` — NN operations
- `linear.o` — Linear/MLP layers
- `test_tensor.exe` — Test suite

## Cuando el C propio esté listo
Se reemplaza GCC pero los `.h` no cambian — la API es estable.

## Estado: ✅ Funcional
