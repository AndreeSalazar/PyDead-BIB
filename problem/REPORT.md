# PyDead-BIB — Test Failure Report
**Fecha:** 2026-03-18  
**Compilador:** PyDead-BIB v4.0.0  
**Target:** Windows x86-64  
**Total tests:** 55 | **Pass:** 50 | **Fail:** 5

---

## Resumen de Resultados

| # | Test | Estado | Error | Área afectada |
|---|------|--------|-------|---------------|
| 1 | `test_comprehensions.py` | ❌ SEGFAULT | exit -1073741819 (0xC0000005) | **List indexing + while loop** |
| 2 | `test_dict_v45.py` | ❌ SEGFAULT | exit -1073741819 (0xC0000005) | **Dict mutation + indexing** |
| 3 | `test_list_iter_v46.py` | ❌ SEGFAULT | exit -1073741819 (0xC0000005) | **for-in iteration sobre listas** |
| 4 | `test_simd.py` | ❌ SEGFAULT | exit -1073741819 (0xC0000005) | **List indexing (8 elementos) + while loop** |
| 5 | `test_str_v45.py` | ❌ TIMEOUT | Infinite loop / hang | **String methods (.upper/.lower/.strip/.find/.startswith/.endswith)** |

---

## Análisis por Test

### 1. `test_comprehensions.py` — SEGFAULT
```python
a = [1, 2, 3, 4, 5]
print(a[0])    # list index access
print(a[4])    # list index access
print(len(a))  # len() on list
x = 0
i = 0
while i < 5:
    x = x + a[i]  # ← probable causa: a[i] con variable index
    i = i + 1
print(x)
```
**Diagnóstico:** El codegen genera access violation al indexar una lista con una variable (`a[i]`). El acceso con literal (`a[0]`, `a[4]`) posiblemente también falla, o el crash es en el acceso dinámico dentro del while loop. Revisar `isa.rs` → `ListGet` con índice variable.

### 2. `test_dict_v45.py` — SEGFAULT
```python
d = {1: 10, 2: 20}
d[3] = 30          # dict set
print(len(d))      # len() on dict
print(d[1])        # dict get
print(d[3])        # dict get
```
**Diagnóstico:** Dict con keys int funciona en `test_dict.py` (que pasa), pero `test_dict_v45.py` falla. La diferencia probable es `d[3] = 30` (mutación de dict post-creación) o `len(d)` sobre dict mutado. Revisar codegen de `DictSet` dinámico.

### 3. `test_list_iter_v46.py` — SEGFAULT
```python
for x in my_list:    # ← for-in sobre lista
    print(x)
```
**Diagnóstico:** `for x in list` genera `IterCreate` + `IterNext` que crashea en runtime. El iterador sobre listas no está correctamente implementado en el backend. Revisar `isa.rs` → `IterCreate`/`IterNext` codegen.

### 4. `test_simd.py` — SEGFAULT
```python
a = [1, 2, 3, 4, 5, 6, 7, 8]  # 8 elementos
print(a[0])
print(a[7])
total = 0
i = 0
while i < 8:
    total = total + a[i]       # ← mismo patrón que test_comprehensions
    i = i + 1
```
**Diagnóstico:** Mismo problema que `test_comprehensions.py`. Listas más grandes (8 elementos) + acceso por índice variable. Puede ser un issue de heap allocation o de codegen para `ListGet` con registro como índice.

### 5. `test_str_v45.py` — TIMEOUT (infinite loop)
```python
s = "  Hello, World!  "
print(s.upper())        # ← string method
print(s.lower())
print(s.strip())
print(s.find("World"))
print(s.startswith("  Hello"))
print(s.endswith("World!  "))
```
**Diagnóstico:** Los métodos de string (`upper()`, `lower()`, etc.) generan un loop infinito en el código máquina. El `test_strings.py` pasa (usa operaciones más simples). Revisar codegen de `StringMethod` en `isa.rs`, especialmente los loops internos que procesan caracteres — probable que la condición de terminación del loop no funcione.

---

## Áreas del Compilador a Investigar

| Archivo | Funcionalidad | Prioridad |
|---------|---------------|-----------|
| `src/rust/backend/isa.rs` | `ListGet` con índice variable (registro) | 🔴 ALTA |
| `src/rust/backend/isa.rs` | `IterCreate` / `IterNext` para listas | 🔴 ALTA |
| `src/rust/backend/isa.rs` | `DictSet` dinámico (post-creación) | 🔴 ALTA |
| `src/rust/backend/isa.rs` | `StringMethod` (upper/lower/strip/find/startswith/endswith) | 🔴 ALTA |
| `src/rust/frontend/python/py_to_ir.rs` | IR generation para list comprehension patterns | 🟡 MEDIA |

---

## Patrón Común
Los 4 SEGFAULTs comparten el código de error `0xC0000005` (Access Violation en Windows), lo que indica que el código máquina generado intenta leer/escribir memoria inválida. Esto típicamente ocurre cuando:
1. Un puntero a heap no se resuelve correctamente
2. Un offset de acceso a array/dict se calcula mal
3. El iterador no obtiene la dirección correcta de los datos

El TIMEOUT indica un loop sin condición de salida correcta en el machine code emitido.
