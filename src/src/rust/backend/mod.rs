// ============================================================
// ADead-BIB Backend — Binary Is Binary
// ============================================================
// Emits BYTES directly. No ASM intermediary. No linker.
//
// Structure:
//   cpu/ : x86-64 bytes → PE/ELF/RAW (FASM-inspired encoding)
//   gpu/ : GPU bytes → SPIR-V/CUDA/HEX
//
// Pipeline: Code → AST → ISA IR → Encoder → Bytes → Binary
// ============================================================

pub mod cpu;
pub mod gpu;

// Core format re-exports
pub use cpu::elf;
pub use cpu::flat_binary;
pub use cpu::pe;
pub use cpu::pe_tiny;

// Legacy re-exports (use isa::codegen instead for new code)
pub use cpu::codegen;
pub use cpu::codegen_v2;
pub use cpu::microvm;
pub use cpu::pe_minimal;
pub use cpu::syscalls;
pub use cpu::win32_resolver;
