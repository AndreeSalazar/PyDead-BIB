// ============================================================
// ADead-BIB C++ Frontend
// ============================================================
// Full C++11/14/17/20 to ADead-BIB IR pipeline
//
// Pipeline: C++ Source → CppLexer → CppParser → CppAST → CppToIR → Program
//
// Features:
//   - Classes with inheritance → Structs + devirtualized methods
//   - Templates → Monomorphized (only used instantiations)
//   - Namespaces → Flattened with prefixed names
//   - Lambdas → Inline closures
//   - Smart pointers → Raw pointers (zero overhead)
//   - Exceptions → Eliminated (error codes)
//   - RTTI → Eliminated
//   - STL containers → Recognized and optimized
//
// Supported C++ standards:
//   - C++11: auto, lambda, move, range-for, nullptr, constexpr
//   - C++14: generic lambdas, relaxed constexpr
//   - C++17: structured bindings, if constexpr, string_view
//   - C++20: concepts, coroutines, spaceship operator
//
// Sin GCC. Sin LLVM. Sin Clang. Solo ADead-BIB. 💀🦈
// ============================================================

pub mod cpp_ast;
pub mod cpp_compiler_extensions;
pub mod cpp_lexer;
pub mod cpp_parser;
pub mod cpp_preprocessor;
pub mod cpp_stdlib;
pub mod cpp_to_ir;

pub use cpp_to_ir::compile_cpp_to_program;
