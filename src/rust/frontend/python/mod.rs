// ============================================================
// PyDead-BIB Python Frontend — Complete Pipeline
// ============================================================
// Python 2.7/3.0→3.13 → PyDead-BIB IR pipeline
//
// Python Source → PyPreprocessor → PyImportResolver → PyLexer →
//                 PyParser → PyAST → PyTypeInferencer → IR (ADeadOp)
//
// Modules:
//   py_preprocessor    — encoding detection, __future__ handling
//   py_import_resolver — static import resolution, dead import elimination
//   py_lexer           — Tokenizer: Python source → PyToken stream
//   py_ast             — Python AST types (PyExpr, PyStmt, etc.)
//   py_parser          — Recursive descent: PyToken → Python AST
//   py_types           — Type inference: duck typing → concrete static types
//   py_to_ir           — Python AST → ADeadOp IR (SSA-form)
//
// Sin CPython. Sin GIL. Sin runtime. Solo PyDead-BIB. 💀🦈
// ============================================================

pub mod ast;
pub mod py_import_resolver;
pub mod lexer;
pub mod parser;
pub mod py_preprocessor;
pub mod to_ir;
pub mod types;

pub use to_ir::compile_python_to_ir;
