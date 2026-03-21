# ADead-BIB FFI GPU

**Runtime GPU con API Simple para SPIR-V, Vulkan, CUDA**

Author: Eddi Andreé Salazar Matos  
Email: eddi.salazar.dev@gmail.com  
Made with ❤️ in Peru 🇵🇪

---

## 🎯 Visión

FFI GPU proporciona una API simple para:
- Gestión de memoria GPU (buffers)
- Carga y ejecución de kernels SPIR-V
- Sincronización y eventos
- Binary Layout Optimizer para GPU

## 📁 Estructura

```
FFI GPU/
├── README.md
├── python/
│   ├── gpu_runtime.py      # Runtime GPU Python
│   ├── gpu_buffer.py       # Gestión de buffers
│   ├── gpu_kernel.py       # Carga/ejecución kernels
│   └── gpu_optimizer.py    # Binary Layout Optimizer
├── rust/
│   └── gpu_ffi.rs          # FFI Rust para GPU
├── kernels/
│   ├── matmul.spv          # Kernel matmul SPIR-V
│   ├── vecadd.spv          # Kernel vector add
│   └── reduce.spv          # Kernel reduction
└── examples/
    ├── matmul_demo.py      # Demo matmul
    └── vecadd_demo.py      # Demo vector add
```

## 🔥 API Ideal

```python
from gpu_runtime import GPU

# Inicializar GPU
gpu = GPU()

# Cargar kernel SPIR-V
kernel = gpu.load_spirv("matmul.spv")

# Crear buffers
A = gpu.buffer(data_a)           # CPU → GPU
B = gpu.buffer(data_b)
C = gpu.buffer(size=N*N)         # Solo GPU

# Ejecutar kernel
gpu.dispatch(kernel, A, B, C, groups=(32, 32, 1))

# Sincronizar
gpu.wait()

# Leer resultado
result = C.read()                 # GPU → CPU
```

## 📦 Gestión de Memoria

| Función | Descripción |
|---------|-------------|
| `gpu.buffer(data)` | Crear buffer y copiar CPU → GPU |
| `gpu.buffer(size=N)` | Crear buffer vacío en GPU |
| `buffer.write(data)` | Copiar CPU → GPU |
| `buffer.read()` | Copiar GPU → CPU |
| `buffer.free()` | Liberar memoria GPU |

## 🚀 Ejecución de Kernels

| Función | Descripción |
|---------|-------------|
| `gpu.load_spirv(path)` | Cargar bytecode SPIR-V |
| `gpu.load_adead(path)` | Cargar bytecode ADead-BIB |
| `gpu.create_pipeline(kernel)` | Crear pipeline de compute |
| `gpu.dispatch(kernel, *buffers, groups)` | Ejecutar kernel |

## ⏱ Sincronización

| Función | Descripción |
|---------|-------------|
| `gpu.wait()` | Esperar toda ejecución |
| `gpu.fence()` | Crear fence |
| `gpu.event()` | Crear evento |
| `gpu.stream()` | Crear stream/queue |

## 🔧 Binary Layout Optimizer

Optimiza el layout de datos para GPU:

```python
from gpu_optimizer import GPUOptimizer

opt = GPUOptimizer()

# Optimizar layout de matriz para GPU
optimized = opt.optimize_matrix(data, tile_size=32)

# Alinear a cache line
aligned = opt.align(data, alignment=128)

# Coalesced access pattern
coalesced = opt.coalesce(data, stride=32)
```

## 🏗️ Arquitectura Interna

```
┌─────────────────────────────────────────────┐
│              FFI GPU API                     │
├─────────────────────────────────────────────┤
│  gpu_runtime.py  │  gpu_buffer.py           │
│  gpu_kernel.py   │  gpu_optimizer.py        │
├─────────────────────────────────────────────┤
│           Vulkan/wgpu Runtime               │
├─────────────────────────────────────────────┤
│  Command Buffers │ Descriptor Sets          │
│  Pipeline State  │ Queue Submission         │
├─────────────────────────────────────────────┤
│              SPIR-V Bytecode                │
├─────────────────────────────────────────────┤
│         GPU Hardware (RTX 3060)             │
└─────────────────────────────────────────────┘
```

## 📊 Rendimiento Esperado

| Operación | CPU | GPU | Speedup |
|-----------|-----|-----|---------|
| MatMul 1024x1024 | 200ms | 5ms | **40x** |
| VecAdd 1M | 10ms | 0.5ms | **20x** |
| Reduce 1M | 15ms | 1ms | **15x** |

---

Made with ⚡ for ADead-BIB v3.2
