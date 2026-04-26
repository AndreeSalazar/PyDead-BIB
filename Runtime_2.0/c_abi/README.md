# C ABI — Runtime 2.0 💀🦈

Compiladores C temporales hasta que el C propio esté completo.

## Herramientas

| Herramienta | Ruta | Versión |
|-------------|------|---------|
| GCC (MSYS2) | `C:\msys64\mingw64\bin\gcc.exe` | MinGW-w64 GCC 15.2.0 |
| MSVC (VS 2022) | `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools` | cl.exe 14.44 |
| NVCC | `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.1\bin\nvcc.exe` | CUDA 13.1 |

## Build GCC

```cmd
cd Runtime_2.0\c_abi
build.cmd
```

Genera `.o` en `Runtime_2.0\build\`:

- `tensor.o` — Tensor core AVX2
- `arena.o` — Arena allocator
- `nn_ops.o` — NN operations (GELU, LayerNorm, CE Loss)
- `linear.o` — Linear/MLP layers
- `autograd.o` — Autograd tape-based AD
- `optim.o` — SGD + AdamW
- `tensor_io.o` — Tensor I/O
- `test_tensor.exe` — 30 tests
- `test_full.exe` — 69 tests + training loop

## Build MSVC

```cmd
cd Runtime_2.0\c_abi
build_msvc.cmd
```

Genera `.obj` en `Runtime_2.0\build_msvc\` (mismos módulos).

## Tests verificados

- GCC: test_tensor 30/30, test_full 69/69
- MSVC: test_tensor 30/30, test_full 69/69

## Cuando el C propio esté listo

Se reemplaza GCC/MSVC pero los `.h` no cambian — la API es estable.

## Estado: ✅ Funcional (GCC + MSVC)
