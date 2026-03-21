# ADead-BIB Extension para VS Code

Soporte completo para el lenguaje **ADead-BIB** - OOP Puro + ASM Simbionte.

## Características

- ✅ **Syntax Highlighting** para archivos `.adB`
- ✅ **Snippets** para código común
- ✅ **Diagnósticos en tiempo real** (errores y warnings)
- ✅ **Comandos integrados** (Build, Run, Check, Optimize)
- ✅ **Atajos de teclado**

## Instalación

### Desde VSIX (local)

```bash
cd EXTENSION/adead-bib-vscode
npm install
npm run compile
vsce package
code --install-extension adead-bib-1.0.0.vsix
```

### Requisitos

- VS Code 1.74+
- ADead-BIB compilador (`adB` o `adeadc`) en el PATH

## Uso

### Comandos

| Comando | Atajo | Descripción |
|---------|-------|-------------|
| `ADead-BIB: Build` | `Ctrl+Shift+B` | Compilar archivo actual |
| `ADead-BIB: Run` | `F5` | Compilar y ejecutar |
| `ADead-BIB: Check Syntax` | `Ctrl+Shift+C` | Verificar sintaxis |
| `ADead-BIB: Build Optimized` | - | Compilación ultra-optimizada |

### Snippets

| Prefijo | Descripción |
|---------|-------------|
| `fn main` | Función principal |
| `fn` | Definir función |
| `let` | Variable |
| `const` | Constante |
| `println` | Imprimir con salto |
| `if` | Estructura if |
| `while` | Bucle while |
| `for` | Bucle for |
| `struct` | Estructura |
| `trait` | Trait |
| `impl` | Implementación |
| `cpu` | Bloque CPU |
| `gpu` | Bloque GPU |
| `emit` | Emitir HEX |

## Configuración

```json
{
  "adead-bib.compilerPath": "adB",
  "adead-bib.showDiagnostics": true
}
```

## Arquitectura

```
Editor (JS) ──▶ adB check --json ──▶ Diagnósticos
```

La extensión **no analiza el código**, solo **le pregunta al compilador**.

- **Rust (adeadc)** = 🧠 Cerebro (parser, análisis, validación)
- **JS (Extension)** = 👁️ Cara (UI, colores, presentación)

## Desarrollo

```bash
# Instalar dependencias
npm install

# Compilar
npm run compile

# Watch mode
npm run watch

# Empaquetar
vsce package
```

## Licencia

GPLv2 - Ver [LICENSE](../../LICENSE)

---

**ADead-BIB: Rust es el cerebro, JS es la cara.**
