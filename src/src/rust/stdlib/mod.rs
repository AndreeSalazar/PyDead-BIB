// ============================================================
// ADead-BIB Standard Library v7.0
// ============================================================
// PROPIA — Sin libc externa — Sin linker
//
// Cada fastos_*.rs implementa las funciones de su header
// correspondiente usando syscalls directos o instrucciones x87/SSE2.
//
// header_main.h hereda TODO automáticamente.
// Tree shaking garantiza que solo lo usado llega al binario.
// ============================================================

pub mod header_main;
pub mod c;
pub mod cpp;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod canon_tests;
#[cfg(test)]
mod fase_tests;

pub use header_main::HeaderMain;
