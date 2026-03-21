# ADead-BIB Extension — Arquitectura JS + Rust CLI

> **Filosofía**: Rust es el cerebro, el editor solo es la cara.

---

## 1. ¿Por qué JS + Rust CLI?

| Enfoque       | Resultado |
|---------------|-----------|
| Todo JS       | frágil    |
| Todo Rust     | imposible |
| **JS + Rust CLI** | **ideal** |

Las extensiones de VS Code se escriben en JS/TS, pero el **análisis real** lo hace Rust.

```
Editor (JS) ──▶ CLI adeadc (Rust) ──▶ Resultado
```

---

## 2. Arquitectura Recomendada

### 🟦 VS Code Extension (JS/TS)
- Botones y UI
- Syntax highlighting
- Ejecución de comandos
- Presentación de errores/warnings

### 🟧 adeadc (Rust)
- Parsea `.adB`
- Detecta errores
- Detecta `emit![]`, `cpu::`, `gpu::`
- Devuelve JSON

---

## 3. Comando: `adB check --json`

```bash
adB check archivo.adB --json
```

### Salida JSON:

```json
{
  "file": "main.adB",
  "status": "ok",
  "errors": [],
  "warnings": [
    {
      "line": 42,
      "column": 5,
      "type": "raw_binary",
      "severity": "warning",
      "message": "emit![] usado - código binario directo"
    },
    {
      "line": 15,
      "column": 1,
      "type": "cpu_block",
      "severity": "info",
      "message": "Bloque cpu:: detectado"
    }
  ],
  "diagnostics": {
    "functions": 3,
    "variables": 12,
    "cpu_blocks": 1,
    "gpu_blocks": 0,
    "emit_calls": 2
  }
}
```

---

## 4. Qué hace cada parte

### Rust (adeadc) hace lo difícil:
- ✅ Parser completo
- ✅ Análisis sintáctico
- ✅ Validaciones
- ✅ Warnings inteligentes
- ✅ Clasificación de zonas peligrosas
- ✅ Generación de metadata

### JS (Extension) solo presenta:
- ✅ Syntax highlighting básico (JSON)
- ✅ Autoclose de brackets
- ✅ Comentarios
- ✅ Identificación de `.adB`
- ✅ Mostrar errores en el editor
- ✅ Botones de compilar/ejecutar

---

## 5. Ventajas de esta arquitectura

| Ventaja | Descripción |
|---------|-------------|
| **Una sola fuente de verdad** | El compilador ES la autoridad |
| **Cero duplicación** | No hay lógica repetida en JS |
| **Coherencia** | Compilador y editor siempre de acuerdo |
| **Menos bugs** | Un solo lugar donde arreglar |
| **Fácil de mantener** | Rust hace lo difícil, JS solo pinta |

---

## 6. Estructura de la Extensión

```
adead-bib-vscode/
├── package.json           # Configuración de la extensión
├── syntaxes/
│   └── adead-bib.tmLanguage.json  # Syntax highlighting
├── language-configuration.json    # Brackets, comentarios
├── src/
│   ├── extension.ts       # Punto de entrada
│   ├── diagnostics.ts     # Llama a adB check --json
│   ├── commands.ts        # Comandos (build, run, check)
│   └── utils.ts           # Utilidades
└── README.md
```

---

## 7. Ejemplo de Integración

### extension.ts (simplificado)

```typescript
import * as vscode from 'vscode';
import { exec } from 'child_process';

export function activate(context: vscode.ExtensionContext) {
    // Comando: Verificar sintaxis
    let checkCmd = vscode.commands.registerCommand('adead-bib.check', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;
        
        const file = editor.document.fileName;
        
        exec(`adB check "${file}" --json`, (err, stdout) => {
            if (err) {
                vscode.window.showErrorMessage('Error al verificar');
                return;
            }
            
            const result = JSON.parse(stdout);
            showDiagnostics(result);
        });
    });
    
    context.subscriptions.push(checkCmd);
}

function showDiagnostics(result: any) {
    const diagnostics: vscode.Diagnostic[] = [];
    
    for (const warning of result.warnings) {
        const range = new vscode.Range(
            warning.line - 1, warning.column - 1,
            warning.line - 1, 100
        );
        
        const diag = new vscode.Diagnostic(
            range,
            warning.message,
            vscode.DiagnosticSeverity.Warning
        );
        
        diagnostics.push(diag);
    }
    
    // Mostrar en el editor...
}
```

---

## 8. Syntax Highlighting (JSON)

### adead-bib.tmLanguage.json

```json
{
  "name": "ADead-BIB",
  "scopeName": "source.adB",
  "fileTypes": ["adB"],
  "patterns": [
    {
      "name": "keyword.control.adB",
      "match": "\\b(fn|let|const|if|else|while|for|return|struct|trait|impl)\\b"
    },
    {
      "name": "keyword.other.adB",
      "match": "\\b(cpu|gpu|emit|print|println)\\b"
    },
    {
      "name": "string.quoted.double.adB",
      "begin": "\"",
      "end": "\""
    },
    {
      "name": "comment.line.adB",
      "match": "//.*$"
    },
    {
      "name": "constant.numeric.hex.adB",
      "match": "0x[0-9A-Fa-f_]+"
    },
    {
      "name": "constant.numeric.adB",
      "match": "\\b[0-9]+\\b"
    }
  ]
}
```

---

## 9. Próximos Pasos

### Orden lógico:

1. **Implementar `adB check --json`** en Rust
2. **Definir warnings básicos** (emit, cpu, gpu)
3. **Crear extensión VS Code mínima**
4. **Publicar en marketplace**

---

## 10. Relación con ADead-BIB

Esta arquitectura sigue la filosofía de ADead-BIB:

> **No duplicar capas innecesarias**

- **Rust = autoridad** (compilador, análisis, validación)
- **Editor = interfaz** (presentación, UI, colores)

El editor **no analiza el código**, solo **le pregunta al compilador**.

---

## Resumen

```
┌─────────────────────────────────────────────────────────┐
│                    VS Code Extension                     │
│                        (JS/TS)                          │
│                                                         │
│   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   │
│   │ Syntax  │  │ Buttons │  │ Errors  │  │  UI     │   │
│   │Highlight│  │ & Menu  │  │ Display │  │ Panels  │   │
│   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘   │
│        │            │            │            │         │
│        └────────────┴─────┬──────┴────────────┘         │
│                           │                             │
│                           ▼                             │
│                    adB check --json                     │
│                           │                             │
└───────────────────────────┼─────────────────────────────┘
                            │
                            ▼
┌───────────────────────────────────────────────────────────┐
│                      adeadc (Rust)                        │
│                                                           │
│   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐     │
│   │ Parser  │──▶│ Type    │──▶│ Warning │──▶│  JSON   │     │
│   │         │  │ Checker │  │ Detect  │  │ Output  │     │
│   └─────────┘  └─────────┘  └─────────┘  └─────────┘     │
│                                                           │
│   🧠 El cerebro: análisis real, validación, autoridad    │
└───────────────────────────────────────────────────────────┘
```

**ADead-BIB Extension: Rust es el cerebro, JS es la cara.**
