// ============================================================
// ADead-BIB Frontend v3.0
// C/C++ Native Frontends — No custom .adB syntax
// ============================================================

pub mod ast;
pub mod c;
pub mod cpp;
pub mod type_checker;
pub mod types;

// Legacy modules kept for compatibility but not actively used
#[allow(dead_code)]
pub mod lexer;
#[allow(dead_code)]
pub mod parser;
