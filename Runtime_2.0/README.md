# Runtime 2.0 — PyDead-BIB 💀🦈

> **"runtime mínimo: sin GC, sin VM, sin interpretación, sin reflection"**
> **"solo: memoria continua, ejecución matemática, scheduler determinista"**

---

## Filosofía

- **NO** es un runtime como CPython/PyPy/JVM
- **ES** un conjunto de primitivas compiladas nativas para IA
- Cada componente compila a `.o` y se enlaza estáticamente
- El compilador (`src/rust/`) genera código que **LLAMA** a estas primitivas
- Memoria continua + AVX2 SIMD + determinismo = IA nativa

---

## Arquitectura

```
Runtime_2.0/
├── tensor/       # Tipo Tensor nativo — memoria continua, strides, shapes
│   ├── tensor.h          # API pública
│   ├── tensor.c          # Implementación AVX2
│   └── test_tensor.c     # Tests
├── math_ops/     # Operaciones IA — activaciones, loss, normalization
│   ├── nn_ops.h          # API
│   └── nn_ops.c          # GELU, LayerNorm, CrossEntropy, etc.
├── memory/       # Arena allocator — sin GC, determinista
│   ├── arena.h           # API
│   └── arena.c           # VirtualAlloc/mmap arena
├── c_abi/        # C temporal (MSYS2 GCC) hasta tener C propio completo
│   ├── build.cmd         # Compilar .c → .o con GCC
│   └── README.md         # Instrucciones
├── cuda/         # CUDA kernels — matmul GPU, relu GPU
│   ├── matmul.cu         # Tiled matmul kernel
│   ├── build.cmd         # Compilar .cu → .ptx con NVCC
│   └── README.md         # Instrucciones
├── nn/           # Linear, MLP — capas de red neuronal
│   ├── linear.h          # API
│   └── linear.c          # Linear + MLP forward
└── autograd/     # (Futuro) Diferenciación automática
    └── autograd.h        # Diseño tape-based reverse AD
```

---

## C ABI — Compilador C Temporal

Hasta que el C propio esté completo, usamos compiladores externos:

| Herramienta | Ruta | Uso |
|-------------|------|-----|
| MSYS2 GCC | `C:\msys64\mingw64\bin\gcc.exe` | Compilar `.c` → `.o` |
| NVCC | `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.1\bin\nvcc.exe` | Compilar `.cu` → `.ptx` |

Cuando el C propio esté listo, se reemplaza GCC pero la API (`.h`) **no cambia**.

---

## Cómo Compila

```
1. Runtime_2.0/c_abi/build.cmd    → compila todos los .c → .o (GCC temporal)
2. Runtime_2.0/cuda/build.cmd     → compila .cu → .ptx (NVCC)
3. PyDead-BIB (Rust) genera       → Python → x86-64 con CALL a runtime
4. Binario final                  → standalone: runtime embebido + código usuario
```

---

## Build Rápido

```cmd
cd Runtime_2.0\c_abi
build.cmd

cd ..\cuda
build.cmd
```

---

## Estado

| Módulo | Estado | Prioridad |
|--------|--------|-----------|
| tensor/ | ✅ Implementado | ★★★★★ |
| math_ops/ | ✅ Implementado | ★★★★★ |
| memory/ | ✅ Implementado | ★★★★☆ |
| nn/ | ✅ Implementado | ★★★★☆ |
| cuda/ | ✅ Kernel listo | ★★★★☆ |
| c_abi/ | ✅ Build scripts | ★★★☆☆ |
| autograd/ | 📐 Diseño | ★★★★★ |

---

*Runtime 2.0 — PyDead-BIB — Abril 2026*
*"memoria continua + ejecución matemática + scheduler determinista"*
*Eddi Andreé Salazar Matos — Lima, Perú 🇵🇪 💀🦈*
