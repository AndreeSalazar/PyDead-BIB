# Metal-Dead para PyDead-BIB v3.0

**IA Personal Ultra-Eficiente — Compilado NATIVO x86-64 — Sin CPython**

Author: Eddi Andree Salazar Matos
Email: eddi.salazar.dev@gmail.com
Lima, Peru

---

## Inicio Rapido

```powershell
cd PyDead-BIB

# Compilar y ejecutar cualquier modulo
cargo run --release -- Metal_Dead/core/metal_dead.py
.\metal_dead.exe

# Compilar JARVIS
cargo run --release -- Metal_Dead/jarvis/jarvis.py
.\jarvis.exe

# Compilar todo el sistema
cargo run --release -- Metal_Dead/__main__.py
.\__main__.exe

# Test suite completo (80 tests)
cargo run --release -- test

# GPU test con PyTorch (CPython)
python Metal_Dead/integrations/pytorch_gpu_test.py
```

---

## Estructura

```
Metal_Dead/
├── __init__.py              # Init PyDead-BIB
├── __main__.py              # Entry point compilado
├── core/
│   ├── tokenizer.py         # Tokenizador caracter
│   ├── model.py             # Transformer compilado
│   ├── memory.py            # Memoria en RAM
│   ├── context.py           # Contexto personal
│   ├── intelligence.py      # Motor inteligencia
│   ├── metal_dead.py        # Sistema principal
│   ├── metal_dead_smart.py  # Pensamiento critico
│   ├── metal_dead_cpu.py    # CPU SIMD optimizado
│   └── cpu_compute.py       # CPU compute nativo
├── integrations/
│   ├── gpu_compute.py       # GPU CUDA simulacion
│   ├── gpu_advanced.py      # Flash Attention + BF16
│   ├── adead_accelerator.py # PyDead-BIB accelerator
│   ├── metal_dead_smart_gpu.py # Smart + GPU
│   ├── llm_bridge.py        # Ollama + PyTorch LLM
│   └── pytorch_gpu_test.py  # GPU benchmark (CPython)
├── jarvis/
│   └── jarvis.py            # Asistente JARVIS completo
├── tools/
│   ├── web_search.py        # Busqueda web
│   ├── file_manager.py      # Gestion archivos
│   └── data_analyst.py      # Analisis datos
├── ui/
│   ├── chat.py              # Chat interactivo
│   └── cli.py               # Linea de comandos
└── data/                    # Datos persistentes
```

---

## Caracteristicas

| Caracteristica | Descripcion |
| --- | --- |
| **Compilado NATIVO** | Python a x86-64 sin CPython |
| **GPU CUDA** | RTX 3060 12GB via PyTorch |
| **CPU SIMD AVX2** | Vectorizacion nativa 256-bit |
| **LLM Ollama** | llama3 en localhost:11434 |
| **PyTorch** | CUDA 12.1 + Tensor Cores |
| **Sin Runtime** | Binario puro ~5-11KB |
| **Pensamiento Critico** | Razona antes de responder |
| **Base Conocimiento** | 13 temas integrados |
| **JARVIS** | Asistente completo |
| **80/80 Tests** | Todo compila y ejecuta |

---

## Rendimiento (RTX 3060 12GB)

| Operacion | Resultado |
| --- | --- |
| MatMul 2048x2048 | 7486 GFLOPS |
| MatMul 1024x1024 | 4680 GFLOPS |
| Softmax 1024x1024 | 0.21ms |
| Attention seq=128 | 3.72ms |
| Compilacion PyDead-BIB | < 100ms |
| Binario Metal-Dead | 5-11 KB |

---

## Archivos Compilados (27 archivos PyDead-BIB)

Todos compilan a binarios nativos x86-64 Windows PE:

- **core/** — 8 archivos (tokenizer, model, memory, context, intelligence, metal_dead, smart, cpu)
- **integrations/** — 6 archivos (gpu, gpu_advanced, accelerator, smart_gpu, llm_bridge, init)
- **ui/** — 3 archivos (chat, cli, init)
- **jarvis/** — 2 archivos (jarvis, init)
- **tools/** — 4 archivos (web_search, file_manager, data_analyst, init)
- **root** — 4 archivos (init, main, core_init, integ_init)

---

## Requisitos

```powershell
# PyDead-BIB (compilador)
cargo build --release

# PyTorch GPU (opcional, para benchmark real)
pip install torch --index-url https://download.pytorch.org/whl/cu121

# Ollama LLM (opcional, para LLM local)
# Ya instalado: ollama v0.18.0
```

---

Made with PyDead-BIB v3.0 — Eddi Andree Salazar Matos — Lima, Peru
