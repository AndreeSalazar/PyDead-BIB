# 📖 Guías de PyDead-BIB 💀🦈
> Python → x86-64 Nativo — Sin CPython — Sin GIL — Sin runtime

---

## 🚀 Modos de Ejecución

### 1. JIT 2.0 — Default (recomendado)
```powershell
cargo run -- <archivo.py>
# o si tienes el binario instalado:
pyb <archivo.py>
```
**Qué hace:** Compila y ejecuta **en memoria** (RAM). No escribe ningún `.exe` al disco.  
**Velocidad:** ~5ms arranque. Sin filesystem. Puro RAM.

**Ejemplo:**
```powershell
cargo run -- tests/test_str_v45.py
```
Output esperado:
```
  HELLO, WORLD!
  hello, world!
  18
  True
  True
  Hello, World!
  7
  Hello, Peru!
  42
```

---

### 2. JIT 2.0 con Stats — `pyb run`
```powershell
cargo run -- run <archivo.py>
```
**Qué hace:** igual que el default, pero muestra estadísticas del JIT:  
tiempo de alloc, tiempo de patch, tiempo de ejecución, cache HIT/COLD, tamaño en RAM.

**Ejemplo:**
```powershell
cargo run -- run tests/test_list_v45.py
```

---

### 3. Compilar a `.exe` nativo — `pyb py`
```powershell
cargo run -- py <archivo.py>
cargo run -- py <archivo.py> -o output.exe
```
**Qué hace:** Genera un `.exe` standalone nativo para Windows x86-64.

**Ejemplo:**
```powershell
cargo run -- py tests/test_hello.py
# genera: test_hello.exe (6.5KB)
.\test_hello.exe
```

---

### 4. Step — Análisis de fases
```powershell
cargo run -- step <archivo.py>
```
**Qué hace:** Muestra las 13 fases de compilación en detalle (lexer, parser, IR, ISA, etc.)

**Ejemplo:**
```powershell
cargo run -- step tests/test_str_v45.py
```
Output esperado:
```
▸ Phase 01: PREPROCESSOR   [0.184ms]
▸ Phase 02: IMPORT ELIMINATOR
▸ Phase 03: LEXER  tokens: 51
▸ Phase 04: PARSER  AST: 6 top-level nodes
...
▸ Phase 10: ISA COMPILER  .text: 4422 bytes
...
✅ Compilation complete — 13/13 phases
```

---

### 5. Suite de tests — `pyb test`
```powershell
cargo run -- test
```
Output esperado:
```
✅ TOTAL: 89/89 PASS
Binary Is Binary 💀🦈🇵🇪
```

---

## 📝 Ejemplos de Código Soportado (v4.5)

### Strings
```python
# Concatenación
s = "Hola" + ", " + "Mundo!"
print(s)              # Hola, Mundo!

# Métodos
print(s.upper())      # HOLA, MUNDO!
print(s.lower())      # hola, mundo!
print(s.strip())      # Hola, Mundo!  ← sin espacios
print(s.find("Mundo"))   # 6
print(s.startswith("Hola"))  # 1 (True)
print(s.endswith("!"))       # 1 (True)
print(s.replace("Mundo", "Peru"))  # Hola, Peru!

# Conversión
n = 2024
print(str(n))         # 2024

# Comparación de contenido
print(len(s))         # 12
```

### Listas
```python
l = [5, 3, 1, 4, 2]
l.sort()              # ordena in-place
l.reverse()           # invierte in-place
v = l.pop()           # quita y retorna último
l.append(99)          # agrega al final
print(len(l))         # longitud
```

### Diccionarios (claves enteras)
```python
d = {1: 100, 2: 200}
d[3] = 300            # asignar valor
print(d[1])           # leer valor: 100
print(len(d))         # tamaño: 3
```

### Operadores
```python
# `in` para listas y strings
nums = [1, 2, 3, 4]
print(3 in nums)        # 1
print(9 not in nums)    # 1

s = "PyDead-BIB"
print("Dead" in s)      # 1

# str * N
sep = "-" * 20
print(sep)              # --------------------
```

### Funciones y clases
```python
def sumar(a: int, b: int) -> int:
    return a + b

class Punto:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

p = Punto(10, 20)
print(sumar(p.x, p.y))   # 30
```

### F-strings
```python
nombre = "Peru"
version = 45
print(f"PyDead-BIB v{version} — {nombre} 💀🦈")
```

### Módulos stdlib
```python
import math
print(math.sqrt(144))    # 12.0
print(math.pi)           # 3.141592...

import os
print(os.getcwd())

import sys
print(sys.platform)      # win32
```

---

## 🔧 Comandos Rápidos

| Comando | Acción |
|---------|--------|
| `cargo run -- <file.py>` | JIT 2.0 (default) |
| `cargo run -- run <file.py>` | JIT 2.0 + stats |
| `cargo run -- py <file.py>` | Compilar a .exe |
| `cargo run -- step <file.py>` | Ver 13 fases |
| `cargo run -- test` | Suite 89 tests |
| `cargo run -- version` | Ver versión |

---

## ⚡ Tiempos de referencia (i7 Intel, Windows)

| Fase | Tiempo |
|------|--------|
| Lexer + Parser | ~0.4ms |
| IR Gen | ~0.4ms |
| ISA Compile | ~0.9ms |
| JIT alloc+exec | ~1.0ms |
| **Total** | **~5ms** |

> 💀 Sin CPython — Sin GIL — Sin runtime — Binary Is Binary 🦈
