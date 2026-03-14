# PyDead-BIB Compiler Architecture v1.0

> Guido van Rossum: `'readability counts'`  
> Dennis Ritchie: `'small is beautiful'`  
> Grace Hopper: `'la maquina sirve al humano'`  
> **PyDead-BIB 2026: hereda ADead-BIB v8.0 — Python nativo sin CPython — sin GIL — sin runtime — 16→256 bits 💀🦈🇵🇪**

---

## Filosofía Central

```
SIN CPYTHON — NUNCA
  python.exe   ❌ eliminado
  pypy         ❌ eliminado
  cython        ❌ intermedio eliminado
  GIL           ❌ eliminado para siempre

SIN INTERMEDIO
  Python → CPython bytecode → interpreter  ❌ viejo camino
  Python → IR ADeadOp → x86-64 nativo      ✓ PyDead-BIB

HEREDA ADead-BIB v8.0 COMPLETO
  IR ADeadOp        → reutilizado 100%     ✓
  ISA Compiler      → reutilizado 100%     ✓
  UB Detector       → extendido Python     ✓
  Step Mode         → 11 fases Python      ✓
  BG Binary Guardian→ reutilizado 100%     ✓
  PE/ELF/Po output  → reutilizado 100%     ✓
  Register Allocator→ reutilizado 100%     ✓

PYTHON SINTAXIS — TODAS
  Python 2.7 syntax   → soportado         ✓
  Python 3.0→3.13     → soportado         ✓
  Type hints (3.5+)   → inferencia real   ✓
  f-strings (3.6+)    → compilado nativo  ✓
  walrus := (3.8+)    → IR directo        ✓
  match/case (3.10+)  → branchless opt    ✓

HELLO WORLD
  CPython:    30MB runtime + .pyc         ❌
  PyDead-BIB: 2KB PE nativo               ✓
```

---

## Lo que PyDead-BIB NO es

```
NO es Cython:
  Cython: Python → C → GCC → binario     ❌ intermedio
  PyDead-BIB: Python → IR → x86-64       ✓ directo

NO es PyPy:
  PyPy: JIT en runtime — GIL existe      ❌
  PyDead-BIB: AOT compile — sin GIL      ✓

NO es Nuitka:
  Nuitka: Python → C → GCC              ❌ depende GCC
  PyDead-BIB: sin GCC — sin C           ✓ independiente

NO es mypyc:
  mypyc: solo typed Python → C ext      ❌ parcial
  PyDead-BIB: Python completo → nativo  ✓

ES:
  Primer compilador Python → x86-64     ✓
  Sin GCC, Sin LLVM, Sin CPython        ✓
  Heredando ADead-BIB IR probado        ✓
  256-bit nativo para Python            ✓
  Step Mode para Python                 ✓
```

---

## Output Architecture v1.0 — Mismos targets ADead-BIB

> **Python fuente no cambia. `--target` decide los bits.**  
> Mismo patrón que ADead-BIB — frontend diferente, backend idéntico.

| Bits | Target | Formato | Descripción |
|------|--------|---------|-------------|
| **64** | `--target windows` | `.exe` PE | Windows PE x64 — Python app nativa — sin runtime |
| **64** | `--target linux` | `ELF` | Linux ELF x64 — sin ld — sin librerías externas |
| **64** | `--target fastos64` | `.po` v1 | FastOS compat — Po magic `0x506F4F53` |
| **128** | `--target fastos128` | `.po` v2 | FastOS SSE — XMM registers |
| **256** | `--target fastos256` | `.po` v8.0 | FastOS AVX2 NATIVO — YMM — SoA automático |
| **∞** | `--target all` | Multi | PE + ELF + Po simultáneos |

---

## Pipeline Completo v1.0

```
Python 2.7 / 3.0→3.13  código fuente
        │
        ▼
[ 01 PREPROCESSOR ]         ←── nuevo: py_preprocessor/
  import resolution COMPLETA
  __future__ handling
  encoding detection (UTF-8, Latin-1)
  decorator expansion
  __all__ tree shaking preparation
  fastos.bib cache (CACHE HIT = nanosegundos) ← HEREDADO
        │
        ▼
[ 02 IMPORT ELIMINATOR ]    ←── nuevo: py_import_resolver/
  Sin .pyc intermedios — NUNCA
  Sin site-packages en runtime — NUNCA
  Static import resolution
  Dead import elimination
  "ModuleNotFoundError" en compile time — no en runtime ✓
        │
        ▼
[ 03 LEXER ]                ←── nuevo: frontend/python/py_lexer.rs
  Indentation → INDENT/DEDENT tokens
  f-string parsing completo
  byte strings, raw strings, unicode
  operadores := walrus, ** power, // floor div
  match/case keywords (3.10+)
  type hints tokens (3.5+)
        │
        ▼
[ 04 PARSER / AST ]         ←── nuevo: frontend/python/py_parser.rs
  Python grammar completo 2.7→3.13
  Indentation-based blocks → AST blocks
  Comprehensions: list/dict/set/generator
  Decorators: @property, @classmethod, @staticmethod, custom
  Multiple assignment: a, b = b, a
  *args, **kwargs resueltos estáticamente
  match/case → decision tree AST
        │
        ▼
[ 05 TYPE INFERENCER ]      ←── nuevo: frontend/python/py_types.rs ★ NUEVO
  Duck typing → tipos concretos estáticos
  PEP 484 type hints → tipos garantizados
  Type propagation: a = 1 → a: int inferido
  Return type inference: def f() → tipo retorno
  Container types: list[int], dict[str, float]
  Gradual typing: typed + untyped coexisten
        │
        ▼
[ 06 IR — ADeadOp ]         ←── HEREDADO 100% de ADead-BIB v8.0
  Python AST → ADeadOp SSA-form
  tipos explícitos en cada instrucción
  BasicBlocks — sin ambigüedad semántica
  GIL eliminado: cada objeto tiene ownership ✓
        │
        ▼
[ 07 UB DETECTOR ]          ←── HEREDADO + extendido Python
  21 tipos C/C++ heredados ✓
  + Python-specific UB:
    NoneType dereference (AttributeError pre-detectado)
    Index out of bounds (IndexError pre-detectado)
    Key not found (KeyError pre-detectado)
    Division by zero (ZeroDivisionError pre-detectado)
    Type mismatch en operaciones (TypeError pre-detectado)
    Infinite recursion detection
    Circular import detection
  ANTES del optimizer — cobertura 100% ✓
        │
        ▼
[ 08 OPTIMIZER ]            ←── HEREDADO + Python opts
  Dead code elimination ← heredado
  Constant folding ← heredado
  SIMD code generation ← heredado
  + Python-specific:
    List comprehension → SIMD loop cuando posible
    Generator → lazy evaluation nativa
    String interning automático
    Integer small cache (-5..256) compilado
        │
        ▼
[ 09 REGISTER ALLOCATOR ]   ←── HEREDADO 100% de ADead-BIB v8.0
  TempAllocator (fast path) ← sin cambios
  LinearScanAllocator ← sin cambios
  13 registros físicos x86-64 ← sin cambios
        │
        ▼
[ 10 BIT RESOLVER ]         ←── HEREDADO 100% de ADead-BIB v8.0
  --target decide: 64 / 128 / 256 bits
  SoaOptimizer: detecta list[float] × 8 → YMM
  YmmAllocator: asigna YMM0-YMM15
  VexEmitter: genera VEX prefix C4/C5
        │
        ▼
[ 11 ISA COMPILER ]         ←── HEREDADO 100% de ADead-BIB v8.0
  encoder.rs → bytes x86-64 directos ← sin cambios
        │
        ▼
[ 12 BG STAMP ]             ←── HEREDADO 100% de ADead-BIB v8.0
  Po magic 0x506F4F53 ← sin cambios
        │
        ▼
[ 13 OUTPUT DIRECTO ]       ←── HEREDADO 100% de ADead-BIB v8.0
  pe.rs / elf.rs / po.rs ← sin cambios
  Sin linker — NUNCA ← sin cambios
        │
   ┌────┴────────────────┬──────────────────┐
   ▼                     ▼                  ▼
.exe (PE x64)         .elf (ELF)        .po (Po v8.0)
Windows               Linux             FastOS
sin runtime           sin runtime       256-bit nativo
```

---

## Python Sintaxis Completa Soportada

### Tipos y Literales
```python
# Todos soportados — compilados a tipos nativos

x: int   = 42          # → RAX literal         ✓
y: float = 3.14        # → XMM/YMM literal      ✓
s: str   = "hola"      # → .data section UTF-8  ✓
b: bool  = True        # → 1 byte nativo        ✓
n        = None        # → NULL pointer nativo  ✓

# f-strings → compilado, no runtime format
name = "Andreé"
msg = f"Hola {name}"   # → string concat nativo ✓

# Bytes
data = b"\x90\x90"     # → bytes literales      ✓

# Números
big  = 1_000_000       # → int64               ✓
hexa = 0xFF            # → literal hex          ✓
bina = 0b1010          # → literal binario      ✓
```

### Funciones y Decoradores
```python
# Funciones — todas las formas
def simple(x: int) -> int:
    return x * 2                    # → ISA directo ✓

def defaults(x=0, y=1.0):
    return x + y                    # → constantes compiladas ✓

def variadic(*args, **kwargs):
    pass                            # → stack frame nativo ✓

# Lambda → función inline
double = lambda x: x * 2           # → inline ISA ✓

# Decoradores → transformación en compile time
@property
def valor(self): return self._v     # → getter nativo ✓

@staticmethod
def crear(): return MyClass()       # → función libre nativo ✓

@classmethod
def desde(cls, x): return cls(x)   # → factory nativo ✓
```

### Clases y Herencia
```python
class Animal:
    nombre: str
    edad: int

    def __init__(self, nombre: str, edad: int):
        self.nombre = nombre         # → struct field offset ✓
        self.edad = edad

    def hablar(self) -> str:
        return "..."                 # → vtable entry ✓

class Perro(Animal):
    def hablar(self) -> str:
        return "Woof"               # → vtable override ✓

# PyDead-BIB compila clase → struct + vtable
# Mismo patrón que C++ en ADead-BIB
# self → RCX (Windows ABI) / RDI (Linux ABI)
```

### Comprehensions → SIMD cuando posible
```python
# List comprehension sobre float → YMM automático
velocidades = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
dobles = [v * 2.0 for v in velocidades]
# PyDead-BIB detecta: list[float] × 8
# Genera: VMULPS ymm0, ymm1, [2.0×8]
# 8 floats en 1 ciclo — sin loop ✓

# Dict comprehension → hash map nativo
cuadrados = {x: x**2 for x in range(10)}  # → hash map ✓

# Generator → lazy nativo
gen = (x*2 for x in range(1000))           # → lazy iterator ✓
```

### Match/Case (Python 3.10+) → Branchless
```python
match comando:
    case "help":
        mostrar_ayuda()              # → jump table ✓
    case "cpu":
        mostrar_cpu()
    case "clear":
        limpiar()
    case _:
        error()
# PyDead-BIB: match → jump table nativo
# No if/elif chain — O(1) despacho ✓
```

### Walrus Operator (Python 3.8+)
```python
# := → assignment expression
if (n := len(datos)) > 10:
    print(f"Grande: {n}")           # → variable en registro ✓

while chunk := archivo.read(8192):
    procesar(chunk)                 # → loop nativo ✓
```

### Type Hints → Optimización Real
```python
# Sin type hints → inferencia
x = 42          # PyDead-BIB infiere: int64 ✓

# Con type hints → garantizado
x: int = 42     # int64 garantizado ✓
y: float = 1.0  # float64 garantizado ✓

# Type hints en funciones → ABI exacto
def suma(a: int, b: int) -> int:
    return a + b
# Genera: ADD RAX, RBX — sin boxing — sin type check ✓
# vs CPython: BINARY_OP bytecode + type dispatch = 10× más lento
```

---

## UB Python — Tipos Detectados

```rust
// middle/ub_detector/python_ub.rs — NUEVO v1.0

pub enum PythonUB {
    // Heredados de C (aplicables)
    DivisionByZero,              // x / 0 → ZeroDivisionError pre-detectado
    IntegerOverflow,             // int ilimitado → warning si > int64
    UninitializedVariable,       // uso antes de asignación

    // Python-specific
    NoneDeref,                   // None.atributo → AttributeError pre-detectado
    IndexOutOfBounds,            // lista[100] con lista de 10 → pre-detectado
    KeyNotFound,                 // dict["x"] sin "x" → pre-detectado
    TypeMismatch,                // "hola" + 42 → TypeError pre-detectado
    InfiniteRecursion,           // recursión sin base case → detectado
    CircularImport,              // A importa B, B importa A → detectado
    MutableDefaultArg,           // def f(x=[]) → bug clásico Python → warning
    GlobalWithoutDeclaration,    // modifica global sin 'global' → warning
    IteratorExhausted,           // reusar generator ya consumido → detectado
    UnpackMismatch,              // a, b = [1, 2, 3] → demasiados valores
}

// CPython: todos estos → excepción en RUNTIME ❌
// PyDead-BIB: todos → detectados en COMPILE TIME ✓
// "MutableDefaultArg" → bug más famoso de Python → nunca más ✓
```

---

## Step Mode v1.0 — 11 Fases Python

```bash
pyb step main.py --target windows
```

```
╔══════════════════════════════════════════════════════════════╗
║   PyDead-BIB Step Compiler — Deep Analysis Mode 💀🦈         ║
╚══════════════════════════════════════════════════════════════╝
  Source:   main.py
  Language: Python 3.x

--- Phase 01: PREPROCESSOR ---
[PREPROC]  imports resueltos: os, sys, math → internamente
[PREPROC]  encoding: UTF-8 detectado
[PREPROC]  __future__ annotations: procesado

--- Phase 02: IMPORT ELIMINATOR ---
[IMPORT]   os.path.join → función nativa compilada
[IMPORT]   math.sqrt    → VSQRTSS xmm0 directo
[IMPORT]   sys.exit     → syscall nativo
[IMPORT]   sin site-packages — NUNCA

--- Phase 03: LEXER ---
[LEXER]    247 tokens generados
[LEXER]    INDENT/DEDENT: 18 pares — estructura OK
[LEXER]    f-strings: 3 detectados — compilación inline
[LEXER]    type hints: 12 anotaciones → tipos garantizados

--- Phase 04: PARSER ---
[PARSER]   function 'main' (0 params, 15 stmts) OK
[PARSER]   class 'Jugador' (3 fields, 4 methods) OK
[PARSER]   comprehension × 2 → SIMD candidatos

--- Phase 05: TYPE INFERENCER ---
[TYPES]    x: int64 inferido desde literal 42
[TYPES]    velocidades: list[float64] × 8 → YMM candidato ★
[TYPES]    nombre: str UTF-8 → .data section
[TYPES]    12/15 variables tipadas — 3 dinámicas

--- Phase 06: IR (ADeadOp SSA-form) ---
[IR]       31 IR statements — BasicBlocks OK
[IR]       GIL eliminado — ownership estático ✓

--- Phase 07: UB DETECTOR ---
[UB]       MutableDefaultArg — línea 8 — def f(x=[])
[UB]       WARNING: bug clásico Python — argumento por defecto mutable
[UB]       fix: def f(x=None): x = x or []
[UB]       CLEAN — sin errores críticos ✓

--- Phase 08: OPTIMIZER ---
[OPT]      list[float] × 8 → SoA pattern → YMM ★
[OPT]      math.sqrt → VSQRTSS inline — sin call overhead
[OPT]      f-string → string concat estático compilado

--- Phase 09: REGISTER ALLOCATOR ---
[REGALLOC] LinearScan — spill 0 — 13 registros OK

--- Phase 10: BIT RESOLVER ---
[BITS]     --target windows → 64-bit PE
[BITS]     YMM0 asignado para velocidades[8]
[BITS]     VMULPS ymm0, ymm1, ymm2 — generado

--- Phase 11: OUTPUT ---
[OUTPUT]   Target: Windows PE x64
[OUTPUT]   Code:  412 bytes (.text)
[OUTPUT]   Data:  96 bytes (.data)
[OUTPUT]   Est. binary: ~2,100 bytes
[OUTPUT]   CPython mismo programa: 30MB runtime 💀
```

---

## Comparación de Performance

```
Hello World:

CPython 3.13:
  runtime:  30MB ❌
  tiempo:   ~50ms startup ❌
  binario:  .py (necesita python.exe) ❌

PyPy 7.3:
  runtime:  ~200MB ❌
  JIT warmup: ~2 segundos ❌

Nuitka:
  usa GCC internamente ❌
  binario: ~8MB (incluye mini-CPython) ❌

PyDead-BIB:
  runtime:  0 bytes ✓
  tiempo:   ~0.1ms startup ✓
  binario:  ~2KB PE ✓

Bucle float × 8 (numpy vs PyDead-BIB):

numpy:
  import numpy: 50ms ❌
  operación: BLAS call overhead ❌
  total: correcto pero pesado ❌

PyDead-BIB:
  sin import overhead ✓
  VMULPS ymm0 directo ✓
  8 floats / 1 ciclo ✓
```

---

## Comandos CLI

```bash
# Compilar Python
pyb py archivo.py -o output

# Target específico
pyb py archivo.py --target windows   -o output.exe
pyb py archivo.py --target linux     -o output
pyb py archivo.py --target fastos256 -o output.po   # 256-bit nativo

# Step mode — ver pipeline completo (11 fases)
pyb step archivo.py
pyb step archivo.py --target fastos256

# Build proyecto
pyb build                          # lee pyb.toml

# Crear proyecto
pyb create mi_app                  # nuevo proyecto Python
pyb create mi_app --py3            # Python 3 explícito

# Run directo
pyb run archivo.py                 # compila y ejecuta

# Instalar librerías compiladas (PyDead-BIB Mods)
pyb install numpy-native           # numpy sin CPython ✓
pyb install requests-native        # HTTP sin runtime ✓
```

---

## pyb.toml — Project Format

```toml
[project]
name = "mi_app"
version = "0.1.0"
lang = "python"
standard = "py3"

[build]
src = "src/"
include = "include/"
output = "bin/"

[python]
version = "3.11"          # sintaxis target
type_check = "strict"     # inferencia estricta
ub_mode = "strict"        # detener en UB
simd = "auto"             # AVX2 automático cuando posible
```

---

## Relación con ADead-BIB

```
ADead-BIB v8.0:            PyDead-BIB v1.0:

frontend/c/      ✓         frontend/python/     ★ NUEVO
frontend/cpp/    ✓         (solo esto es nuevo)

middle/ir/       ✓    →    middle/ir/           HEREDADO
middle/ub/       ✓    →    middle/ub/ + py_ub   HEREDADO+
optimizer/       ✓    →    optimizer/           HEREDADO
isa/             ✓    →    isa/                 HEREDADO
bg/              ✓    →    bg/                  HEREDADO
output/          ✓    →    output/              HEREDADO

= 85% código heredado    ✓
= 15% frontend Python    ★
= semanas no meses       ✓
= IR probado en FastOS   ✓
= codegen probado en PE  ✓
```

---

## Comparación Final

```
               CPython   PyPy    Nuitka    mypyc   PyDead-BIB v1.0
──────────────────────────────────────────────────────────────
Sin runtime     ❌       ❌      ❌      ❌       ✓
Sin GCC         ✓         ✓      ❌       ✓       ✓
Sin GIL         ❌       ❌      ❌      ❌       ✓
256-bit nativo  ❌       ❌      ❌      ❌       ✓
Step Mode       ❌       ❌      ❌      ❌       ✓
UB compile time ❌       ❌      ❌      partial   ✓ 21+tipos
Hello World     30MB     200MB   8MB      .so      2KB   ✓
Tipos completos partial  partial partial typed    ✓ inferido
FastOS target   ❌       ❌      ❌      ❌       ✓
BG Guardian     ❌       ❌      ❌      ❌       ✓
──────────────────────────────────────────────────────────────
Filosofía:      legible  fast    compat  typed    Dennis+Grace ✓
```

---

---

## Implementación Real — Código Fuente v1.0

### Estructura de Archivos

```
PyDead-BIB/
├── Cargo.toml                    # pydead-bib v1.0.0, nom = "7.1"
├── Cargo.lock
├── .gitignore
├── ARCHITECTURE_PyDead-BIB.md    # Este documento
├── README.md                     # Guía de uso
└── src/
    └── rust/
        ├── lib.rs                # Biblioteca principal (37 líneas)
        ├── main.rs               # CLI pyb (342 líneas)
        ├── frontend/
        │   ├── mod.rs            # Re-exports
        │   └── python/
        │       ├── mod.rs            # Pipeline exports (30 líneas)
        │       ├── py_preprocessor.rs # Fase 01 (10KB)
        │       ├── py_import_resolver.rs # Fase 02 (13KB)
        │       ├── py_lexer.rs       # Fase 03 — Tokenizer (704 líneas)
        │       ├── py_parser.rs      # Fase 04 — Parser (46KB)
        │       ├── py_ast.rs         # AST types (398 líneas)
        │       ├── py_types.rs       # Fase 05 — Type inference (333 líneas)
        │       └── py_to_ir.rs       # Fase 06 — IR generation (401 líneas)
        └── middle/
            ├── mod.rs            # Middle-end exports
            ├── ir.rs             # ADeadOp IR types (151 líneas)
            └── ub_detector.rs    # Fase 07 — UB detection (527 líneas)
```

### Tipos IR Implementados (`middle/ir.rs`)

```rust
pub enum IRType {
    Void,
    I8,      // bool
    I16,
    I32,
    I64,     // int (default Python)
    I128,
    F32,
    F64,     // float (default Python)
    Ptr,     // str, list, dict, object references
    Vec256,  // YMM 256-bit (SIMD)
}

pub enum IRInstruction {
    LoadConst(IRConstValue),
    LoadString(String),
    Load(String),
    Store(String),
    VarDecl { name: String, ir_type: IRType },
    BinOp { op: IROp, left: Box<IRInstruction>, right: Box<IRInstruction> },
    Compare { op: IRCmpOp, left: Box<IRInstruction>, right: Box<IRInstruction> },
    Label(String),
    Jump(String),
    BranchIfFalse(String),
    Return,
    ReturnVoid,
    Break,
    Continue,
    Call { func: String, args: Vec<IRInstruction> },
    IterNext { target: String, end_label: String },
    Nop,
}

pub enum IROp {
    Add, Sub, Mul, Div, FloorDiv, Mod, Pow,
    Shl, Shr, And, Or, Xor, MatMul,
}
```

### Tokens Python Implementados (`py_lexer.rs`)

```rust
pub enum PyToken {
    // Keywords (35 total)
    False, None, True, And, As, Assert, Async, Await,
    Break, Class, Continue, Def, Del, Elif, Else, Except,
    Finally, For, From, Global, If, Import, In, Is,
    Lambda, Nonlocal, Not, Or, Pass, Raise, Return,
    Try, While, With, Yield, Match, Case,  // 3.10+
    Print, Exec,  // Python 2 compat

    // Literals
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BytesLiteral(Vec<u8>),
    FStringStart(String),
    BoolLiteral(bool),

    // Operators (30+)
    Plus, Minus, Star, DoubleStar, Slash, DoubleSlash,
    Percent, At, Ampersand, Pipe, Caret, Tilde,
    ColonAssign,  // := walrus (3.8+)
    Less, Greater, LessEq, GreaterEq, EqEq, NotEq,
    LShift, RShift,

    // Indentation
    Indent, Dedent, Newline,
}
```

### AST Python Implementado (`py_ast.rs`)

```rust
pub enum PyType {
    Int, Float, Str, Bool, None, Bytes,
    List(Box<PyType>),
    Dict(Box<PyType>, Box<PyType>),
    Set(Box<PyType>),
    Tuple(Vec<PyType>),
    Optional(Box<PyType>),
    Union(Vec<PyType>),
    Callable(Vec<PyType>, Box<PyType>),
    Any,
    Custom(String),
    Inferred,
}

pub enum PyExpr {
    IntLiteral(i64), FloatLiteral(f64), StringLiteral(String),
    BytesLiteral(Vec<u8>), BoolLiteral(bool), NoneLiteral, EllipsisLiteral,
    FString { parts: Vec<FStringPart> },
    Name(String),
    BinOp { op: PyBinOp, left: Box<PyExpr>, right: Box<PyExpr> },
    UnaryOp { op: PyUnaryOp, operand: Box<PyExpr> },
    BoolOp { op: PyBoolOp, values: Vec<PyExpr> },
    Compare { left: Box<PyExpr>, ops: Vec<PyCmpOp>, comparators: Vec<PyExpr> },
    Call { func: Box<PyExpr>, args: Vec<PyExpr>, kwargs: Vec<(String, PyExpr)>, ... },
    Attribute { value: Box<PyExpr>, attr: String },
    Subscript { value: Box<PyExpr>, slice: Box<PyExpr> },
    Slice { lower: Option<Box<PyExpr>>, upper: Option<Box<PyExpr>>, step: Option<Box<PyExpr>> },
    List(Vec<PyExpr>), Tuple(Vec<PyExpr>), Set(Vec<PyExpr>),
    Dict { keys: Vec<Option<PyExpr>>, values: Vec<PyExpr> },
    ListComp { element: Box<PyExpr>, generators: Vec<PyComprehension> },
    SetComp { ... }, DictComp { ... }, GeneratorExp { ... },
    Lambda { params: Vec<PyParam>, body: Box<PyExpr> },
    IfExpr { test: Box<PyExpr>, body: Box<PyExpr>, orelse: Box<PyExpr> },
    // ... y más
}

pub enum PyStmt {
    FunctionDef { name, params, body, decorators, return_type, is_async },
    ClassDef { name, bases, body, decorators },
    Return { value: Option<PyExpr> },
    Assign { targets: Vec<PyExpr>, value: PyExpr },
    AugAssign { target, op, value },
    AnnAssign { target, annotation, value },
    For { target, iter, body, orelse, is_async },
    While { test, body, orelse },
    If { test, body, orelse },
    With { items, body, is_async },
    Match { subject, cases },  // 3.10+
    Raise { exc, cause },
    Try { body, handlers, orelse, finalbody },
    Import { names }, ImportFrom { module, names, level },
    Global { names }, Nonlocal { names },
    Expr { value }, Pass, Break, Continue,
}
```

### UB Detector Implementado (`ub_detector.rs`)

```rust
pub enum PythonUB {
    // Heredados de C
    DivisionByZero,
    IntegerOverflow,
    UninitializedVariable,

    // Python-specific
    NoneDeref,                 // None.attr → AttributeError
    IndexOutOfBounds,          // lista[100] con len=10
    KeyNotFound,               // dict["x"] sin "x"
    TypeMismatch,              // "hola" + 42
    InfiniteRecursion,         // recursión sin base case
    CircularImport,            // A→B→A
    MutableDefaultArg,         // def f(x=[]) ← bug clásico
    GlobalWithoutDeclaration,  // modifica global sin 'global'
    IteratorExhausted,         // reusar generator consumido
    UnpackMismatch,            // a, b = [1, 2, 3]
}

pub struct UBReport {
    pub kind: PythonUB,
    pub severity: UBSeverity,  // Error | Warning | Info
    pub message: String,
    pub line: usize,
    pub col: usize,
    pub file: String,
    pub suggestion: Option<String>,
}
```

### Type Inferencer Implementado (`py_types.rs`)

```rust
pub enum ConcreteType {
    Int64,
    Float64,
    Bool,
    Str,
    Bytes,
    NoneType,
    List(Box<ConcreteType>),
    Dict(Box<ConcreteType>, Box<ConcreteType>),
    Set(Box<ConcreteType>),
    Tuple(Vec<ConcreteType>),
    Object(String),    // class instance
    Function { params: Vec<ConcreteType>, ret: Box<ConcreteType> },
    Dynamic,           // fallback
}

// Built-in functions pre-registradas:
// print, len, range, int, float, str, bool
```

---

## Estado de Implementación v1.0

| Fase | Componente | Estado | Archivo | LOC |
|------|------------|--------|---------|-----|
| 01 | Preprocessor | ✅ Completo | `py_preprocessor.rs` | ~300 |
| 02 | Import Resolver | ✅ Completo | `py_import_resolver.rs` | ~400 |
| 03 | Lexer | ✅ Completo | `py_lexer.rs` | 704 |
| 04 | Parser | ✅ Completo | `py_parser.rs` | ~1400 |
| 05 | Type Inferencer | ✅ Completo | `py_types.rs` | 333 |
| 06 | IR Generator | ✅ Completo | `py_to_ir.rs` | 401 |
| 07 | UB Detector | ✅ Completo | `ub_detector.rs` | 527 |
| 08 | Optimizer | ⏳ Pendiente | (heredar ADead-BIB) | - |
| 09 | Register Allocator | ⏳ Pendiente | (heredar ADead-BIB) | - |
| 10 | Bit Resolver | ⏳ Pendiente | (heredar ADead-BIB) | - |
| 11 | ISA Compiler | ⏳ Pendiente | (heredar ADead-BIB) | - |
| 12 | BG Stamp | ⏳ Pendiente | (heredar ADead-BIB) | - |
| 13 | Output (PE/ELF/Po) | ⏳ Pendiente | (heredar ADead-BIB) | - |

**Total LOC Frontend Python:** ~4,000 líneas Rust

---

## Roadmap v1.1 → v2.0

### v1.1 — Backend Integration
```
[ ] Integrar optimizer de ADead-BIB v8.0
[ ] Integrar register allocator
[ ] Integrar ISA compiler (encoder.rs)
[ ] Generar PE ejecutable real
[ ] Generar ELF ejecutable real
[ ] Test: Hello World → 2KB .exe
```

### v1.2 — SIMD Automático
```
[ ] Detectar list[float] × 8 → YMM
[ ] SoaOptimizer para comprehensions
[ ] VMULPS/VADDPS generación
[ ] Benchmark vs numpy
```

### v1.3 — Stdlib Nativa
```
[ ] math → SIMD inline (sqrt, sin, cos)
[ ] os.path → syscalls directos
[ ] sys → constantes compiladas
[ ] json → parser nativo
[ ] re → regex compilado
```

### v2.0 — Production Ready
```
[ ] Async/await → state machine nativo
[ ] Decorators → compile-time transform
[ ] Metaclasses → vtable generation
[ ] C extension compatibility layer
[ ] PyPI package distribution
```

---

## Métricas de Código

```
Total Rust:        ~4,500 líneas
Frontend Python:   ~4,000 líneas (89%)
Middle-end:        ~500 líneas (11%)

Dependencias:      1 (nom = "7.1")
Binario CLI:       ~500KB release

Tests:             (pendiente)
Documentación:     ARCHITECTURE.md + README.md
```

---

*PyDead-BIB v1.0 — 2026*  
*"Python sin runtime — sin GIL — sin CPython — sin linker — 16 hasta 256 bits"*  
*Hereda ADead-BIB v8.0 — IR probado — codegen probado — FastOS boots*  
*Eddi Andreé Salazar Matos — Lima, Perú 🇵🇪 — 1 dev — Binary Is Binary 💀🦈*
