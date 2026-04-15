# PyDead-BIB: Plan de Reconstrucción JIT 2.0 💀🦈

> **Python → Machine Code Nativo (En RAM) — JIT 2.0 Orchestrator**
> Rust actúa exclusivamente como orquestador, tomando el código Python, convirtiéndolo a Machine Code y despachándolo a memoria vía `VirtualAlloc`, sin intérprete, sin GIL y sin CPython.

---

## 📊 Estado Actual & Arquitectura JIT 2.0

> **"El CPU no piensa — ya sabe. La RAM no espera — ya recibe."**

Anteriormente se buscó generar ejecutables `.exe` (AOIT/AOT), pero el motor principal y filosofía nativa de PyDead-BIB ha pivotado completamente a **JIT 2.0** (`pyd run`). En esta variante, el compilador transpila y carga directamente el pipeline hacia memoria ejecutando llamadas de sistema súper veloces al unísono con el código de máquina subyacente.

Rust se dedica a analizar la sintaxis de base (ver `Python_base` para la rúbrica de pruebas desde nivel básico a avanzado), construir el árbol, la inferencia de tipos estricta y delegar el byte/machine array resultante directo a la memoria.

### Pipeline JIT 2.0 (13 Fases)

1.  **Preprocessor & Import Eliminator**: Carga unificada y limpieza UTF-8.
2.  **Lexer & Parser**: Identificación léxica de todos los ast-nodes desde `01_hello` hasta `05_complete` en base a CPython estricto.
3.  **Type Inferencer**: Inferencia de tipos en tiempo de compilación. No hay validación dinámica excesiva, es estático por tipado (Int, Float, Str).
4.  **IR Generation (ADeadOp SSA)**: Conversión a operaciones intermedias en Single Static Assignment, 100% Rust-backed.
5.  **UB Detector**: Captura todos los *Undefined Behaviors* (accesos a punteros libres, out-of-bounds, etc.) en tiempo de compilación (AOT) antes del JIT.
6.  **Optimizer (v1 & v2)**: Eliminación de código muerto y doblamiento de constantes.
7.  **Register Allocator (RA)**: Asignación de registros nativos (RAX, RBX, RCX...) mediante coloring graphs o linear scan.
8.  **ISA x86-64 / SIMD (AVX2)**: Backend directo de generación de bytes x86-64 y soporte AVX2 si el CPU (`cpuid`) lo soporta.
9.  **JIT KILLER Execution (`VirtualAlloc`)**: Carga en página RWX, resolución de Imports/IAT instantánea al vuelo y `jmp` o call system pointer resultando en ejecución sub-milisegundo.

---

## 🎯 `Python_base` — Rúbrica de Pruebas "De Básico a Avanzado"

Para garantizar la fiabilidad del Orquestador Rust, se construye y valida frente la suite pura en CPython llamada `Python_base`. Todo lo que esté en estos archivos y funciones, debe ser soportado y tener comportamiento idéntico (pero millones de veces más rápido en RAM) en PyDead-BIB.

### Estructura de Test Suites

| Nivel              | Objetivo de la Sintaxis                              | Estado JIT 2.0   |
| ------------------ | ---------------------------------------------------- | ---------------- |
| `01_hello`         | Print strings, prints vacíos, sintaxis ultra base.   | En Progreso / OK |
| `02_basic`         | Literals (int/float/str/bool), operadores math.      | En Progreso      |
| `03_intermediate`  | Control Flow (if/for/while), funciones y closures.   | En Progreso      |
| `04_advanced`      | OOP simple, excepciones, raise, imports clásicos.    | En Progreso      |
| `05_complete`      | Programas end-to-end complejos y optimizaciones (SIMD) | Construcción     |

---

## 🚀 Metas de esta reconstrucción

1.  Soportar **El 100% de la Suite de Pruebas `Python_base`** nativamente.
2.  Mantener estricto control de memoria: sin GIL, determinista, y Memory-safe a nivel compilación gracias a `ub_detector.rs`.
3.  Optimizar SIMD vectorizando listas en CPython de forma transparente usando vectores YMM y xmm.

*Lima, Perú 🇵🇪 — Binary Is Binary 💀🦈*
