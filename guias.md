# 📖 Guías de PyDead-BIB 💀🦈

> Python → x86-64 Nativo — Sin CPython — Sin GIL — Sin runtime
> **CLI:** `pyd` (PyDead-BIB Compiler v5.0)

---

## � Instalación

### Windows (PowerShell como Admin)

```powershell
# 1. Compilar el proyecto
cargo build --release

# 2. Crear directorio y copiar ejecutable
mkdir $env:USERPROFILE\.pyd -Force
Copy-Item target\release\pyd.exe $env:USERPROFILE\.pyd\pyd.exe

# 3. Agregar al PATH (permanente)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";$env:USERPROFILE\.pyd", "User")

# 4. Reiniciar terminal y verificar
pyd --version
```

### Linux/macOS

```bash
# 1. Compilar
cargo build --release

# 2. Instalar
mkdir -p ~/.local/bin
cp target/release/pyd ~/.local/bin/

# 3. Agregar al PATH (en ~/.bashrc o ~/.zshrc)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# 4. Verificar
pyd --version
```

---

## 🚀 Comandos Principales

### 1. Ejecutar Python — `pyd run` (recomendado)

```bash
pyd run archivo.py
```

**Qué hace:** Compila y ejecuta en memoria (JIT 2.0). No escribe `.exe` al disco.

**Ejemplo:**

```bash
pyd run tests/jit_runner/test_jit_strings.py
```

**Output:**

```
╔════════════════════════════════════════════════════════════════╗
║   PyDead-BIB JIT KILLER v2.0 💀🦈                             ║
╚════════════════════════════════════════════════════════════════╝
  ⚡ compile: 1.125ms
  ⚡ time-to-RAM: 1.464ms
  ✓ JIT complete (exit: 0)
```

---

### 2. Debugging — `pyd step`

```bash
pyd step archivo.py
```

**Qué hace:** Muestra cada fase del compilador con información detallada para debugging.

**Fases mostradas:**

| Fase | Descripción |
|------|-------------|
| 01 | SOURCE CODE — código fuente con números de línea |
| 02 | PREPROCESSOR — preprocesamiento |
| 03 | LEXER — todos los tokens generados |
| 04 | PARSER — AST completo |
| 05 | TYPE INFERENCER — tipos + class layouts |
| 06 | IR GENERATION — instrucciones IR de cada función |
| 07 | UB DETECTOR — errores de undefined behavior |
| 08-09 | OPTIMIZER + REGALLOC — registros asignados |
| 10 | ISA COMPILER — bytes generados |

**Ejemplo:**

```bash
pyd step tests/jit_runner/test_jit_classes.py
```

---

### 3. Compilar a Ejecutable — `pyd py`

```bash
pyd py archivo.py              # genera archivo.exe
pyd py archivo.py -o salida.exe  # nombre personalizado
```

**Qué hace:** Genera ejecutable nativo standalone (.exe Windows, ELF Linux).

**Ejemplo:**

```bash
pyd py hello.py -o hello.exe
./hello.exe
```

---

### 4. Ejecutar Directo

```bash
pyd archivo.py
```

**Qué hace:** Alias de `pyd run`. Ejecuta directamente con JIT 2.0.

---

### 5. Tests

```bash
pyd test
```

**Qué hace:** Ejecuta la suite completa de tests.

---

## 📋 Referencia Rápida

| Comando | Descripción |
|---------|-------------|
| `pyd run <file.py>` | Ejecutar con JIT 2.0 + stats |
| `pyd step <file.py>` | Debugging paso a paso |
| `pyd py <file.py>` | Compilar a .exe/.elf |
| `pyd <file.py>` | Ejecutar directo (alias de run) |
| `pyd test` | Ejecutar suite de tests |
| `pyd --version` | Ver versión |
| `pyd --help` | Mostrar ayuda |

---

## 📝 Ejemplos de Código Soportado (v5.0)

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

## ⚡ Tiempos de Referencia

| Fase | Tiempo |
|------|--------|
| Preprocess + Lex | ~0.03ms |
| Parse | ~0.05ms |
| Type Inference | ~0.05ms |
| IR Gen | ~0.07ms |
| Optimize + RegAlloc | ~0.02ms |
| ISA Compile | ~0.16ms |
| JIT alloc+exec | ~0.25ms |
| **Total** | **~1.0ms** |

---

## 🎯 Type Strictness ULTRA

PyDead-BIB usa tipado estricto al estilo FORTRAN:

```python
# ✅ Permitido
x = 5 + 3           # int + int
y = 3.14 + 2.71     # float + float
s = "Hola" + "!"    # str + str

# 💀 BLOQUEADO (error de compilación)
z = 5 + 3.14        # int + float → ERROR
w = "Hola" + 42     # str + int → ERROR

# ✅ Conversión explícita
z = float(5) + 3.14  # OK
w = "Hola" + str(42) # OK
```

---

Aquí está el comando para ejecutar el test de strings:
```bash
#.\target\release\pyd.exe run tests\jit_runner\test_jit_strings.py
```


> 💀 Sin CPython — Sin GIL — Sin runtime — Binary Is Binary 🦈🇵🇪
