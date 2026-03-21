# PyDead-BIB
# ADead-BIB v8.0
> **Python x86-64 Nativo - Sin CPython - Sin GIL - Sin Runtime**
**Compilador Nativo: C99 + C++17 + Machine Code Puro + 256-bit Nativo**
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-Techne%20v1.0-purple.svg)](TECHNE_LICENSE_v1.0.license)
[![Version](https://img.shields.io/badge/version-4.0.0-green.svg)](https://github.com/Andreesalazar/PyDead-BIB)
[![Tests](https://img.shields.io/badge/Tests-83%2F83%20PASS-brightgreen.svg)](https://github.com/Andreesalazar/PyDead-BIB)
[![JIT](https://img.shields.io/badge/JIT%20KILLER-v2.0-ff0off.svg)](https://github.com/Andreesalazar/PyDead-BIB)
> Zero Overhead · Zero Bloat · Zero Dead Code
> Sin NASM · Sin LLVM · Sin GCC · Sin Clang
> Sin libc externa · Sin linker · 100% Autosuficiente
> FASM-style: bytes directos al CPU
> 256-bit nativo: YMM/AVX2 · SOA natural · VEX prefix
> #include <header_main

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

```powershell
# Windows PowerShell
$env:PATH += ";$PWD\target\release"

# Linux/macOS
export PATH="$PATH:$PWD/target/release"
```

## Uso Rápido

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

✅ Frontend compilation complete
Sin CPython — Sin GIL — Sin runtime 💀🦈
```

### Crear Proyecto

pyb create mi_app
cd mi_app
pyb run src/main.py

Estructura generada:

```text
mi_app/
├── pyb.toml        # Configuración del proyecto
└── src/
    └── main.py     # Código fuente


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

## Licencia

Este software esta protegido bajo la **TECHNE LICENSE v1.0**.

> *"El arte pertenece al artesano. Su uso da frutos que deben compartirse."*

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