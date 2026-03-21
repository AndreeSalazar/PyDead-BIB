// ============================================================
// ADead-BIB — CPU Backend (FASM-inspired architecture)
// ============================================================
// BINARY IS BINARY — Emits x86-64 bytes DIRECTLY.
// No ASM text. No external assembler. No linker.
//
// Architecture (inspired by FASM's FORMATS.INC):
//
//   Format Generators:
//   - pe.rs            : PE/COFF x64 generator (Windows .exe/.dll)
//   - pe_tiny.rs       : Ultra-compact PE (nano/micro, sub-1KB)
//   - elf.rs           : ELF64 generator (Linux)
//   - flat_binary.rs   : Raw flat binary (boot sectors, bare-metal)
//   - fastos_format.rs : FastOS custom format (magic: "FsOS")
//
//   Code Generation:
//   - codegen.rs       : Legacy codegen (use isa::codegen instead)
//   - codegen_v2.rs    : Legacy codegen v2 (use isa::codegen instead)
//   - os_codegen.rs    : OS-level: GDT/IDT/paging, multi-mode, Rust bridge
//   - syscalls.rs      : Direct syscalls Windows/Linux
//   - win32_resolver.rs: PE import resolution
//
//   Experimental:
//   - pe_compact.rs    : Experimental PE with SectionAlign=FileAlign=0x200
//   - pe_isa.rs        : Experimental PE with inline ISA
//   - pe_ultra.rs      : Experimental ultra-compact PE v2
//   - pe_minimal.rs    : Experimental minimal PE
//   - pe_valid.rs      : Experimental spec-validated PE
//   - binary_raw.rs    : Raw binary emitter
//   - microvm.rs       : MicroVM bytecode
//
// Pipeline: AST → ISA IR → Encoder → bytes → PE/ELF/RAW
// ============================================================

// === Core format generators ===
pub mod elf;
pub mod fastos_format;
pub mod flat_binary;
pub mod iat_registry;
pub mod pe;
pub mod pe_tiny;

// === Code generation ===
pub mod codegen;
pub mod codegen_v2;
pub mod os_codegen;
pub mod syscalls;
pub mod win32_resolver;

// === Experimental PE variants ===
pub mod pe_compact;
pub mod pe_isa;
pub mod pe_minimal;
pub mod pe_ultra;
pub mod pe_valid;

// === Other ===
pub mod binary_raw;
pub mod microvm;
