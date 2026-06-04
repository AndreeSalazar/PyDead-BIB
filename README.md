# PyDead-BIB 💀🦈

```
  ██████╗ ██╗   ██╗██████╗ ███████╗ █████╗ ██████╗       ██████╗ ██╗██████╗ 
  ██╔══██╗╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗██╔══██╗      ██╔══██╗██║██╔══██╗
  ██████╔╝ ╚████╔╝ ██║  ██║█████╗  ███████║██║  ██║█████╗██████╔╝██║██████╔╝
  ██╔═══╝   ╚██╔╝  ██║  ██║██╔══╝  ██╔══██║██║  ██║╚════╝██╔══██╗██║██╔══██╗
  ██║        ██║   ██████╔╝███████╗██║  ██║██████╔╝      ██████╔╝██║██████╔╝
  ╚═╝        ╚═╝   ╚═════╝ ╚══════╝╚═╝  ╚═╝╚═════╝       ╚═════╝ ╚═╝╚═════╝ 
```

> **Python + Vulkan: Compilación y Aceleración GPU para IA**

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Techne%20v1.0-purple.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-6.0.0-green.svg)](https://github.com/AndreeSalazar/PyDead-BIB)

```text
╔═══════════════════════════════════════════════════════════════════════════╗
║  v6.0 — PyDead-BIB: Python Frontend + Vulkan Compute Backend            ║
║  rustpython-parser (100% Python) + ash (Vulkan) + MatMul GPU            ║
╚═══════════════════════════════════════════════════════════════════════════╝
```

---

## 🚀 ¿Qué es PyDead-BIB v6.0?

**PyDead-BIB** es un proyecto de compilación y ejecución de código Python enfocado en **aceleración por hardware (IA)**. 

En su versión 6.0, el proyecto ha evolucionado de un compilador custom a una arquitectura modular basada en **Workspaces de Rust**, aprovechando herramientas de la industria para garantizar compatibilidad y rendimiento:

1.  **Frontend (`pyd-parser`)**: Utiliza `rustpython-parser` para garantizar **compatibilidad 100% con la sintaxis de Python** sin reinventar la rueda.
2.  **Core (`pyd-core`)**: Define la Representación Intermedia (IR) y las estructuras de datos para tensores.
3.  **Backend (`pyd-vulkan`)**: Utiliza `ash` (Vulkan bindings para Rust) para ejecutar operaciones de álgebra lineal (como **MatMul**) directamente en la **GPU**.
4.  **CLI (`pyd-cli`)**: Interfaz de línea de comandos que orquesta el flujo completo: `Python → IR → GPU → Resultado`.

---

## 🏗️ Arquitectura del Proyecto

El proyecto está organizado como un **Cargo Workspace** para máxima modularidad:

```text
PyDead-BIB/
├── Cargo.toml              # Workspace raíz
├── crates/
│   ├── pyd-cli/            # Binario principal (CLI)
│   ├── pyd-core/           # IR, tipos y estructuras base
│   ├── pyd-parser/         # Frontend: rustpython-parser → IR
│   ├── pyd-vulkan/         # Backend: ash (Vulkan) para Compute Shaders
│   └── pyd-gc/             # (Futuro) Garbage Collector
├── shaders/
│   └── matmul.comp         # Compute Shader SPIR-V para multiplicación de matrices
└── hello.py                # Script de prueba
```

### Flujo de Ejecución (MatMul en GPU)

```text
Código Python (.py)
        ↓
┌───────────────────────────────────────────┐
│  pyd-parser (rustpython-parser)           │
│  Parsea AST de Python → Genera IR         │
└───────────────────────────────────────────┘
        ↓
┌───────────────────────────────────────────┐
│  pyd-core                                 │
│  Instrucciones: CreateTensor, MatMul      │
└───────────────────────────────────────────┘
        ↓
┌───────────────────────────────────────────┐
│  pyd-vulkan (ash)                         │
│  1. Reserva buffers en VRAM               │
│  2. Carga datos (Host → Device)           │
│  3. Ejecuta Compute Shader (matmul.comp)  │
│  4. Lee resultado (Device → Host)         │
└───────────────────────────────────────────┘
        ↓
  Resultado en CPU
```

---

## 🛠️ Requisitos

- **Rust 1.75+** ([rustup.rs](https://rustup.rs))
- **Vulkan SDK** instalado y variable de entorno `VULKAN_SDK` configurada.
- **GPU compatible con Vulkan 1.2+** (NVIDIA, AMD, Intel modernos).
- **glslc** (del Vulkan SDK) para compilar shaders (opcional, el binario `.spv` ya está incluido).

---

## 📦 Instalación y Build

```bash
# Clonar el repositorio
git clone https://github.com/AndreeSalazar/PyDead-BIB.git
cd PyDead-BIB

# Compilar el workspace completo
cargo build --release

# El ejecutable estará en:
# Windows: target/release/pyd-cli.exe
# Linux:   target/release/pyd-cli
```

### Compilar el Shader (Si modificas `shaders/matmul.comp`)

```bash
# Usando glslc (incluido en Vulkan SDK)
glslc shaders/matmul.comp -o shaders/matmul.comp.spv
```

---

## 🚀 Uso

Ejecuta el pipeline completo de prueba:

```bash
cargo run --bin pyd-cli
```

### Salida esperada

```text
💀🦈 PyDead-BIB v6.0 - Enfoque Total en IA + Vulkan
✅ Motor Vulkan inicializado correctamente.

📜 Código Python a compilar:
a = Tensor([2, 2])
b = Tensor([2, 2])
c = a @ b

✅ Código parseado y convertido a IR. Total instrucciones: 3

📦 Creando Tensor en GPU: 'a' con forma [2, 2]
📦 Creando Tensor en GPU: 'b' con forma [2, 2]
📦 Creando Tensor en GPU: 'c' con forma [2, 2]

🔥 Ejecutando operación: c = a @ b
📥 Leyendo resultado del Tensor 'c' desde la GPU...
✅ Resultado de MatMul (C = A @ B):
   [ 19.0, 22.0 ]
   [ 43.0, 50.0 ]

🎉 ¡MATEMÁTICAS CORRECTAS! La GPU calculó la multiplicación de matrices perfectamente.
```

---

## 🧪 Estructura de Crates

### `pyd-core`
Define las estructuras fundamentales:
- `TensorMeta`: Metadatos de tensores (nombre, forma).
- `Instruction`: Enum con las operaciones IR (`CreateTensor`, `MatMul`).

### `pyd-parser`
Convierte código Python en instrucciones IR:
```rust
use pyd_parser::parse_and_lower;

let instructions = parse_and_lower("a = Tensor([2, 2])")?;
```

### `pyd-vulkan`
Motor de computación GPU usando `ash`:
- `VulkanEngine::new()`: Inicializa instancia, dispositivo y colas de computación.
- `create_tensor_buffer()`: Reserva memoria en VRAM y sube datos iniciales.
- `execute_matmul()`: Carga el shader SPIR-V y prepara el pipeline de computación.

---

## 🗺️ Roadmap

### v6.0 (Actual)
- [x] Workspace modular con Cargo.
- [x] Integración de `rustpython-parser` para frontend.
- [x] Motor Vulkan básico con `ash` (ash 0.38).
- [x] Pipeline de prueba para MatMul 2x2.

### v6.1 (Próximo)
- [ ] Implementar *Compute Dispatch* real en `pyd-vulkan` (Descriptor Sets + Command Buffers).
- [ ] Soporte para tensores N-dimensionales.
- [ ] Parseo real de literales de lista desde Python.

### v7.0
- [ ] Integración con modelos de IA simples (Redes Neuronales Feed-Forward).
- [ ] Optimización de memoria con *Memory Pools* en Vulkan.
- [ ] Soporte para múltiples operaciones (Add, ReLU, Softmax).

---

## 📄 Licencia

Este software está protegido bajo la **TECHNE LICENSE v1.0**.

> *"El arte pertenece al artesano. Su uso da frutos que deben compartirse."*

| Uso | Costo |
| --- | --- |
| Personal / individual | **GRATIS** |
| Estudiantes / educación | **GRATIS** |
| Open source (OSI) | **GRATIS** |
| ONG / nonprofit | **GRATIS** |
| Startup < $1M/year | **GRATIS** |
| Empresa > $1M revenue | **10% royalty** sobre revenue atribuible |

---

## 👨‍💻 Autor

**Eddi Andree Salazar Matos**  
Lima, Perú 🇵🇪  

GitHub: [github.com/AndreeSalazar](https://github.com/AndreeSalazar)  

---

*"Python + Vulkan — Aceleración real para IA"*  
*Licensed under Techne v1.0 — Lima, Perú 🇵🇪 — 2026*
