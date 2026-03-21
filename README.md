# ADead-BIB v8.0 💀🦈

**Compilador Nativo: C99 · C++17 → Machine Code Puro · 256-bit Nativo**

> Zero Overhead · Zero Bloat · Zero Dead Code  
> Sin NASM · Sin LLVM · Sin GCC · Sin Clang  
> Sin libc externa · Sin linker · 100% Autosuficiente  
> FASM-style: bytes directos al CPU  
> 256-bit nativo: YMM/AVX2 · SoA natural · VEX prefix  
> `#include <header_main.h>` = TODO disponible

```
Tu Código (.c / .cpp)
        ↓
┌───────────────────────────────────────────┐
│          ADead-BIB Compiler (adb)         │
│                                           │
│  .c  → Preprocessor → Lexer → Parser      │
│  .cpp → Preprocessor → Lexer → Parser     │
│                    ↓                      │
│             CToIR / CppToIR               │
│                    ↓                      │
│             Program (IR)                  │
│                    ↓                      │
│             IsaCompiler                   │
│             (ADeadOp stream)              │
│                    ↓                      │
│             Optimizer                     │
│             (DCE, Fold, Inline, Peep)     │
│                    ↓                      │
│             BitResolver (v8.0)            │
│             (16/32/64/128/256 bits)       │
│                    ↓                      │
│             SoA Optimizer                 │
│             (float arr[8] → YMM register) │
│                    ↓                      │
│             Encoder + VEX Emitter         │
│             (FASM-style, x86-64/AVX2)     │
│                    ↓                      │
│             PE / ELF / Po                 │
└───────────────────────────────────────────┘
        ↓
  .exe / .elf / .po / .bin
  (Machine Code Puro · 256-bit)
```

---

## Tabla de Contenidos

1. [Filosofía](#filosofía)
2. [Instalación](#instalación)
3. [Inicio Rápido](#inicio-rápido)
4. [Step Compiler](#step-compiler)
5. [Frontends: C99 y C++17](#frontends-c99-y-c17)
6. [256-bit Pipeline (v8.0)](#256-bit-pipeline-v80)
7. [Referencia Técnica](#referencia-técnica)
8. [Estructura del Proyecto](#estructura-del-proyecto)
9. [Tamaños de Binario](#tamaños-de-binario)
10. [Resultados de Tests](#resultados-de-tests)
11. [Comandos CLI](#comandos-cli)
12. [GPU Backend](#gpu-backend)

---

## Filosofía

### ¿Por qué existe ADead-BIB?

Los compiladores industriales (MSVC, GCC, Clang/LLVM) son **referencias técnicas invaluables** — definieron cómo se compila C y C++ durante décadas. ADead-BIB los estudia, los respeta, y toma sus decisiones de ABI y calling convention como referencia. Lo que rechaza es el overhead que arrastran.

| Referencia | Lo que ADead-BIB toma | Lo que ADead-BIB rechaza |
|---|---|---|
| **MSVC** | Windows x64 ABI (RCX, RDX, R8, R9), shadow space 32 bytes, PE format | Runtime >100 KB, CRT implícito, excepciones SEH |
| **GCC** | System V AMD64 ABI (RDI, RSI, RDX, RCX), ELF format, optimizaciones agresivas | Múltiples backends indirectos, código generado inflado |
| **LLVM** | Concepto de IR intermedio, passes de optimización, instruction selection | IR genérico que no llega a bytes directos, overhead de abstracción |
| **FASM** | **Generación directa de bytes sin ensamblador externo** | — (FASM es la referencia que ADead-BIB sigue fielmente) |

**El resultado:** ADead-BIB genera binarios de **2–10 KB** donde GCC genera **50+ KB** y MSVC genera **100+ KB** para el mismo programa.

---

### Canon: C99 y C++98

ADead-BIB compila **C99** y **C++98** como estándares canónicos — representan las intenciones más claras de estos lenguajes.

**C99 — El Canon de C:**  
`int` = 32 bits, `char` = 8 bits, `long long` = 64 bits — tamaños exactos.  
Punteros = direcciones reales. `malloc/free` = control manual. `arr[i]` = `*(arr + i * sizeof(element))`.  
El programador sabe exactamente qué bytes genera cada línea.

**C++98 — El Canon de C++:**  
Classes = structs con métodos. Vtable solo cuando hay `virtual`. Templates = monomorphización.  
Constructores/Destructores = RAII sin overhead de excepciones.  
**Zero overhead principle** — lo que no usas, no pagas.

---

### ¿Por qué ADead-BIB está escrito en Rust?

Rust es el **guardián** que detecta los problemas que C y C++ no pueden ver en sí mismos:

| Problema en C/C++ | Rust lo detecta porque... |
|---|---|
| Buffer overflow | Ownership + bounds checking |
| Use-after-free | Borrow checker |
| Data races | Send + Sync traits |
| Null pointer | `Option<T>` obligatorio |
| Memory leaks | RAII + Drop |

**Rust no es el lenguaje que ADead-BIB compila — es el lenguaje que garantiza que ADead-BIB compile correctamente.**

---

### Eliminación Absoluta

Todo lo que no contribuye a la ejecución final se elimina:

```
Exceptions try/catch/throw     → error codes (cero stack unwinding)
RTTI (typeid, dynamic_cast)    → eliminado si no se usa
Smart pointers (unique/shared) → raw pointers (cero reference counting)
STL containers overhead        → inlined (solo operaciones usadas)
Funciones no llamadas          → eliminadas por DCE
Variables no leídas            → eliminadas por DCE
Branches inalcanzables         → eliminadas por constant folding
```

**Machine Code Puro** = solo las instrucciones x86-64 que el CPU necesita ejecutar. Nada más.

---

## Instalación

```bash
# 1. Clonar y compilar
git clone https://github.com/AndreeSalazar/ADead-BIB.git
cd ADead-BIB
cargo build --release

# 2. Agregar adb al PATH
#    Windows (PowerShell):
$env:Path += ";C:\ruta\a\ADead-BIB\target\release"
#    Permanente (Admin):
[Environment]::SetEnvironmentVariable('Path', $env:Path + ';C:\ruta\a\ADead-BIB\target\release', 'User')

#    Linux / macOS:
export PATH="$PATH:$HOME/ADead-BIB/target/release"
#    Permanente:
echo 'export PATH="$PATH:$HOME/ADead-BIB/target/release"' >> ~/.bashrc

#    FastOS: No necesita PATH — adb es nativo del sistema

# 3. Instalar headers globales
adb install

# 4. Verificar
adb --version
```

> `adb --version` muestra la ruta exacta y las instrucciones de PATH para tu sistema.

---

## Inicio Rápido

```bash
adb create hola          # Proyecto C
adb create mundo --cpp   # Proyecto C++
cd hola
adb run                  # Compila src/main.c → bin/hola.exe y ejecuta
# → "Hola desde hola"

adb cc hello.c -o hello.exe    # Compilar archivo suelto C
adb cxx app.cpp -o app.exe     # Compilar archivo suelto C++
adb run test.c                 # Compilar y ejecutar directo

adb gpu                        # GPU (SPIR-V directo)
adb step main.c                # Step Compiler — ver cada fase
```

### Estructura de Proyecto (`adb create`)

```
hola/
├── adb.toml           # Configuración del proyecto
│     [project]
│     name    = "hola"
│     version = "0.1.0"
│     lang    = "c"       # o "cpp"
│     standard= "c99"     # o "cpp17"
│
│     [build]
│     src     = "src/"
│     include = "include/"
│     output  = "bin/"
│
├── include/
│   └── header_main.h   ← todo disponible
├── src/
│   └── main.c
└── bin/                ← output de compilación
```

### Resolución de Headers (sin flags -I)

`#include <header.h>` busca en este orden:

1. `include/` del proyecto
2. `~/.adead/include/` (headers globales de `adb install`)
3. stdlib interna — C99/C++ completa (fallback)

Sin `-I flags`, sin CMake, sin Makefile.

---

## v8.0 — 256-bit Nativo + Autosuficiencia Total

```c
// Un solo include. Todo disponible. Sin linker. 256-bit nativo.
#include <header_main.h>

int main() {
    printf("Hello from ADead-BIB v8.0!\n");

    // SoA natural → detectado automáticamente → YMM register
    float pos_x[8] = {1,2,3,4,5,6,7,8};
    float pos_y[8] = {8,7,6,5,4,3,2,1};

    // 8 sumas en 1 instrucción: VADDPS ymm0, ymm0, ymm1
    for (int i = 0; i < 8; i++)
        pos_x[i] += pos_y[i];

    return 0;
}
```

- **Sin libc externa** — toda la stdlib C/C++ está implementada internamente
- **Sin linker** — unity build, todo compila a un solo IR y un solo binario
- **Tree shaking** — solo las funciones que usas llegan al binario final
- **256-bit nativo** — `float arr[8]` detectado como SoA → YMM register automático
- **BitResolver** — detecta automáticamente si compilar a 16/32/64/128/256 bits
- **VEX Emitter** — genera VEX prefix C4/C5 para instrucciones AVX2
- **Po v8.0** — header de 32 bytes con `ymm_used`, `soa_map`, `bg_stamp`
- **`fastos_*.h`** — headers individuales para control granular (`fastos_stdio.h`, `fastos_math.h`, etc.)

---

## Step Compiler

```bash
adb step main.c
```

Muestra cada fase del pipeline en tiempo real:

```
[SOURCE]   12 lines, 245 bytes

--- Phase 1: PREPROCESSOR ---
[PREPROC]  165 lines after preprocessing
[PREPROC]  #include <stdio.h> -> resolved internally

--- Phase 2: LEXER ---
[LEXER]    78 tokens generated
[LEXER]       1:0    Int                      OK
[LEXER]       1:1    Identifier("main")       OK

--- Phase 3: PARSER ---
[PARSER]   function 'main' (0 params, 3 stmts) OK
[PARSER]   Total: 1 functions, 0 structs, 28 typedefs

--- Phase 4: IR ---
[IR]       function 'main' -> 5 IR statements OK
[IR]         VarDecl { var_type: I32, name: "x", value: Some(Number(42)) }
[IR]         Println(String("Hello"))

--- Phase 5: UB DETECTOR ---
[UB]       No undefined behavior detected OK

--- Phase 6: CODEGEN (x86-64) ---
[CODEGEN]  127 bytes of machine code generated
[CODEGEN]  First 16 bytes:
[CODEGEN]    E9 00 00 00 00 55 48 89 E5 53 41 54 56 57 48 81

--- Phase 7: OUTPUT ---
[OUTPUT]   Target: Windows PE x86-64
[OUTPUT]   Code: 127 bytes  |  Data: 32 bytes
[OUTPUT]   Est. binary: ~1183 bytes
```

Funciona con C y C++: `adb step archivo.c` o `adb step archivo.cpp`

---

## Frontends: C99 y C++17

### C99 — Canon de C

**Pipeline:** `C source → Preprocessor → Lexer → Parser → AST → IR → IsaCompiler → Encoder → x86-64 → PE/ELF`

| Característica | Estado | Intención |
|---|---|---|
| Variables y tipos (`int`, `char`, `float`, `double`, ...) | ✅ | Tamaños exactos → registros correctos |
| Punteros y aritmética de punteros | ✅ | Direcciones reales, stride por `sizeof` |
| Arrays y acceso directo | ✅ | `arr[i]` = `*(arr + i * size)` |
| Structs, unions, typedefs | ✅ | Layout en memoria explícito |
| Enums | ✅ | Constantes enteras |
| Function pointers | ✅ | `call reg` directo |
| Preprocesador (`#include`, `#define`, `#ifdef`) | ✅ | 75+ headers de sistema inyectados |
| Control de flujo (`if`, `for`, `while`, `switch`) | ✅ | Branch → `jcc` directo |
| Recursión | ✅ | Stack frame real por llamada |
| `malloc/free` | ✅ | Enlace directo Win32 API / Linux syscall |
| Bitwise (`&`, `\|`, `^`, `<<`, `>>`, `~`) | ✅ | Instrucciones x86-64 directas |
| Operadores compuestos (`+=`, `-=`, `*=`, ...) | ✅ | In-place, sin temporales |

### C++17 — Canon de C++ (Zero Overhead)

**Pipeline:** `C++ source → Preprocessor → Lexer → Parser → AST → IR → IsaCompiler → Encoder → x86-64 → PE/ELF`

| Característica | Estado | Cómo lo compila ADead-BIB |
|---|---|---|
| Classes (campos, métodos, constructores, destructores) | ✅ | `struct` + funciones con `this` pointer |
| Herencia (single, multiple) | ✅ | Campos concatenados + vtable si hay `virtual` |
| Virtual functions | ✅ | Devirtualizadas cuando es posible |
| Templates (function, class) | ✅ | Monomorphización — solo instancias usadas |
| Namespaces (anidados, `using`) | ✅ | Prefijo de nombres, cero costo runtime |
| Operator overloading | ✅ | Inline a instrucciones directas |
| `auto`, `constexpr`, `nullptr`, `enum class` | ✅ | Resueltos en compilación |
| Range-for | ✅ | Loop con índice, sin iterador runtime |
| Lambdas | ✅ | Closure inline, captures resueltos |
| Casts (`static_cast`, `reinterpret_cast`, ...) | ✅ | Resueltos en compilación o eliminados |
| **Exceptions (try/catch/throw)** | ✅ → eliminados | Convertidos a error codes |
| **Smart pointers (unique_ptr, shared_ptr)** | ✅ → eliminados | Convertidos a raw pointers |
| **RTTI (typeid, dynamic_cast runtime)** | ✅ → eliminado | Si no se usa, no existe |

---

## 256-bit Pipeline (v8.0)

ADead-BIB v8.0 introduce soporte nativo para registros YMM (256-bit) via AVX2, con detección automática de patrones SoA (Structure-of-Arrays).

### BitResolver — Detección automática de ancho

El BitResolver analiza el IR y decide el ancho óptimo de compilación:

| Target | Bits | Registros | Uso |
|---|---|---|---|
| `boot16` | 16 | AX-DX | Stage1 bootloader |
| `boot32` | 32 | EAX-EDI | Stage2 protected mode |
| `fastos64` | 64 | RAX-R15 | FastOS standard |
| `fastos128` | 128 | XMM0-XMM15 | SSE/SSE4.2 vectorial |
| `fastos256` | 256 | **YMM0-YMM15** | **AVX2 nativo** ★ |

### SoA Optimizer — Vectorización natural

```c
// ADead-BIB detecta este patrón automáticamente:
float pos_x[8];   // 8 × float32 = 256 bits → YMM0
float pos_y[8];   // 8 × float32 = 256 bits → YMM1
float vel_x[8];   // 8 × float32 = 256 bits → YMM2

// Este loop se compila a UNA instrucción:
for (int i = 0; i < 8; i++)
    pos_x[i] += vel_x[i];
// → VADDPS ymm0, ymm0, ymm2    (8 sumas en 1 ciclo)
```

| Tipo | Elementos/YMM | Instrucción |
|---|---|---|
| `float` (32-bit) | 8 | VADDPS, VMULPS, VFMADD231PS |
| `double` (64-bit) | 4 | VADDPD, VMULPD |
| `int` (32-bit) | 8 | VPADDD, VPCMPEQD |

### VEX Emitter — Encoding directo

Genera VEX prefix C4/C5 para todas las instrucciones AVX2:

```
Instrucción              Bytes                  Encoding
──────────────────────────────────────────────────────────────
VADDPS ymm0,ymm0,ymm1   C5 FC 58 C1           VEX.256.0F 58 /r
VMOVAPS ymm0,[rbp-32]    C5 FC 28 45 E0        VEX.256.0F 28 /r
VFMADD231PS ymm0,y1,y2   C4 E2 75 B8 C2        VEX.256.66.0F38 B8 /r
VZEROUPPER               C5 F8 77              VEX.128.0F 77
```

### Po v8.0 — Header de 32 bytes

```
Offset  Size  Field       Description
──────────────────────────────────────────
0x00    4     magic       0x506F4F53 ('PoOS')
0x04    1     version     0x80 (v8.0)
0x05    1     bits        16/64/128/0xFF(256)
0x06    2     ymm_used    bitmask YMM0-YMM15
0x08    4     code_off    offset to .text
0x0C    4     code_size   size of .text
0x10    4     data_off    offset to .data
0x14    4     data_size   size of .data
0x18    4     soa_map     offset to SoA table
0x1C    4     bg_stamp    BG verification hash
```

---

## Referencia Técnica

### Calling Conventions

```
Windows x64 (referencia MSVC):
  Args:         RCX, RDX, R8, R9, stack
  Ret:          RAX (int), XMM0 (float)
  Shadow space: 32 bytes
  Callee-saved: RBX, RBP, RDI, RSI, R12–R15

System V AMD64 (referencia GCC):
  Args:         RDI, RSI, RDX, RCX, R8, R9, stack
  Ret:          RAX (int), XMM0 (float)
  Red zone:     128 bytes
  Callee-saved: RBX, RBP, R12–R15
```

ADead-BIB detecta el target automáticamente y usa la convención correcta.

---

### Encoding FASM-Style (Bytes Directos)

```
Instrucción        Bytes            Encoding
───────────────────────────────────────────────────
mov rax, rbx       48 89 D8         REX.W + MOV r/m64, r64
add rax, 42        48 83 C0 2A      REX.W + ADD r/m64, imm8
call printf        E8 xx xx xx xx   CALL rel32
ret                C3               RET
push rbp           55               PUSH r64
sub rsp, 32        48 83 EC 20      SUB r/m64, imm8
xor eax, eax       31 C0            XOR r32, r32
```

---

### Optimizaciones

| Optimización | Referencia | Qué hace |
|---|---|---|
| Dead Code Elimination | GCC -O1, LLVM `dce` | Elimina funciones, variables y branches no usados |
| Constant Folding | GCC -O1, LLVM `constprop` | `2 + 3 * 4` → `14` en compilación, 0 instrucciones runtime |
| Inlining | GCC -O2, LLVM `inline` | Funciones pequeñas expandidas en el caller |
| Peephole | GCC -O2 | `add reg, 1` → `inc reg`, `mov reg, reg` eliminado |
| Register Allocation | GCC/LLVM `regalloc` | Temporales en R10–R15, minimiza push/pop |
| Strength Reduction | GCC -O2 | `x * 0` → `0`, `x * 2` → `shl x, 1` |
| Dynamic Stack Frame | ADead-BIB propio | Stack frame calculado exacto, no 128 bytes fijos |

---

### ISA Layer: ADeadOp → x86-64

| ADeadOp | Descripción | x86-64 |
|---|---|---|
| `Mov { dst, src }` | Mover datos | `89/8B` + ModR/M |
| `Add { dst, src }` | Suma | `01/03` + ModR/M |
| `Sub { dst, src }` | Resta | `29/2B` + ModR/M |
| `Mul { src }` | Multiplicación | `F7 /4` |
| `Div { src }` | División | `F7 /6` |
| `Shl { dst, amount }` | Shift left | `C1 /4 imm8` |
| `Shr { dst, amount }` | Shift right | `C1 /5 imm8` |
| `Cmp { left, right }` | Comparar | `39/3B` |
| `Jmp { target }` | Salto | `EB/E9` |
| `Je/Jne/Jl/Jg` | Saltos condicionales | `74/75/7C/7F` |
| `Call { target }` | Llamar función | `E8 rel32` |
| `Ret` | Retornar | `C3` |
| `Push { src }` | Push stack | `50+r` |
| `Pop { dst }` | Pop stack | `58+r` |
| `Cli` / `Sti` | Interrupciones | `FA` / `FB` |
| `Hlt` | Halt CPU | `F4` |
| `In { port, dst }` | Leer puerto I/O | `E4/EC` |
| `Out { port, src }` | Escribir puerto I/O | `E6/EE` |

---

## Estructura del Proyecto

```
ADead-BIB/
├── src/rust/
│   ├── main.rs                        # CLI driver (adb)
│   ├── lib.rs                         # Exports públicos
│   ├── builder.rs                     # Orchestrator del pipeline
│   │
│   ├── cli/                           # Terminal UI (ANSI, phase bars)
│   │
│   ├── frontend/                      # Frontends C99 y C++17
│   │   ├── ast.rs                     # IR compartido
│   │   ├── types.rs                   # Sistema de tipos
│   │   ├── type_checker.rs            # Análisis estático
│   │   ├── c/                         # C99: lexer, parser, AST, IR, preprocessor, stdlib
│   │   └── cpp/                       # C++: lexer, parser, AST, IR, preprocessor, STL stubs
│   │
│   ├── preprocessor/                  # Sin CMake, sin linker
│   │   ├── resolver.rs                # Header resolution
│   │   ├── dedup.rs                   # Symbol deduplication
│   │   └── expander.rs                # C++17 → C++98 canon (34 features)
│   │
│   ├── stdlib/                        # Standard library propia
│   │   ├── header_main.rs             # header_main.h — hereda TODO
│   │   ├── c/                         # C99: stdio, stdlib, string, math...
│   │   └── cpp/                       # C++: iostream, vector, map, memory...
│   │
│   ├── middle/                        # Middle-end (IR avanzado)
│   │   ├── ir/                        # SSA IR
│   │   ├── ub_detector/               # 21+ tipos de UB detection
│   │   │   ├── null_check.rs          # NullPointerDereference
│   │   │   ├── bounds_check.rs        # ArrayOutOfBounds
│   │   │   ├── overflow_check.rs      # IntegerOverflow / DivByZero
│   │   │   ├── uninit_check.rs        # UninitializedVariable
│   │   │   ├── useafter_check.rs      # UseAfterFree / DanglingPtr
│   │   │   ├── lifetime.rs            # DoubleFree / lifetime analysis
│   │   │   └── format_check.rs        # FormatStringMismatch
│   │   ├── analysis/                  # CFG, dominator tree, loops
│   │   └── passes/                    # Transform passes (DCE, GVN, LICM, inline...)
│   │
│   ├── optimizer/                     # AST-level optimizations
│   │   ├── const_fold.rs / const_prop.rs / dead_code.rs
│   │   ├── inline_exp.rs / branchless.rs / simd.rs
│   │   └── binary_optimizer.rs        # Binary-level size optimization
│   │
│   ├── isa/                           # ISA Layer — el núcleo
│   │   ├── isa_compiler.rs            # Program IR → ADeadOp stream
│   │   ├── encoder.rs                 # ADeadOp → x86-64 bytes (FASM-style)
│   │   ├── decoder.rs                 # x86-64 → ADeadOp (disassembly)
│   │   ├── optimizer.rs               # Peephole optimization
│   │   ├── reg_alloc.rs               # Register allocator
│   │   ├── bit_resolver.rs            # v8.0: BitTarget 16→256, SoA detection
│   │   ├── soa_optimizer.rs           # v8.0: SoA pattern detection (float[8]→YMM)
│   │   ├── ymm_allocator.rs           # v8.0: YMM0-YMM15 register allocation
│   │   ├── vex_emitter.rs             # v8.0: VEX C4/C5 prefix encoding
│   │   └── compiler/                  # expressions, statements, control_flow, functions, arrays
│   │
│   ├── output/                        # Binary output (sin linker)
│   │   ├── pe.rs                      # Windows PE (.exe)
│   │   ├── elf.rs                     # Linux ELF
│   │   └── po.rs                      # FastOS .po v8.0 (32-byte header, YMM/SoA/BG)
│   │
│   ├── backend/
│   │   ├── cpu/                       # x86-64: PE, ELF, flat binary, MicroVM, syscalls, Win32
│   │   └── gpu/                       # Vulkan, SPIR-V, CUDA, HIP, unified CPU↔GPU pipeline
│   │
│   ├── bg/                            # Binary Guardian (security policy)
│   ├── cache/                         # FastOS.BIB Cache v2 (FNV-1a)
│   ├── runtime/                       # CPU/GPU feature detection + dispatch
│   └── toolchain/                     # Calling conventions, GCC/Clang/MSVC compat, name mangling
│
├── examples/
│   ├── c/                             # 34 archivos C99  — todos compilan ✅
│   ├── cpp/                           # 22 archivos C++  — todos compilan ✅
│   ├── boot/                          # Boot sectors, kernels
│   └── gpu/                           # GPU compute shaders
│
├── Test-Canon/                        # Canon verification suite (48 tests)
├── Test-UB-Global/                    # Global UB test suite
├── ub_tests/                          # UB detection tests
├── EXTENSION/                         # VS Code extension
├── Cargo.toml                         # 100% Rust, sin deps de C/C++
├── ARCHITECTURE.md
└── README.md
```

---

## Tamaños de Binario

| Programa | ADead-BIB | GCC -Os | MSVC /O1 |
|---|---|---|---|
| Hello World | **2.0 KB** | ~50 KB | ~100 KB |
| Counter + printf | **2.0 KB** | ~50 KB | ~100 KB |
| Recursión (fib, power) | **2.5 KB** | ~50 KB | ~100 KB |
| Classes + OOP | **3.0 KB** | ~55 KB | ~110 KB |
| Templates | **3.5 KB** | ~55 KB | ~110 KB |
| Stdlib largo (~100 funcs) | **42 KB** | ~200 KB | ~300 KB |

Sin CRT. Sin exception handling tables. Sin RTTI. Sin debug info por defecto. Solo machine code puro.

---

## Resultados de Tests

| Frontend | Archivos | Pasan | Tasa |
|---|---|---|---|
| C99 examples | 34 | 34 | **100%** ✅ |
| C++ examples | 22 | 22 | **100%** ✅ |
| C99 Canon | 18 | 18 | **100%** ✅ |
| C++98 Canon | 15 | 15 | **100%** ✅ |
| Integration tests | 18 | 18 | **100%** ✅ |
| FASE tests (C99+C++17+PE) | 19 | 19 | **100%** ✅ |
| **Total Rust tests** | **580** | **580** | **100%** ✅ |

```
C99 Canon (18):   tipos, punteros, arrays, structs, unions, enums,
                  typedef, control, funciones, function pointers,
                  preprocesador, bitwise, casting, scope, strings,
                  malloc, sizeof, expresiones complejas — ALL PASS ✅

C++98 Canon (15): clases, herencia, virtual/polimorfismo, templates,
                  namespaces, operator overload, referencias,
                  const correctness, constructores, static members,
                  punteros objetos, enum class, STL — ALL PASS ✅

Integration (18): header_main.h C/C++, fastos_*.h, symbol registries,
                  no-linker verification, full E2E programs — ALL PASS ✅
```

---

## Comandos CLI

```bash
# Proyectos
adb create hola              # Nuevo proyecto C
adb create hola --cpp        # Nuevo proyecto C++
adb build                    # Compilar proyecto (lee adb.toml)
adb run                      # Compilar y ejecutar proyecto

# C99
adb cc hello.c -o hello.exe
adb cc main.c                # → main.exe automático

# C++
adb cxx app.cpp -o app.exe
adb cxx main.cpp             # → main.exe automático

# Auto-detect
adb build program.c          # Detecta .c → C99
adb build program.cpp        # Detecta .cpp → C++
adb run test.c               # Compilar y ejecutar

# Headers globales
adb install                  # Instala headers en ~/.adead/include/
adb include                  # Muestra ruta de headers

# Flat Binary (OS/Kernel)
adb cc kernel.c -o kernel.bin --flat
adb cc boot.c -o boot.bin --flat16 --org=0x7C00 --size=512

# FastOS targets (v8.0)
adb cc kernel.c --target fastos64 -o kernel.po
adb cc kernel.c --target fastos128 -o kernel.po
adb cc kernel.c --target fastos256 -o kernel.po   # 256-bit YMM/AVX2
adb cc kernel.c --target boot16 -o stage1.bin
adb cc kernel.c --target boot32 -o stage2.bin

# Binarios mínimos
adb nano output.exe          # PE más pequeño posible
adb micro output.exe         # PE32 < 256 bytes

# GPU
adb gpu                      # Detectar GPU + generar shader
adb spirv matmul 1024        # SPIR-V compute shader

# Step Compiler
adb step program.c           # Visualizar pipeline paso a paso

# MicroVM
adb vm program.c             # Compilar a MicroVM bytecode (4-bit ops)

# Vulkan / CUDA
adb vulkan shader.comp       # Compilar + ejecutar con Vulkan runtime
adb cuda kernel.cu           # CUDA code generation

# CPU↔GPU Hybrid
adb unified program.c        # CPU↔GPU auto-dispatch pipeline

# Auto-detect por extensión
adb program.c                # → C99
adb program.cpp              # → C++
```

---

## GPU Backend

```
Código ADead → AST → SPIR-V bytes (directo, sin IR intermedio)
```

```python
# FFI GPU (Python)
from FFI_GPU import GPU

gpu = GPU()
A = gpu.buffer(data_a)
B = gpu.buffer(data_b)
C = gpu.buffer(size=N)

kernel = gpu.load_spirv("vecadd.spv")
gpu.dispatch(kernel, A, B, C, groups=(N//256, 1, 1))
result = C.read()
```

---

## Autor

**Eddi Andreé Salazar Matos**  
eddi.salazar.dev@gmail.com  
Hecho en Perú 🇵🇪

## Licencia

**GNU General Public License v2.0**

```
Copyright (C) 2024–2026 Eddi Andreé Salazar Matos
```

---

**ADead-BIB v8.0: C99 · C++17 → Machine Code Puro · 256-bit Nativo 💀🦈**

```
MSVC, GCC, LLVM  = referencias técnicas estudiadas y respetadas
FASM             = el modelo de encoding directo que ADead-BIB sigue
Rust             = el guardián que garantiza que el compilador nunca falle
header_main.h    = un include, todo disponible
adb create       = como cargo new, pero para C/C++
YMM/AVX2         = 256-bit nativo, SoA natural, VEX prefix
```

> *"C = intención absoluta del programador*  
> *C++ = zero overhead principle*  
> *Rust = guardián de correctitud*  
> *FASM = bytes directos al CPU*  
> *YMM = 256 bits nativos, 8 floats en paralelo*  
> *ADead-BIB = único en el mundo 💀🦈 🇵🇪"*

```bash
adb create hola
cd hola
adb run
# → "Hola desde hola" — 2KB, sin GCC, sin linker
```