# ADead-BIB ISA Module

Módulo central de compilación: AST → ADeadIR → x86-64 bytes.

## Archivos

| Archivo | Descripción |
|---------|-------------|
| `mod.rs` | Definiciones de ADeadOp, Reg, Operand, ADeadIR |
| `compiler/` | **Compilador modular (nuevo)** |
| `isa_compiler.rs` | Compilador monolítico (legacy) |
| `encoder.rs` | Codificador FASM-inspired IR→bytes |
| `decoder.rs` | Decodificador x86-64 bytes→texto |
| `optimizer.rs` | Optimizaciones de IR |
| `reg_alloc.rs` | Asignación de registros temporales |
| `codegen.rs` | Re-export alias |

## Estructura Modular (compiler/)

```text
compiler/
├── mod.rs          # Re-exports
├── core.rs         # Target, CpuMode, IsaCompiler struct
├── compile.rs      # compile(), string collection
├── functions.rs    # compile_function, prologue/epilogue
├── statements.rs   # emit_statement (Stmt::*)
├── expressions.rs  # emit_expression (Expr::*)
├── helpers.rs      # emit_print, emit_assign, emit_call
├── control_flow.rs # emit_if, emit_while, emit_for
└── arrays.rs       # emit_index_assign, emit_index_access
```

## Pipeline de Compilación

```text
C/C++ Source
    ↓
Lexer (c_lexer.rs / cpp_lexer.rs)
    ↓
Parser (c_parser.rs / cpp_parser.rs)
    ↓
AST (ast.rs)
    ↓
IR Converter (c_to_ir.rs / cpp_to_ir.rs)
    ↓
ISA Compiler (compiler/)  ← ESTE MÓDULO
    ↓
ADeadIR (Vec<ADeadOp>)
    ↓
Encoder (encoder.rs)
    ↓
x86-64 bytes
    ↓
PE/ELF Binary
```

## Uso

```rust
use adead_bib::isa::compiler::{IsaCompiler, Target};

let mut compiler = IsaCompiler::new(Target::Windows);
compiler.compile(&program);
```

## FASM-Inspired Design

El encoder está inspirado en FASM (Flat Assembler):

- Encoding genérico con REX+ModR/M computado
- Auto-selección imm8/imm32
- Auto-selección disp8/disp32
- Multi-pass para optimización de saltos

Sin ASM. Sin NASM. Sin LLVM. Solo ISA puro.
