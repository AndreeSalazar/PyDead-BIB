# 📊 Reporte de Gaps — PyDead-BIB v4.5
> Estado actual vs Python estándar. Prioridades para v4.6+

---

## ✅ Implementado (v4.5)

| Feature | Estado |
|---------|--------|
| int, float, bool literals | ✅ |
| str: upper, lower, find, replace, startswith, endswith, strip, split | ✅ |
| str + str concat | ✅ |
| str * N repeat | ✅ |
| str() int→str | ✅ |
| list: sort, reverse, pop, append, len | ✅ |
| dict: get, len, set, keys/values/items | ✅ |
| `x in list/str/dict` | ✅ |
| `x not in` | ✅ |
| f-strings | ✅ |
| for/while loops | ✅ |
| if/elif/else | ✅ |
| def, return | ✅ |
| class + __init__ + self | ✅ |
| inheritance | ✅ |
| import math/os/sys/random/json | ✅ |
| try/except | ✅ |
| with/context manager (básico) | ✅ |
| async/await (básico) | ✅ |
| generators (básico) | ✅ |
| JIT 2.0 in-memory | ✅ |
| x86-64 nativo Windows | ✅ |

---

## 🔴 CRÍTICO — Faltan para uso real

### 1. Dict con claves tipo `str`
**Estado:** Clave `"abc"` en dict crashea en runtime  
**Causa:** El hasher de `__pyb_dict_set`/`__pyb_dict_get` usa el puntero de la cadena, no su contenido  
**Fix necesario:** Implementar `__pyb_str_hash` (djb2 o FNV-1a) + comparar por valor en `dict_get`  
**Impacto:** Alto — casi todos los dicts reales usan str keys  
```python
# FALLA ahora:
d = {"nombre": "Andre", "ciudad": "Lima"}
print(d["nombre"])
```

### 2. for x in list (iteración real)
**Estado:** `for i in range(n)` funciona. `for x in my_list` no genera IR correcto  
**Causa:** El IR generator no emite `__pyb_list_get` en el loop body para listas heap  
**Fix necesario:** Detectar `for x in list_var` y generar: `i=0; while i < len: x = list[i]; i++`  
**Impacto:** Alto — patrón Python más común  
```python
# FALLA ahora:
nums = [10, 20, 30]
for n in nums:
    print(n)
```

### 3. print(list) — formato `[a, b, c]`
**Estado:** `print(my_list)` imprime como int (el puntero)  
**Causa:** El print handler no detecta variables lista para llamar `__pyb_list_print`  
**Fix necesario:** Marcar list vars en `list_vars` set y detectar en print handler  
**Impacto:** Alto — debugging muy difícil sin esto  

### 4. Exceptions con mensaje real
**Estado:** `raise ValueError("msg")` funciona pero `except ValueError as e: print(e)` no captura msg  
**Causa:** El runtime de excepciones no pasa el mensaje al handler  
**Impacto:** Medio-alto

### 5. str.format() y % formatting
**Estado:** f-strings funcionan. `"hola {}".format(x)` no  
**Causa:** No hay stub para `__pyb_str_format`  
**Impacto:** Medio

---

## 🟡 IMPORTANTE — Para completar tipos

### 6. str.split() retorna lista real
**Estado:** `split()` llama a `str_find` (retorna índice, no lista)  
**Fix:** Implementar `__pyb_str_split` que retorna una `PyList` de heap strings  
```python
# INCORRECTO ahora:
partes = "a,b,c".split(",")  # retorna int, no lista
```

### 7. str.join()
**Estado:** No implementado  
**Fix:** `__pyb_str_join(sep, list_ptr)` → heap string con separadores  
```python
resultado = ",".join(["a", "b", "c"])  # "a,b,c"
```

### 8. list comprehensions con expresión real
**Estado:** `[x for x in range(n)]` funciona básico  
**No funciona:** `[x*2 for x in lista if x > 0]`  
**Fix:** Completar IR gen para comprehensions con condición y transformación  

### 9. AugAssign `s += "texto"` para strings
**Estado:** `+=` para ints funciona. Para strings no ruta a `str_concat`  
**Fix:** Detectar string vars en AugAssign Add y emitir `__pyb_str_concat`  
```python
s = "Hola"
s += ", Mundo"  # INCORRECTO: hace Add numérico
```

### 10. Slicing `s[1:5]`, `l[2:]`
**Estado:** `__pyb_str_slice` existe pero `s[1:5]` en AST no se rutea a él  
**Fix:** Detectar `PyExpr::Subscript` con slice range y emitir `str_slice` o `list_slice`  
```python
sub = s[0:5]   # FALLA
sub = s[-3:]   # FALLA
```

### 11. `len()` sobre strings
**Estado:** `len(str_var)` llama a `__builtin_len` (pensando que es lista)  
**Fix:** Detectar string vars en `len()` y llamar `__pyb_str_len` en su lugar  
```python
print(len("hola"))  # INCORRECTO: usa list_len que lee el header de lista
```

---

## 🟢 MEJORAS — Para mayor compatibilidad

### 12. Múltiples valores de retorno con tuple
**Estado:** Una función puede retornar un tuple pero la asignación `a, b = func()` no desempaqueta heap tuples  

### 13. Default args en funciones
**Estado:** `def f(x, y=10)` no compila — el parser lo acepta pero el IR falla  

### 14. *args y **kwargs
**Estado:** No implementado  

### 15. lambda
**Estado:** No implementado  
**Fix:** Convertir lambda a función anónima en IR  

### 16. Closures reales
**Estado:** Funciones anidadas no capturan scope exterior  

### 17. `__str__`, `__repr__`, `__len__` en clases
**Estado:** No implementado — métodos dunder no se despachan  

### 18. `type()`, `isinstance()`
**Estado:** `isinstance()` compilado pero siempre retorna False  

### 19. `int()`, `float()`, `bool()` builtins
**Estado:** `str()` implementado. Los otros no — necesitan stubs  
```python
x = int("42")    # FALLA
y = float("3.14")  # FALLA
```

### 20. Módulo `re` (regex básico)
**Estado:** No implementado  

---

## 📈 Roadmap Sugerido

### v4.6 (próximo sprint)
1. Dict string keys (`__pyb_str_hash` djb2)
2. `for x in list` iteración real
3. `print(list)` detección automática
4. `s += "str"` AugAssign
5. `len(str)` correcto

### v4.7
6. `str.split()` retorna lista real
7. `str.join()` implementado
8. Slicing `s[a:b]`, `l[a:b]`
9. `int()`, `float()`, `bool()` builtins

### v4.8
10. Default args
11. `__str__` dunder dispatch
12. List comprehensions completas
13. lambda básico

### v5.0 (Production Ready)
- Closures reales
- *args / **kwargs
- `re` módulo básico
- GC incremental opcional (para objetos de larga vida)
- Linux x86-64 target completo
- Cross-compile ARM64

---

> 💀 Binary Is Binary — La meta: Python real sin CPython 🦈🇵🇪
