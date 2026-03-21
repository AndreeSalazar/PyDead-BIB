// ============================================================
// BG — Binary Guardian
// ============================================================
// Deterministic ISA-Level Capability Guardian
//
// No antivirus. No sandbox clásico. No heurísticas.
// Arquitectura de control estructural.
//
//   Binary → ISA Decoder → ABIB IR → Capability Mapper
//       → Architecture Map → Policy Engine → APPROVE / DENY
//
// ● Pre-execution: analiza una vez, genera mapa compacto.
// ● Deterministic: mismo binario + misma policy = mismo resultado.
// ● O(n) build, O(1) query.
// ● Directo al ISA: no depende de lenguaje, formato, ni alto nivel.
//
// Diseñado para FastOS loader integration.
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

pub mod analyzer;
pub mod arch_map;
pub mod binary_loader;
pub mod capability;
pub mod policy;

// Re-exports — API ergonómica
pub use analyzer::{AnalysisResult, BinaryGuardian};
pub use arch_map::{ArchitectureMap, Capabilities, InstructionClass};
pub use binary_loader::{BinaryInfo, BinaryLoader, SectionInfo, SectionKind};
pub use capability::CapabilityMapper;
pub use policy::{PolicyEngine, SecurityLevel, SecurityPolicy, Verdict, Violation, ViolationType};
