// ============================================================
// ADead-BIB Preprocessor — Sin CMake, Sin Linker
// ============================================================
// header_main.h resolution, symbol deduplication,
// C++11-C++17 expansion to C++98 canon
// ============================================================

pub mod dedup;
pub mod expander;
pub mod resolver;

pub use dedup::SymbolDedup;
pub use expander::MacroExpander;
pub use resolver::HeaderResolver;
