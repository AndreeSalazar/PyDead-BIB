// ============================================================
// BG — Binary Guardian: Analyzer
// ============================================================
// Pipeline completo de análisis pre-execution.
//
//   Binary → Loader → ISA Decoder → ABIB IR
//     → Capability Mapper → Architecture Map
//       → Policy Engine → Verdict
//
// O(n) para generar el mapa. O(1) para evaluar.
// Determinista: mismo input + misma policy = mismo output.
//
// Autor: Eddi Andreé Salazar Matos
// ============================================================

use super::arch_map::ArchitectureMap;
use super::binary_loader::{BinaryInfo, BinaryLoader};
use super::capability::CapabilityMapper;
use super::policy::{PolicyEngine, SecurityLevel, SecurityPolicy, Verdict};
use crate::isa::decoder::Decoder;
use crate::isa::ADeadOp;
use std::fmt;
use std::path::Path;

// ============================================================
// Analysis Result
// ============================================================

/// Resultado completo de un análisis BG.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Información del binario (si fue cargado desde archivo/bytes)
    pub binary_info: Option<BinaryInfo>,
    /// Architecture Map — perfil estructural completo
    pub map: ArchitectureMap,
    /// Veredicto: APPROVED o DENIED
    pub verdict: Verdict,
    /// Nivel mínimo de seguridad inferido para ejecutar
    pub minimum_level: SecurityLevel,
    /// Total de instrucciones analizadas
    pub instruction_count: usize,
    /// Nombre de la policy usada
    pub policy_name: String,
}

impl fmt::Display for AnalysisResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "═══════════════════════════════════════════════")?;
        writeln!(f, "  BG — Binary Guardian: Analysis Result")?;
        writeln!(f, "═══════════════════════════════════════════════")?;
        writeln!(f)?;
        if let Some(ref info) = self.binary_info {
            writeln!(f, "  Binary:     {}", info.path)?;
            writeln!(f, "  Format:     {}", info.format)?;
            writeln!(f, "  Size:       {} bytes", info.total_size)?;
        }
        writeln!(f, "  Policy:     {}", self.policy_name)?;
        writeln!(f, "  Min level:  {}", self.minimum_level)?;
        writeln!(f, "  Opcodes:    {}", self.instruction_count)?;
        writeln!(f)?;
        writeln!(f, "{}", self.map)?;
        writeln!(f, "  ┌─ Verdict ──────────────────────────────┐")?;
        match &self.verdict {
            Verdict::Approved => {
                writeln!(f, "  │              ✓  APPROVED                │")?;
            }
            Verdict::Denied { violations } => {
                writeln!(f, "  │              ✗  DENIED                  │")?;
                writeln!(
                    f,
                    "  │  {} violation(s):                       │",
                    violations.len()
                )?;
                for v in violations {
                    writeln!(f, "  │    {}", v)?;
                }
            }
        }
        writeln!(f, "  └────────────────────────────────────────┘")?;
        writeln!(f, "═══════════════════════════════════════════════")?;
        Ok(())
    }
}

// ============================================================
// BinaryGuardian — API principal
// ============================================================

/// BinaryGuardian — Punto de entrada principal para análisis.
pub struct BinaryGuardian;

impl BinaryGuardian {
    /// Analiza un archivo binario completo (PE/ELF/Raw).
    /// Pipeline: Load → Decode → Map → Validate → Evaluate.
    pub fn analyze_file(path: &Path, policy: &SecurityPolicy) -> Result<AnalysisResult, String> {
        let info = BinaryLoader::load_file(path)?;
        let result = Self::analyze_loaded(&info, policy);
        Ok(result)
    }

    /// Analiza un binario ya cargado en memoria.
    pub fn analyze_loaded(info: &BinaryInfo, policy: &SecurityPolicy) -> AnalysisResult {
        let mut decoder = Decoder::new();
        let ops = decoder.decode_all(&info.code_bytes);
        let mut map = CapabilityMapper::analyze(&ops);

        // Poblar metadata del binario
        map.binary_name = Some(info.path.clone());
        map.binary_size = info.total_size;

        // ==== Memory Map desde BinaryInfo ====
        for section in &info.sections {
            let region_type = match section.kind {
                super::binary_loader::SectionKind::Code => super::arch_map::RegionType::Code,
                super::binary_loader::SectionKind::Data => super::arch_map::RegionType::Data,
                super::binary_loader::SectionKind::ReadOnly => {
                    super::arch_map::RegionType::ReadOnly
                }
                super::binary_loader::SectionKind::RWX => super::arch_map::RegionType::RWX,
                super::binary_loader::SectionKind::Unknown => super::arch_map::RegionType::Data,
            };

            if section.executable {
                map.memory_map.total_code_size += section.size;
            } else {
                map.memory_map.total_data_size += section.size;
            }

            map.memory_map.regions.push(super::arch_map::MemoryRegion {
                region_type,
                offset: section.offset,
                size: section.size,
                name: section.name.clone(),
            });
        }
        map.memory_map.rwx_count = info.rwx_count;

        // ==== Structural Integrity — NUEVO ====
        let (entry_valid, overlapping, anomalous) = BinaryLoader::validate_structure(info);
        map.integrity.entry_point_valid = entry_valid;
        map.integrity.entry_point_checked = true;
        map.integrity.overlapping_sections = overlapping;
        map.integrity.anomalous_permissions = anomalous;

        // Code-to-data ratio
        let total_content = map.memory_map.total_code_size + map.memory_map.total_data_size;
        if total_content > 0 {
            map.integrity.code_to_data_ratio =
                map.memory_map.total_code_size as f64 / total_content as f64;
        }

        // Header ratio
        if info.total_size > 0 {
            map.integrity.header_ratio = info.header_size as f64 / info.total_size as f64;
        }

        // Entry at section start
        map.integrity.entry_at_section_start = info
            .sections
            .iter()
            .any(|s| s.executable && info.entry_point == s.virtual_address);

        // ==== Import/Export Map — NUEVO ====
        for imp in &info.imports {
            let lib = if imp.library.is_empty() {
                "unknown".to_string()
            } else {
                imp.library.clone()
            };
            map.import_export_map
                .imports_by_library
                .entry(lib)
                .or_insert_with(Vec::new)
                .push(imp.function.clone());
            map.import_export_map.import_count += 1;

            // Categorizar determinísticamente
            map.import_export_map.categorize_import(&imp.function);
        }
        map.import_export_map.exports = info.exports.clone();
        map.import_export_map.export_count = info.exports.len();

        // ==== Evaluate ====
        let minimum_level = PolicyEngine::infer_minimum_level(&map);
        let verdict = PolicyEngine::evaluate(&map, policy);

        AnalysisResult {
            binary_info: Some(info.clone()),
            map,
            verdict,
            minimum_level,
            instruction_count: ops.len(),
            policy_name: policy.name.clone(),
        }
    }

    /// Analiza bytes crudos como código (sin formato de contenedor).
    pub fn analyze_bytes(bytes: &[u8], policy: &SecurityPolicy) -> AnalysisResult {
        let mut decoder = Decoder::new();
        let ops = decoder.decode_all(bytes);
        Self::analyze_ops(&ops, policy)
    }

    /// Analiza un vector de instrucciones ya decodificadas.
    pub fn analyze_ops(ops: &[ADeadOp], policy: &SecurityPolicy) -> AnalysisResult {
        let map = CapabilityMapper::analyze(ops);
        let minimum_level = PolicyEngine::infer_minimum_level(&map);
        let verdict = PolicyEngine::evaluate(&map, policy);

        AnalysisResult {
            binary_info: None,
            map,
            verdict,
            minimum_level,
            instruction_count: ops.len(),
            policy_name: policy.name.clone(),
        }
    }

    /// Quick check: ¿aprobaría este binario bajo la policy dada?
    pub fn quick_check(ops: &[ADeadOp], policy: &SecurityPolicy) -> bool {
        let map = CapabilityMapper::analyze(ops);
        PolicyEngine::evaluate(&map, policy).is_approved()
    }

    /// Inspect: retorna solo el ArchitectureMap sin evaluar policy.
    pub fn inspect(ops: &[ADeadOp]) -> ArchitectureMap {
        CapabilityMapper::analyze(ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::*;

    #[test]
    fn test_analyze_safe_ops() {
        let ops = vec![
            ADeadOp::Push {
                src: Operand::Reg(Reg::RBP),
            },
            ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBP),
                src: Operand::Reg(Reg::RSP),
            },
            ADeadOp::Xor {
                dst: Reg::EAX,
                src: Reg::EAX,
            },
            ADeadOp::Pop { dst: Reg::RBP },
            ADeadOp::Ret,
        ];
        let result = BinaryGuardian::analyze_ops(&ops, &SecurityPolicy::user());
        assert!(result.verdict.is_approved());
        assert_eq!(result.instruction_count, 5);
        assert_eq!(result.minimum_level, SecurityLevel::User);
    }

    #[test]
    fn test_quick_check() {
        let safe = vec![ADeadOp::Nop, ADeadOp::Ret];
        assert!(BinaryGuardian::quick_check(&safe, &SecurityPolicy::user()));

        let priv_ops = vec![ADeadOp::Cli, ADeadOp::Hlt];
        assert!(!BinaryGuardian::quick_check(
            &priv_ops,
            &SecurityPolicy::user()
        ));
    }

    #[test]
    fn test_raw_bytes_analysis() {
        // Empty bytes should produce empty but approved result
        let result = BinaryGuardian::analyze_bytes(&[], &SecurityPolicy::user());
        assert!(result.verdict.is_approved());
        assert_eq!(result.instruction_count, 0);
    }

    #[test]
    fn test_inspect() {
        let ops = vec![
            ADeadOp::Syscall,
            ADeadOp::InByte {
                port: Operand::Imm8(0x60),
            },
        ];
        let map = BinaryGuardian::inspect(&ops);
        assert!(map.capabilities.syscalls);
        assert!(map.capabilities.io_port_access);
    }

    #[test]
    fn test_display() {
        let ops = vec![ADeadOp::Nop, ADeadOp::Ret];
        let result = BinaryGuardian::analyze_ops(&ops, &SecurityPolicy::user());
        let display = format!("{}", result);
        assert!(display.contains("APPROVED"));
    }
}
