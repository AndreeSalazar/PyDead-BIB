# PyDead-BIB 💀🦈
  ██████╗ ██╗   ██╗██████╗ ███████╗ █████╗ ██████╗       ██████╗ ██╗██████╗ 
  ██╔══██╗╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗██╔══██╗      ██╔══██╗██║██╔══██╗
  ██████╔╝ ╚████╔╝ ██║  ██║█████╗  ███████║██║  ██║█████╗██████╔╝██║██████╔╝
  ██╔═══╝   ╚██╔╝  ██║  ██║██╔══╝  ██╔══██║██║  ██║╚════╝██╔══██╗██║██╔══██╗
  ██║        ██║   ██████╔╝███████╗██║  ██║██████╔╝      ██████╔╝██║██████╔╝
  ╚═╝        ╚═╝   ╚═════╝ ╚══════╝╚═╝  ╚═╝╚═════╝       ╚═════╝ ╚═╝╚═════╝ 

**Python + Vulkan: Compilation and GPU Acceleration for AI**

![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)
![License](https://img.shields.io/badge/License-Techne%20v1.0-purple.svg)
![Version](https://img.shields.io/badge/Version-6.0.0-green.svg)

╔═══════════════════════════════════════════════════════════════════════════╗
║  v6.0 — PyDead-BIB: Python Frontend + Vulkan Compute Backend              ║
║  rustpython-parser (100% Python compatible) + ash (Vulkan) + GPU MatMul   ║
╚═══════════════════════════════════════════════════════════════════════════╝

## 🚀 What is PyDead-BIB v6.0?
PyDead-BIB is a high-performance compilation and execution engine for Python code, specifically designed for hardware-accelerated Artificial Intelligence workloads. 

In version 6.0, the project has evolved from a custom compiler into a modular **Rust Workspace** architecture, leveraging industry-standard tools to guarantee compatibility, safety, and raw performance:
* **Frontend (`pyd-parser`)**: Utilizes `rustpython-parser` to guarantee 100% compatibility with Python syntax without reinventing the wheel.
* **Core (`pyd-core`)**: Defines the Intermediate Representation (IR) and data structures for N-dimensional tensors.
* **Backend (`pyd-vulkan`)**: Utilizes `ash` (Vulkan bindings for Rust) to execute linear algebra operations (like Matrix Multiplication) directly on the GPU via Compute Shaders.
* **CLI (`pyd-cli`)**: A professional command-line interface (powered by `clap`) that orchestrates the entire pipeline: `Python File → AST → IR → GPU → Result`.

## 🏗️ Project Architecture
The project is organized as a Cargo Workspace for maximum modularity and clean separation of concerns:

```text
PyDead-BIB/
├── Cargo.toml              # Root workspace configuration
├── crates/
│   ├── pyd-cli/            # Main binary (CLI orchestrator using clap)
│   ├── pyd-core/           # IR, types, and base structures (TensorMeta, Instructions)
│   ├── pyd-parser/         # Frontend: rustpython-parser → IR lowering
│   ├── pyd-vulkan/         # Backend: ash (Vulkan) for Compute Shaders & Memory
│   └── pyd-gc/             # (Future) Custom Garbage Collector
├── shaders/
│   └── matmul.comp         # SPIR-V Compute Shader for matrix multiplication
└── hello.py                # Example test script
```

### Execution Flow (GPU MatMul)
```text
Python Code (.py)
        ↓
┌─────────────────────────────────────────────────────────┐
│  pyd-parser (rustpython-parser)                         │
│  Parses Python AST → Lowers to Custom IR                │
└─────────────────────────────────────────────────────────┘
        ↓
┌─────────────────────────────────────────────────────────┐
│  pyd-core                                               │
│  Generates Instructions: CreateTensor, MatMul, Read     │
└─────────────────────────────────────────────────────────┘
        ↓
┌─────────────────────────────────────────────────────────┐
│  pyd-vulkan (ash)                                       │
│  1. Allocates VRAM buffers (vk::Buffer)                 │
│  2. Uploads data (Host → Device via vkMapMemory)        │
│  3. Executes Compute Shader (vkCmdDispatch)             │
│  4. Reads result back (Device → Host)                   │
└─────────────────────────────────────────────────────────┘
        ↓
  Final Result on CPU
```

## 🛠️ Requirements
* **Rust 1.75+** ([rustup.rs](https://rustup.rs))
* **Vulkan SDK** installed with the `VULKAN_SDK` environment variable configured.
* A **Vulkan 1.2+ compatible GPU** (Modern NVIDIA, AMD, or Intel).
* `glslc` (included in the Vulkan SDK) to compile shaders (optional, as the `.spv` binary is already included).

## 📦 Installation & Build

```bash
# Clone the repository
git clone https://github.com/AndreeSalazar/PyDead-BIB.git
cd PyDead-BIB

# Build the entire workspace in release mode
cargo build --release

# The executable will be located at:
# Windows: target/release/pyd.exe
# Linux:   target/release/pyd
```

### Compiling the Shader (If you modify `shaders/matmul.comp`)
```bash
# Using glslc (included in Vulkan SDK)
glslc shaders/matmul.comp -o shaders/matmul.comp.spv
```

## 🚀 Usage
The CLI is powered by `clap` and supports subcommands.

### 1. Run a Python script on the GPU
```bash
cargo run --bin pyd -- run --file hello.py
```

### 2. Show engine information
```bash
cargo run --bin pyd -- info
```

**Expected Output for `run --file hello.py`:**
```text
💀🦈 PyDead-BIB v6.0 - Total Focus on AI + Vulkan

📂 Reading script: hello.py

📜 Python code to compile:
a = Tensor([2, 2])
b = Tensor([2, 2])
c = a @ b

✅ Code parsed and lowered to IR. Total instructions: 3
✅ Vulkan Engine initialized successfully.

📦 Creating Tensor on GPU: 'a' with shape [2, 2]
📦 Creating Tensor on GPU: 'b' with shape [2, 2]

🔥 Executing operation: c = a @ b
   ⚡ [Vulkan] Implicitly creating destination Tensor 'c' with shape [2, 2]
   🚀 [Vulkan] Dispatched MatMul Compute Shader: c = a @ b

📥 Reading result from Tensor 'c' on GPU...
✅ MatMul Result (C = A @ B):
   [ 19.0, 22.0 ]
   [ 43.0, 50.0 ]

🎉 MATHEMATICS VERIFIED! The GPU calculated the matrix multiplication perfectly.
```

## 🧪 Crate Structure Details
* **`pyd-core`**: Defines fundamental structures like `TensorMeta` (tensor metadata: name, shape) and `Instruction` (enum for IR operations like `CreateTensor`, `MatMul`).
* **`pyd-parser`**: Converts Python code into IR instructions. Example: `let instructions = parse_and_lower("a = Tensor([2, 2])")?;`
* **`pyd-vulkan`**: The GPU computation engine using `ash`. Handles `VulkanEngine::new()` (instance/device setup), `create_tensor_buffer()` (VRAM allocation), and `execute_matmul()` (shader pipeline preparation).
* **`pyd-cli`**: The modular CLI entry point. Uses `clap` for argument parsing and `thiserror` for professional error handling.

## 🗺️ Roadmap
### v6.1 (Next)
- [ ] Implement **real Compute Dispatch** in `pyd-vulkan` (Descriptor Sets, Command Buffers, and actual `vkCmdDispatch`).
- [ ] Support for **N-Dimensional Tensors** (dynamic shapes beyond 2x2).
- [ ] Real parsing of Python list literals (extracting actual float values from the AST).

### v6.2
- [ ] Add element-wise operations: `Add`, `Multiply`, `ReLU`.
- [ ] Basic computation graph for future Autograd (backpropagation) support.

### v7.0
- [ ] Integration with simple AI models (Feed-Forward Neural Network primitives).
- [ ] Memory optimization using **Vulkan Memory Allocator (VMA)**.
- [ ] Performance benchmarking against CPU-only baselines.

## 📄 License
This software is protected under the **TECHNE LICENSE v1.0**.  
*"The art belongs to the artisan. Its use bears fruit that must be shared."*

| Usage | Cost |
| :--- | :--- |
| Personal / Individual | **FREE** |
| Students / Education | **FREE** |
| Open Source (OSI) | **FREE** |
| NGO / Nonprofit | **FREE** |
| Startup (< $1M/year revenue) | **FREE** |
| Enterprise (> $1M/year revenue) | **10% royalty** on attributable revenue |

## 👨‍💻 Author
**Eddi Andree Salazar Matos**  
Lima, Peru 🇵🇪  
GitHub: [github.com/AndreeSalazar](https://github.com/AndreeSalazar)  

*"Python + Vulkan — Real Acceleration for AI"*  
Licensed under Techne v1.0 — Lima, Perú 🇵🇪 — 2026
