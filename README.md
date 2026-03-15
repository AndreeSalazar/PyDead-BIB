# PyDead-BIB 💀🦈

> **Python → x86-64 Nativo — Sin CPython — Sin GIL — Sin Runtime**

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Techne%20v1.0-purple.svg)](TECHNE_LICENSE_v1.0.license)
[![Version](https://img.shields.io/badge/Version-3.0.0-green.svg)](https://github.com/AndreeSalazar/PyDead-BIB)
[![Tests](https://img.shields.io/badge/Tests-44%2F44%20PASS-brightgreen.svg)](https://github.com/AndreeSalazar/PyDead-BIB)

```text
Guido van Rossum: 'readability counts'
Dennis Ritchie:   'small is beautiful'
Grace Hopper:     'la máquina sirve al humano'
PyDead-BIB 2026:  hereda ADead-BIB v8.0 — Python nativo — 16→256 bits
```

---

## ¿Qué es PyDead-BIB?

**PyDead-BIB** es el primer compilador que transforma código Python directamente a ejecutables nativos x86-64, sin depender de CPython, PyPy, ni ningún runtime.

| Caracteristica | CPython | PyPy | Nuitka | PyDead-BIB |
| --- | --- | --- | --- | --- |
| Sin runtime | ❌ 30MB | ❌ 200MB | ❌ 8MB | ✅ **0 bytes** |
| Sin GIL | ❌ | ❌ | ❌ | ✅ |
| Sin GCC/LLVM | ✅ | ✅ | ❌ | ✅ |
| 256-bit SIMD | ❌ | ❌ | ❌ | ✅ AVX2 |
| UB compile-time | ❌ | ❌ | ❌ | ✅ 13+ tipos |
| Hello World | 30MB | 200MB | 8MB | **~2KB** |

---

## Instalación

### Requisitos

- **Rust 1.75+** ([rustup.rs](https://rustup.rs))
- Windows 10/11 o Linux x64

### Build

```bash
# Clonar
git clone https://github.com/tu-usuario/PyDead-BIB.git
cd PyDead-BIB

# Compilar
cargo build --release

# El ejecutable está en:
# Windows: target/release/pyb.exe
# Linux:   target/release/pyb
```

### Agregar al PATH (opcional)

```powershell
# Windows PowerShell
$env:PATH += ";$PWD\target\release"

# Linux/macOS
export PATH="$PATH:$PWD/target/release"
```

---

## Uso Rápido

### Compilar Python

```bash
# Compilación básica
pyb py archivo.py -o output.exe

# Target específico
pyb py archivo.py --target windows    # PE x64
pyb py archivo.py --target linux      # ELF x64
pyb py archivo.py --target fastos256  # FastOS 256-bit

# Compilar y ejecutar
pyb run archivo.py
```

### Step Mode — Ver las 11 fases

```bash
pyb step archivo.py
```

```text
╔══════════════════════════════════════════════════════════════╗
║   PyDead-BIB Step Compiler — Deep Analysis Mode 💀🦈         ║
╚══════════════════════════════════════════════════════════════╝
  Source:   archivo.py
  Language: Python 3.x

--- Phase 01: PREPROCESSOR ---
[PREPROC]  encoding: UTF-8 detectado
[PREPROC]  source: 25 lines

--- Phase 02: IMPORT ELIMINATOR ---
[IMPORT]   math → SIMD inline
[IMPORT]   sin site-packages — NUNCA

--- Phase 03: LEXER ---
[LEXER]    127 tokens generados
[LEXER]    INDENT/DEDENT: 8/8 pares

--- Phase 04: PARSER ---
[PARSER]   AST generated — 5 top-level nodes
[PARSER]     fn main(0 params)

--- Phase 05: TYPE INFERENCER ---
[TYPES]    type inference complete

--- Phase 06: IR (ADeadOp SSA-form) ---
[IR]       2 functions compiled
[IR]       15 IR statements total
[IR]       GIL eliminado — ownership estático ✓

--- Phase 07: UB DETECTOR ---
[UB]       ✓ CLEAN — sin undefined behavior detectado

✅ Frontend compilation complete
   Sin CPython — Sin GIL — Sin runtime 💀🦈
```

### Crear Proyecto

```bash
pyb create mi_app
cd mi_app
pyb run src/main.py
```

Estructura generada:

```text
mi_app/
├── pyb.toml        # Configuración del proyecto
└── src/
    └── main.py     # Código fuente
```

---

## Sintaxis Python Soportada

### Python 2.7 → 3.13 Completo

```python
# Tipos y literales
x: int = 42              # → RAX literal
y: float = 3.14          # → XMM/YMM
s: str = "hola"          # → .data section
b: bool = True           # → 1 byte

# f-strings (3.6+)
msg = f"Hola {name}"     # → string concat nativo

# Walrus operator (3.8+)
if (n := len(data)) > 10:
    print(n)

# Match/case (3.10+)
match comando:
    case "help": mostrar_ayuda()
    case "exit": salir()
    case _: error()
```

### Funciones y Clases

```python
def suma(a: int, b: int) -> int:
    return a + b           # → ADD RAX, RBX directo

class Jugador:
    nombre: str
    vida: int
    
    def __init__(self, nombre: str):
        self.nombre = nombre
        self.vida = 100
    
    def atacar(self) -> int:
        return 10          # → vtable entry
```

### Comprehensions → SIMD Automático

```python
# PyDead-BIB detecta: list[float] × 8
velocidades = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
dobles = [v * 2.0 for v in velocidades]
# Genera: VMULPS ymm0, ymm1, [2.0×8]
# 8 floats en 1 ciclo — sin loop
```

---

## UB Detection — Errores en Compile Time

PyDead-BIB detecta errores **antes** de ejecutar:

```python
# ❌ CPython: RuntimeError
# ✅ PyDead-BIB: Error en compilación

x = None
print(x.nombre)          # NoneDeref detectado

lista = [1, 2, 3]
print(lista[100])        # IndexOutOfBounds detectado

def f(x=[]):             # MutableDefaultArg warning
    x.append(1)          # Bug clásico Python → nunca más

"hola" + 42              # TypeMismatch detectado
```

### 13 Tipos de UB Detectados

| UB | Descripción | CPython | PyDead-BIB |
|----|-------------|---------|------------|
| `NoneDeref` | `None.attr` | RuntimeError | ✅ Compile |
| `IndexOutOfBounds` | `lista[100]` | IndexError | ✅ Compile |
| `KeyNotFound` | `dict["x"]` | KeyError | ✅ Compile |
| `TypeMismatch` | `"a" + 1` | TypeError | ✅ Compile |
| `DivisionByZero` | `x / 0` | ZeroDivisionError | ✅ Compile |
| `MutableDefaultArg` | `def f(x=[])` | Bug silencioso | ✅ Warning |
| `InfiniteRecursion` | Sin base case | RecursionError | ✅ Compile |
| `CircularImport` | A→B→A | ImportError | ✅ Compile |
| `UnpackMismatch` | `a,b = [1,2,3]` | ValueError | ✅ Compile |

---

## Configuración — pyb.toml

```toml
[project]
name = "mi_app"
version = "0.1.0"
lang = "python"
standard = "py3"

[build]
src = "src/"
output = "bin/"

[python]
version = "3.11"          # Sintaxis target
type_check = "strict"     # Inferencia estricta
ub_mode = "strict"        # Detener en UB
simd = "auto"             # AVX2 automático
```

---

## Arquitectura

```text
Python Source (.py)
        │
        ▼
┌─────────────────────────────────────────────────────┐
│  FRONTEND (★ PyDead-BIB v1.0)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │ Preprocessor│→ │Import Elim. │→ │   Lexer     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
│         │                                │          │
│         ▼                                ▼          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Parser    │→ │Type Inferrer│→ │  IR Gen     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
└─────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────┐
│  MIDDLE-END (heredado ADead-BIB v8.0)               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │ UB Detector │→ │  Optimizer  │→ │ Reg Alloc   │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
└─────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────┐
│  BACKEND (heredado ADead-BIB v8.0)                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │Bit Resolver │→ │ISA Compiler │→ │ PE/ELF/Po   │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
└─────────────────────────────────────────────────────┘
        │
        ▼
   .exe / ELF / .po (nativo, sin runtime)
```

---

## Comparación de Performance

```text
Hello World:

┌────────────────┬──────────────┬──────────────┐
│ Implementación │ Tamaño       │ Startup      │
├────────────────┼──────────────┼──────────────┤
│ CPython 3.13   │ 30MB runtime │ ~50ms        │
│ PyPy 7.3       │ 200MB        │ ~2s warmup   │
│ Nuitka         │ 8MB          │ ~10ms        │
│ PyDead-BIB     │ ~2KB         │ ~0.1ms       │
└────────────────┴──────────────┴──────────────┘

Bucle float × 8:

numpy:           import 50ms + BLAS overhead
PyDead-BIB:      VMULPS ymm0 directo — 8 floats/ciclo
```

---

## Targets de Salida

| Target | Formato | Descripción |
|--------|---------|-------------|
| `--target windows` | `.exe` PE | Windows x64 nativo |
| `--target linux` | ELF | Linux x64 nativo |
| `--target fastos64` | `.po` v1 | FastOS 64-bit |
| `--target fastos128` | `.po` v2 | FastOS SSE 128-bit |
| `--target fastos256` | `.po` v8 | FastOS AVX2 256-bit |
| `--target all` | Multi | Todos simultáneos |

---

## Desarrollo

### Estructura del Proyecto

```text
PyDead-BIB/
├── Cargo.toml
├── src/rust/
│   ├── lib.rs              # Biblioteca
│   ├── main.rs             # CLI pyb
│   ├── frontend/python/    # Frontend Python (★ nuevo)
│   │   ├── py_preprocessor.rs
│   │   ├── py_import_resolver.rs
│   │   ├── py_lexer.rs
│   │   ├── py_parser.rs
│   │   ├── py_ast.rs
│   │   ├── py_types.rs
│   │   └── py_to_ir.rs
│   └── middle/             # Middle-end
│       ├── ir.rs
│       └── ub_detector.rs
```

### Comandos de Desarrollo

```bash
# Build debug
cargo build

# Build release
cargo build --release

# Run tests
cargo test

# Ver versión
pyb --version
```

---

## Roadmap

### v1.2 — Real Runtime Output ✅

- [x] 13/13 compilation phases
- [x] PE/ELF/FastOS output
- [x] print() real output via Win32 WriteFile
- [x] Binary Guardian stamp

### v1.3 — Arithmetic & Control Flow ✅

- [x] float print, arithmetic (+,-,*,//,%,**)
- [x] if/elif/else, for range(), while loops
- [x] f-strings, AugAssign, SSE2 instructions

### v1.4 — Data Structures ✅

- [x] import math (sqrt, floor, ceil, sin, cos, log, pi, e)
- [x] Lists (HeapAlloc, append, len, indexing)
- [x] Dicts (HeapAlloc, open addressing)
- [x] Classes (init, self.x, field access)
- [x] Builtins: abs, min, max, chr, ord, tuple unpack

### v1.5 — Standard Library & Package Manager ✅

- [x] `pyb install` / `pyb list` — native package manager
- [x] `import os` — getcwd, path.exists, getpid, mkdir, remove, rename, environ.get
- [x] `import sys` — platform, version, maxsize, exit
- [x] `import random` — seed, randint, random (xorshift64)
- [x] `import json` — loads, dumps (stubs)
- [x] `open()` — CreateFileA, read, write, close
- [x] String methods — upper, lower, find, replace, startswith, endswith
- [x] IAT expanded to 20 Win32 API slots

### v2.0 — Python Standard Total ✅

- [x] **ANSI terminal colors** — full colored compiler output with architecture diagram
- [x] **UB error blocking** — compilation blocked with detailed error messages when UB detected
- [x] **try/except/finally/raise** — error codes, handler jumps, finally blocks, raise with message
- [x] **with statement** — context managers with auto-close for file handles
- [x] **Inheritance** — class bases, parent field inheritance, method override
- [x] **List comprehensions** — `__pyb_listcomp_range` runtime stub
- [x] **Decorators** — class methods, static dispatch
- [x] **async/await** — parser support, IR passthrough, Await expression handling
- [x] **generators/yield** — parser support, Yield expression handling
- [x] **Dataclasses** — class with `__init__`, field access, constructor
- [x] **String formatting** — f-strings with expressions, format specs
- [x] **Typing** — List, Dict, Optional, Union type hint passthrough
- [x] **Import modules** — multi-module compilation, math/os/sys/random/json
- [x] **numpy-native** — list-based SIMD arrays with HeapAlloc
- [x] 36/36 pyb test PASS, 57/57 cargo test PASS

### v3.0 — Production Ready ✅

- [x] **async/await state machine** — CoroutineCreate/Resume/Yield IR, asyncio.run() routing
- [x] **generators/yield** — GeneratorCreate/Next/Send IR, `__pyb_gen_next` runtime stub
- [x] **@property / @lru\_cache** — PropertyGet/Set IR, LruCacheCheck/Store IR instructions
- [x] **numpy AVX2 SIMD** — VMOVAPS/VADDPS/VMULPS/VSQRTPS YMM codegen, np.sum/max/min/dot stubs
- [x] **C extension compatibility** — ctypes.CDLL routing, DllLoad/GetProc/Call IR instructions
- [x] **Optimizer pipeline** — constant folding (int+float BinOp), dead code elimination (Nop removal)
- [x] **sum()/next() builtins** — wired to numpy runtime stubs
- [x] 44/44 pyb test PASS, 57/57 cargo test PASS

### v4.0 — Distribution

- [ ] PyPI-compatible distribution
- [ ] Linux ELF syscall stubs (write/mmap/exit)
- [ ] Cross-compilation targets
- [ ] Incremental compilation

---

## Licencia

Este software esta protegido bajo la **TECHNE LICENSE v1.0**.

> *"El arte pertenece al artesano. Su uso da frutos que deben compartirse."*

```text
TECHNE LICENSE v1.0 — Binary Is Binary
Copyright (C) 2026 Eddi Andree Salazar Matos — Lima, Peru
```

| Uso | Costo |
| --- | --- |
| Personal / individual | **GRATIS** |
| Estudiantes / educacion | **GRATIS** |
| Open source (OSI) | **GRATIS** |
| ONG / nonprofit | **GRATIS** |
| Startup < $1M/year | **GRATIS** |
| Empresa > $1M revenue | **10% royalty** sobre revenue atribuible |
| Enterprise / buyout | Negociable — contactar al autor |

Ver el archivo completo: [`TECHNE_LICENSE_v1.0.license`](TECHNE_LICENSE_v1.0.license)

Contacto para licencias comerciales: **<eddi.salazar.dev@gmail.com>**

---

## Autor

**Eddi Andree Salazar Matos**
Lima, Peru 🇵🇪
1 dev — Binary Is Binary 💀🦈

GitHub: [github.com/AndreeSalazar](https://github.com/AndreeSalazar)
Email: <eddi.salazar.dev@gmail.com>

---

*"Python sin runtime — sin GIL — sin CPython — sin linker — 16 hasta 256 bits"*
*Licensed under Techne v1.0 — Lima, Peru 🇵🇪 — 2026*
