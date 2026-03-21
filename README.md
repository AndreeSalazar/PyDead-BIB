# PyDead-BIB 💀🦈

```
  ██████╗ ██╗   ██╗██████╗ ███████╗ █████╗ ██████╗       ██████╗ ██╗██████╗ 
  ██╔══██╗╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗██╔══██╗      ██╔══██╗██║██╔══██╗
  ██████╔╝ ╚████╔╝ ██║  ██║█████╗  ███████║██║  ██║█████╗██████╔╝██║██████╔╝
  ██╔═══╝   ╚██╔╝  ██║  ██║██╔══╝  ██╔══██║██║  ██║╚════╝██╔══██╗██║██╔══██╗
  ██║        ██║   ██████╔╝███████╗██║  ██║██████╔╝      ██████╔╝██║██████╔╝
  ╚═╝        ╚═╝   ╚═════╝ ╚══════╝╚═╝  ╚═╝╚═════╝       ╚═════╝ ╚═╝╚═════╝ 
```

> **Python → x86-64 Nativo — Sin CPython — Sin GIL — Sin Runtime**

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Techne%20v1.0-purple.svg)](TECHNE_LICENSE_v1.0.license)
[![Version](https://img.shields.io/badge/Version-4.3.0-green.svg)](https://github.com/AndreeSalazar/PyDead-BIB)
[![Tests](https://img.shields.io/badge/Tests-135%20PASS-brightgreen.svg)](https://github.com/AndreeSalazar/PyDead-BIB)
[![JIT](https://img.shields.io/badge/JIT%20KILLER-v2.0-ff00ff.svg)](https://github.com/AndreeSalazar/PyDead-BIB)
[![UB](https://img.shields.io/badge/UB-NO%20EXISTE-red.svg)](https://github.com/AndreeSalazar/PyDead-BIB)
[![Types](https://img.shields.io/badge/Tipos-ESTRICTOS-blue.svg)](https://github.com/AndreeSalazar/PyDead-BIB)

```
╔═══════════════════════════════════════════════════════════════════════════╗
║  v4.3 — Python Native Compiler — Sin CPython, Sin GIL, Sin Runtime        ║
║  100% Nativo — x86-64 + AVX2 — UB NO EXISTE — Tipos Estrictos             ║
╚═══════════════════════════════════════════════════════════════════════════╝
```

```text
Guido van Rossum: 'readability counts'
Dennis Ritchie:   'small is beautiful'
Grace Hopper:     'la máquina sirve al humano'
PyDead-BIB 2026:  hereda ADead-BIB v8.0 — Python nativo — 16→256 bits
                  UB NO EXISTE — Tipos Estrictos como Fortran
```

```text
Tu Código Python (.py)
        ↓
┌───────────────────────────────────────────┐
│       PyDead-BIB Compiler (pyb)           │
│                                           │
│  .py → Preprocessor → Import Eliminator   │
│              ↓                            │
│         Lexer → Parser                    │
│              ↓                            │
│       Type Inferencer (estático)          │
│              ↓                            │
│         IR (ADeadOp SSA-form)             │
│              ↓                            │
│         UB Detector (13+ tipos)           │
│              ↓                            │
│         Optimizer (DCE, Fold, Inline)     │
│              ↓                            │
│         BitResolver (64/128/256 bits)     │
│              ↓                            │
│         ISA Compiler + VEX Emitter        │
│         (FASM-style, x86-64/AVX2)         │
│              ↓                            │
│         PE / ELF / Po                     │
└───────────────────────────────────────────┘
        ↓
  .exe / .elf / .po
  (Machine Code Puro · Sin Runtime)
```

---

## Tabla de Contenidos

1. [¿Qué es PyDead-BIB?](#qué-es-pydead-bib)
2. [Comparación](#comparación)
3. [Instalación](#instalación)
4. [Uso Rápido](#uso-rápido)
5. [Step Compiler](#step-compiler)
6. [Sintaxis Python Soportada](#sintaxis-python-soportada)
7. [C ABI / FFI Nativo](#c-abi--ffi-nativo)
8. [UB Detection](#ub-detection--errores-en-compile-time)
9. [Arquitectura](#arquitectura)
10. [JIT KILLER v2.0](#jit-killer-v20-)
11. [Roadmap](#roadmap)
12. [Licencia](#licencia)

---

## ¿Qué es PyDead-BIB?

**PyDead-BIB** es el primer compilador que transforma código Python directamente a ejecutables nativos x86-64, sin depender de CPython, PyPy, ni ningún runtime.

- **Zero Runtime** — no hay intérprete, no hay VM, no hay GC
- **Zero GIL** — ownership estático elimina el Global Interpreter Lock
- **Zero Dependencies** — sin GCC, sin LLVM, sin linker externo
- **256-bit SIMD** — AVX2 automático para listas de floats
- **UB Detection** — 13+ tipos de errores detectados en compile-time

---

## Comparación

| Característica | CPython | PyPy | Nuitka | PyDead-BIB |
| --- | --- | --- | --- | --- |
| Sin runtime | ❌ 30MB | ❌ 200MB | ❌ 8MB | ✅ **0 bytes** |
| Sin GIL | ❌ | ❌ | ❌ | ✅ |
| Sin GCC/LLVM | ✅ | ✅ | ❌ | ✅ |
| 256-bit SIMD | ❌ | ❌ | ❌ | ✅ AVX2 |
| UB compile-time | ❌ | ❌ | ❌ | ✅ 13+ tipos |
| Hello World | 30MB | 200MB | 8MB | **~2KB** |

### Performance

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
  numpy:       import 50ms + BLAS overhead
  PyDead-BIB:  VMULPS ymm0 directo — 8 floats/ciclo
```

---

## Instalación

### Requisitos

- **Rust 1.75+** ([rustup.rs](https://rustup.rs))
- Windows 10/11 o Linux x64

### Build

```bash
# Clonar
git clone https://github.com/AndreeSalazar/PyDead-BIB.git
cd PyDead-BIB

# Compilar
cargo build --release

# El ejecutable está en:
# Windows: target/release/pyb.exe
# Linux:   target/release/pyb
```

### Agregar al PATH

```powershell
# Windows PowerShell
$env:PATH += ";$PWD\target\release"

# Linux/macOS
export PATH="$PATH:$PWD/target/release"
```

---

## Uso Rápido

```bash
# Compilación básica
pyb py archivo.py -o output.exe

# Target específico
pyb py archivo.py --target windows    # PE x64
pyb py archivo.py --target linux      # ELF x64
pyb py archivo.py --target fastos256  # FastOS 256-bit

# Compilar y ejecutar (JIT KILLER)
pyb run archivo.py
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

## Step Compiler

Ver las 13 fases de compilación en tiempo real:

```bash
pyb step archivo.py
```

```text
╔══════════════════════════════════════════════════════════════╗
║   PyDead-BIB Step Compiler — Deep Analysis Mode 💀🦈        ║
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

--- Phase 08: OPTIMIZER ---
[OPT]      constant folding: 3 expressions
[OPT]      dead code elimination: 2 statements

--- Phase 09: BIT RESOLVER ---
[BITS]     target: 64-bit (AVX2 available)

--- Phase 10: ISA COMPILER ---
[ISA]      127 bytes of machine code

--- Phase 11: ENCODER ---
[ENC]      x86-64 + VEX prefix

--- Phase 12: OUTPUT ---
[OUT]      PE x64: 2048 bytes

--- Phase 13: BINARY GUARDIAN ---
[BG]       stamp: 0xDEADBIB

✅ Compilation complete — 0.28ms
Sin CPython — Sin GIL — Sin runtime 💀🦈
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

### Standard Library Nativa

```python
import math    # sqrt, sin, cos, log → SIMD inline
import os      # getcwd, path.exists, mkdir → Win32/syscall
import sys     # platform, version, exit
import random  # randint, random → xorshift64
import json    # loads, dumps (stubs)

# File I/O nativo
with open("data.txt", "r") as f:
    content = f.read()     # → CreateFileA/ReadFile
```

---

## C ABI / FFI Nativo

PyDead-BIB permite llamar funciones C directamente desde Python usando `ctypes`, compilando a llamadas nativas Windows x64 ABI.

### Uso Básico

```python
import ctypes

# Cargar DLL
lib = ctypes.CDLL("mi_biblioteca.dll")

# Llamar función C
resultado = lib.suma(10, 20)    # → CALL [GetProcAddress]

# Tipos C
x = ctypes.c_int(42)            # → 32-bit integer
y = ctypes.c_double(3.14)       # → 64-bit float
```

### Calling Convention

```text
Windows x64 ABI (compatible MSVC):
  Args:         RCX, RDX, R8, R9, stack
  Return:       RAX (int), XMM0 (float)
  Shadow space: 32 bytes
  Callee-saved: RBX, RBP, RDI, RSI, R12–R15
```

### Python + C = Máximo Rendimiento

```python
# Python: lógica de alto nivel
def procesar_datos(datos):
    # C: operaciones críticas
    lib = ctypes.CDLL("simd_ops.dll")
    resultado = lib.procesar_avx2(datos, len(datos))
    return resultado

# PyDead-BIB compila ambos a machine code nativo
# Sin overhead de FFI — llamada directa CALL
```

> **Nota:** Ver `REPORTES.md` para el estado completo de C ABI y puntos faltantes.

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
| --- | --- | --- | --- |
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

## Arquitectura

```text
Python Source (.py)
        │
        ▼
┌─────────────────────────────────────────────────────┐
│  FRONTEND (PyDead-BIB v4.0)                         │
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

### Estructura del Proyecto

```text
PyDead-BIB/
├── Cargo.toml
├── src/rust/
│   ├── lib.rs                  # Biblioteca
│   ├── main.rs                 # CLI pyb
│   ├── frontend/python/        # Frontend Python
│   │   ├── py_preprocessor.rs
│   │   ├── py_import_resolver.rs
│   │   ├── py_lexer.rs
│   │   ├── py_parser.rs
│   │   ├── py_ast.rs
│   │   ├── py_types.rs
│   │   └── py_to_ir.rs
│   ├── middle/                 # Middle-end
│   │   ├── ir.rs
│   │   ├── ub_detector.rs
│   │   └── optimizer.rs
│   └── backend/                # Backend (heredado ADead-BIB)
│       ├── bit_resolver.rs
│       ├── isa_compiler.rs
│       ├── encoder.rs
│       └── output/
│           ├── pe.rs           # Windows PE
│           ├── elf.rs          # Linux ELF
│           └── po.rs           # FastOS
```

---

## JIT KILLER v2.0 💀🦈

**PyDead-BIB v4.0** incluye el ejecutor in-memory JIT KILLER v2.0:

```bash
pyb run archivo.py    # compile + execute in RAM — no .exe written
```

### 7 Mejoras Implementadas

| # | Mejora | Descripción | Impacto |
| --- | --- | --- | --- |
| 1 | **Step Mode Acelerado** | Timing real por fase en ms | Visibilidad total |
| 2 | **Pre-Resolved Dispatch Table** | IAT built once via LazyLock | 0 lookup per call |
| 3 | **Thermal Cache** | FNV-1a hash source, skip recompilation | ~0.001ms 2nd run |
| 4 | **Parallel Compilation** | Architecture ready for rayon | Nx faster |
| 5 | **Zero Copy Data** | .text PAGE_EXECUTE_READWRITE | Más seguro |
| 6 | **CPU Feature Detection** | CPUID: AVX2, SSE4.2, BMI2 | Compile exacto |
| 7 | **Instant Entry** | Pre-patch ALL fixups, then JMP | 0 runtime patch |

### Benchmark (AMD Ryzen 5 5600X)

```text
⚡ time-to-RAM: 0.305ms
  compile:  0.280ms (13 phases)
  JIT:      0.025ms
    alloc:  0.006ms (.text RWX, .data RW)
    patch:  0.005ms (instant image)
    exec:   0.005ms

CPU: AMD Ryzen 5 5600X 6-Core Processor
AVX2: ✓  SSE4.2: ✓  BMI2: ✓
```

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

## Targets de Salida

| Target | Formato | Descripción |
| --- | --- | --- |
| `--target windows` | `.exe` PE | Windows x64 nativo |
| `--target linux` | ELF | Linux x64 nativo |
| `--target fastos64` | `.po` v1 | FastOS 64-bit |
| `--target fastos128` | `.po` v2 | FastOS SSE 128-bit |
| `--target fastos256` | `.po` v8 | FastOS AVX2 256-bit |
| `--target all` | Multi | Todos simultáneos |

---

## Roadmap

### v1.0 — v1.5 ✅

- [x] 13/13 compilation phases
- [x] PE/ELF/FastOS output
- [x] print() real output via Win32 WriteFile
- [x] float print, arithmetic, if/elif/else, for, while
- [x] import math, os, sys, random, json
- [x] Lists, Dicts, Classes
- [x] `pyb install` / `pyb list` — native package manager

### v2.0 — Python Standard Total ✅

- [x] try/except/finally/raise
- [x] with statement — context managers
- [x] Inheritance — class bases, method override
- [x] List comprehensions
- [x] Decorators — @staticmethod, @classmethod
- [x] async/await — parser support
- [x] generators/yield
- [x] Dataclasses
- [x] f-strings con expressions

### v3.0 — Production Ready ✅

- [x] async/await state machine
- [x] generators/yield IR
- [x] @property / @lru_cache
- [x] numpy AVX2 SIMD — VMOVAPS/VADDPS/VMULPS/VSQRTPS
- [x] C extension compatibility — ctypes.CDLL
- [x] Optimizer pipeline — constant folding, DCE

### v4.0 — JIT KILLER ✅

- [x] Global State Tracker
- [x] VirtualAlloc Executor — in-memory JIT
- [x] Type Inferencer v2 — StructLayout
- [x] GPU Dispatch — 10 GPU IR instructions
- [x] JIT KILLER v2.0 — 7 performance improvements
- [x] **83/83 tests PASS**

### v5.0 — Distribution (próximo)

- [ ] PyPI-compatible distribution
- [ ] Linux ELF syscall stubs
- [ ] Cross-compilation targets
- [ ] Incremental compilation

---

## Licencia

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
| Enterprise / buyout | Negociable — contactar al autor |

Ver el archivo completo: [`TECHNE_LICENSE_v1.0.license`](TECHNE_LICENSE_v1.0.license)

---

## Autor

**Eddi Andree Salazar Matos**  
Lima, Perú 🇵🇪  
1 dev — Binary Is Binary 💀🦈

GitHub: [github.com/AndreeSalazar](https://github.com/AndreeSalazar)  
Email: <eddi.salazar.dev@gmail.com>

---

*"Python sin runtime — sin GIL — sin CPython — 16 hasta 256 bits"*  
*Licensed under Techne v1.0 — Lima, Perú 🇵🇪 — 2026*