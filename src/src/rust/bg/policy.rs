// ============================================================
// BG — Binary Guardian: Policy Engine
// ============================================================
// Evalúa un ArchitectureMap contra una SecurityPolicy.
//
// No heurísticas. No scoring. No probabilidades.
//
// Verdict = (ArchitectureMap ∩ AllowedCapabilities) ?
//   APPROVED : DENIED { violations }
//
// Determinista: mismo binario + misma policy = mismo resultado.
// Siempre. Cada vez.
//
// Diseñado para FastOS: el kernel loader usa esto para gate de ejecución.
//
// Autor: Eddi Andreé Salazar Matos
// ============================================================

use super::arch_map::*;
use std::fmt;

// ============================================================
// Security Level (mapea a CPU rings)
// ============================================================

/// Nivel de seguridad — mapea directamente a privilege rings x86-64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// Ring 0 — Acceso total al hardware. Solo kernel.
    Kernel = 0,
    /// Ring 1 — IO + ops restringidas. Drivers.
    Driver = 1,
    /// Ring 2 — Ops restringidas, sin IO directo. Services.
    Service = 2,
    /// Ring 3 — Solo instrucciones safe. Aplicaciones usuario.
    User = 3,
}

impl fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityLevel::Kernel => write!(f, "KERNEL (Ring 0)"),
            SecurityLevel::Driver => write!(f, "DRIVER (Ring 1)"),
            SecurityLevel::Service => write!(f, "SERVICE (Ring 2)"),
            SecurityLevel::User => write!(f, "USER (Ring 3)"),
        }
    }
}

// ============================================================
// Security Policy
// ============================================================

/// Una policy de seguridad que define qué puede hacer un binario.
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub name: String,
    pub level: SecurityLevel,
    /// Whitelist de vectores de syscall permitidos (None = todos los del nivel)
    pub allowed_syscall_vectors: Option<Vec<u8>>,
    /// Whitelist de puertos IO permitidos (None = todos los del nivel)
    pub allowed_io_ports: Option<Vec<u16>>,
    /// Máximo de sitios de control flow indirecto (None = ilimitado)
    pub max_indirect_sites: Option<usize>,
    /// Permitir regiones de memoria RWX
    pub allow_rwx: bool,
    /// Permitir código auto-modificante
    pub allow_self_modifying: bool,
    /// Permitir far jumps (cambios de segmento)
    pub allow_far_jumps: bool,
    /// Requerir integridad estructural limpia
    pub require_structural_integrity: bool,
    /// Permitir APIs de inyección de proceso
    pub allow_process_injection: bool,
}

impl SecurityPolicy {
    /// Policy para código kernel — todo permitido.
    pub fn kernel() -> Self {
        Self {
            name: "kernel".into(),
            level: SecurityLevel::Kernel,
            allowed_syscall_vectors: None,
            allowed_io_ports: None,
            max_indirect_sites: None,
            allow_rwx: true,
            allow_self_modifying: true,
            allow_far_jumps: true,
            require_structural_integrity: false,
            allow_process_injection: true,
        }
    }

    /// Policy para drivers — IO + restringido, sin CR/MSR/tablas de descriptores.
    pub fn driver() -> Self {
        Self {
            name: "driver".into(),
            level: SecurityLevel::Driver,
            allowed_syscall_vectors: None,
            allowed_io_ports: None,
            max_indirect_sites: None,
            allow_rwx: false,
            allow_self_modifying: false,
            allow_far_jumps: false,
            require_structural_integrity: true,
            allow_process_injection: false,
        }
    }

    /// Policy para servicios — solo syscalls, sin hardware directo.
    pub fn service() -> Self {
        Self {
            name: "service".into(),
            level: SecurityLevel::Service,
            allowed_syscall_vectors: None,
            allowed_io_ports: Some(Vec::new()),
            max_indirect_sites: Some(64),
            allow_rwx: false,
            allow_self_modifying: false,
            allow_far_jumps: false,
            require_structural_integrity: true,
            allow_process_injection: false,
        }
    }

    /// Policy para aplicaciones usuario — safe + syscalls solamente.
    pub fn user() -> Self {
        Self {
            name: "user".into(),
            level: SecurityLevel::User,
            allowed_syscall_vectors: None,
            allowed_io_ports: Some(Vec::new()),
            max_indirect_sites: Some(32),
            allow_rwx: false,
            allow_self_modifying: false,
            allow_far_jumps: false,
            require_structural_integrity: true,
            allow_process_injection: false,
        }
    }

    /// Sandbox estricto — casi nada permitido.
    pub fn sandbox() -> Self {
        Self {
            name: "sandbox".into(),
            level: SecurityLevel::User,
            allowed_syscall_vectors: Some(Vec::new()),
            allowed_io_ports: Some(Vec::new()),
            max_indirect_sites: Some(0),
            allow_rwx: false,
            allow_self_modifying: false,
            allow_far_jumps: false,
            require_structural_integrity: true,
            allow_process_injection: false,
        }
    }

    /// Policy personalizada con nombre.
    pub fn custom(name: &str, level: SecurityLevel) -> Self {
        Self {
            name: name.into(),
            level,
            allowed_syscall_vectors: None,
            allowed_io_ports: None,
            max_indirect_sites: None,
            allow_rwx: false,
            allow_self_modifying: false,
            allow_far_jumps: false,
            require_structural_integrity: true,
            allow_process_injection: false,
        }
    }
}

impl fmt::Display for SecurityPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Policy '{}' [{}]", self.name, self.level)
    }
}

// ============================================================
// Violation
// ============================================================

/// Tipo de violación de seguridad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationType {
    PrivilegedInstruction,
    UnauthorizedIO,
    UnauthorizedSyscall,
    RWXMemory,
    SelfModifyingCode,
    ExcessiveIndirectControl,
    UnauthorizedFarJump,
    ControlRegisterAccess,
    MSRAccess,
    DescriptorTableAccess,
    InterruptControl,
    // ---- NUEVAS violaciones pre-execution ----
    /// Entry point no apunta a una sección de código válida
    InvalidEntryPoint,
    /// Secciones del binario se solapan (anomalía estructural)
    OverlappingSections,
    /// Binario importa APIs de inyección de proceso
    ProcessInjectionImports,
    /// Secciones con permisos anómalos (data+execute)
    AnomalousPermissions,
}

impl fmt::Display for ViolationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViolationType::PrivilegedInstruction => write!(f, "PRIVILEGED_INSTRUCTION"),
            ViolationType::UnauthorizedIO => write!(f, "UNAUTHORIZED_IO"),
            ViolationType::UnauthorizedSyscall => write!(f, "UNAUTHORIZED_SYSCALL"),
            ViolationType::RWXMemory => write!(f, "RWX_MEMORY"),
            ViolationType::SelfModifyingCode => write!(f, "SELF_MODIFYING_CODE"),
            ViolationType::ExcessiveIndirectControl => write!(f, "EXCESSIVE_INDIRECT_CONTROL"),
            ViolationType::UnauthorizedFarJump => write!(f, "UNAUTHORIZED_FAR_JUMP"),
            ViolationType::ControlRegisterAccess => write!(f, "CONTROL_REGISTER_ACCESS"),
            ViolationType::MSRAccess => write!(f, "MSR_ACCESS"),
            ViolationType::DescriptorTableAccess => write!(f, "DESCRIPTOR_TABLE_ACCESS"),
            ViolationType::InterruptControl => write!(f, "INTERRUPT_CONTROL"),
            ViolationType::InvalidEntryPoint => write!(f, "INVALID_ENTRY_POINT"),
            ViolationType::OverlappingSections => write!(f, "OVERLAPPING_SECTIONS"),
            ViolationType::ProcessInjectionImports => write!(f, "PROCESS_INJECTION_IMPORTS"),
            ViolationType::AnomalousPermissions => write!(f, "ANOMALOUS_PERMISSIONS"),
        }
    }
}

/// Una violación de seguridad específica encontrada durante evaluación.
#[derive(Debug, Clone)]
pub struct Violation {
    pub kind: ViolationType,
    pub instruction_index: Option<usize>,
    pub description: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(idx) = self.instruction_index {
            write!(f, "[{}] @{}: {}", self.kind, idx, self.description)
        } else {
            write!(f, "[{}]: {}", self.kind, self.description)
        }
    }
}

// ============================================================
// Verdict
// ============================================================

/// Veredicto final: APPROVED o DENIED.
#[derive(Debug, Clone)]
pub enum Verdict {
    Approved,
    Denied { violations: Vec<Violation> },
}

impl Verdict {
    pub fn is_approved(&self) -> bool {
        matches!(self, Verdict::Approved)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, Verdict::Denied { .. })
    }

    pub fn violations(&self) -> &[Violation] {
        match self {
            Verdict::Approved => &[],
            Verdict::Denied { violations } => violations,
        }
    }

    pub fn violation_count(&self) -> usize {
        self.violations().len()
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Approved => write!(f, "APPROVED"),
            Verdict::Denied { violations } => {
                writeln!(f, "DENIED — {} violation(s):", violations.len())?;
                for v in violations {
                    writeln!(f, "    {}", v)?;
                }
                Ok(())
            }
        }
    }
}

// ============================================================
// Policy Engine
// ============================================================

/// Policy Engine — Evalúa un ArchitectureMap contra una SecurityPolicy.
///
/// Determinista. Misma entrada → misma salida. Siempre.
pub struct PolicyEngine;

impl PolicyEngine {
    /// Evalúa el architecture map de un binario contra una policy.
    /// Retorna APPROVED o DENIED con lista de violaciones específicas.
    pub fn evaluate(map: &ArchitectureMap, policy: &SecurityPolicy) -> Verdict {
        let mut violations = Vec::new();

        match policy.level {
            SecurityLevel::Kernel => {
                // Kernel puede hacer todo — sin checks de instrucciones
            }
            SecurityLevel::Driver => {
                Self::check_driver_violations(map, &mut violations);
            }
            SecurityLevel::Service => {
                Self::check_service_violations(map, policy, &mut violations);
            }
            SecurityLevel::User => {
                Self::check_user_violations(map, policy, &mut violations);
            }
        }

        // Checks universales (aplican a todos los niveles excepto Kernel)
        if policy.level != SecurityLevel::Kernel {
            Self::check_universal(map, policy, &mut violations);
        }

        // Checks de integridad estructural — NUEVO
        if policy.require_structural_integrity {
            Self::check_structural_integrity(map, &mut violations);
        }

        // Checks de imports — NUEVO
        if !policy.allow_process_injection {
            Self::check_import_violations(map, &mut violations);
        }

        if violations.is_empty() {
            Verdict::Approved
        } else {
            Verdict::Denied { violations }
        }
    }

    /// Infiere el nivel de seguridad mínimo requerido para ejecutar un binario.
    pub fn infer_minimum_level(map: &ArchitectureMap) -> SecurityLevel {
        if map.capabilities.requires_kernel() {
            if map.capabilities.control_register_access
                || map.capabilities.msr_access
                || map.capabilities.descriptor_table_access
                || map.capabilities.interrupt_control
            {
                SecurityLevel::Kernel
            } else {
                SecurityLevel::Driver
            }
        } else if map.capabilities.syscalls || map.capabilities.interrupts {
            SecurityLevel::Service
        } else {
            SecurityLevel::User
        }
    }

    fn check_driver_violations(map: &ArchitectureMap, violations: &mut Vec<Violation>) {
        if map.capabilities.control_register_access {
            violations.push(Violation {
                kind: ViolationType::ControlRegisterAccess,
                instruction_index: None,
                description: "Driver cannot access control registers (CR0-CR4)".into(),
            });
        }
        if map.capabilities.msr_access {
            violations.push(Violation {
                kind: ViolationType::MSRAccess,
                instruction_index: None,
                description: "Driver cannot access MSRs (RDMSR/WRMSR)".into(),
            });
        }
        if map.capabilities.descriptor_table_access {
            violations.push(Violation {
                kind: ViolationType::DescriptorTableAccess,
                instruction_index: None,
                description: "Driver cannot modify GDT/IDT (LGDT/LIDT)".into(),
            });
        }
    }

    fn check_service_violations(
        map: &ArchitectureMap,
        policy: &SecurityPolicy,
        violations: &mut Vec<Violation>,
    ) {
        if map.capabilities.privileged_instructions {
            for (idx, class) in &map.instruction_map.flagged {
                if *class == InstructionClass::Privileged {
                    violations.push(Violation {
                        kind: ViolationType::PrivilegedInstruction,
                        instruction_index: Some(*idx),
                        description: "Service cannot use privileged instructions".into(),
                    });
                }
            }
        }

        if let Some(ref allowed) = policy.allowed_io_ports {
            for access in &map.io_map.accesses {
                let denied = match access.port {
                    Some(port) => !allowed.contains(&port),
                    None => true,
                };
                if denied {
                    violations.push(Violation {
                        kind: ViolationType::UnauthorizedIO,
                        instruction_index: Some(access.instruction_index),
                        description: format!("Service cannot access IO port {:?}", access.port),
                    });
                }
            }
        }
    }

    fn check_user_violations(
        map: &ArchitectureMap,
        policy: &SecurityPolicy,
        violations: &mut Vec<Violation>,
    ) {
        if map.capabilities.privileged_instructions {
            for (idx, class) in &map.instruction_map.flagged {
                if *class == InstructionClass::Privileged {
                    violations.push(Violation {
                        kind: ViolationType::PrivilegedInstruction,
                        instruction_index: Some(*idx),
                        description: "User code cannot use privileged instructions".into(),
                    });
                }
            }
        }

        if let Some(ref allowed) = policy.allowed_io_ports {
            for access in &map.io_map.accesses {
                let denied = match access.port {
                    Some(port) => !allowed.contains(&port),
                    None => true,
                };
                if denied {
                    violations.push(Violation {
                        kind: ViolationType::UnauthorizedIO,
                        instruction_index: Some(access.instruction_index),
                        description: format!("User code cannot access IO port {:?}", access.port),
                    });
                }
            }
        }

        if let Some(ref allowed_vectors) = policy.allowed_syscall_vectors {
            for vector in &map.syscall_map.interrupt_vectors {
                if !allowed_vectors.contains(vector) {
                    violations.push(Violation {
                        kind: ViolationType::UnauthorizedSyscall,
                        instruction_index: None,
                        description: format!("User code cannot use INT 0x{:02X}", vector),
                    });
                }
            }
        }
    }

    fn check_universal(
        map: &ArchitectureMap,
        policy: &SecurityPolicy,
        violations: &mut Vec<Violation>,
    ) {
        if !policy.allow_rwx && map.memory_map.rwx_count > 0 {
            violations.push(Violation {
                kind: ViolationType::RWXMemory,
                instruction_index: None,
                description: format!("{} RWX memory region(s) detected", map.memory_map.rwx_count),
            });
        }

        if !policy.allow_self_modifying && map.memory_map.self_modifying_code {
            violations.push(Violation {
                kind: ViolationType::SelfModifyingCode,
                instruction_index: None,
                description: "Self-modifying code detected".into(),
            });
        }

        if !policy.allow_far_jumps && map.control_flow_map.far_jumps > 0 {
            violations.push(Violation {
                kind: ViolationType::UnauthorizedFarJump,
                instruction_index: None,
                description: format!("{} far jump(s) detected", map.control_flow_map.far_jumps),
            });
        }

        if let Some(max) = policy.max_indirect_sites {
            let total = map.control_flow_map.indirect_sites.len();
            if total > max {
                violations.push(Violation {
                    kind: ViolationType::ExcessiveIndirectControl,
                    instruction_index: None,
                    description: format!("{} indirect control flow sites (max: {})", total, max),
                });
            }
        }
    }

    /// Checks de integridad estructural — NUEVO.
    /// Solo aplican cuando hay info de estructura del binario.
    fn check_structural_integrity(map: &ArchitectureMap, violations: &mut Vec<Violation>) {
        if map.integrity.entry_point_checked && !map.integrity.entry_point_valid {
            violations.push(Violation {
                kind: ViolationType::InvalidEntryPoint,
                instruction_index: None,
                description: "Entry point does not reference a valid code section".into(),
            });
        }

        if map.integrity.overlapping_sections {
            violations.push(Violation {
                kind: ViolationType::OverlappingSections,
                instruction_index: None,
                description: "Binary contains overlapping sections (structural anomaly)".into(),
            });
        }

        if map.integrity.anomalous_permissions > 0 {
            violations.push(Violation {
                kind: ViolationType::AnomalousPermissions,
                instruction_index: None,
                description: format!(
                    "{} section(s) with anomalous permissions (writable+executable)",
                    map.integrity.anomalous_permissions
                ),
            });
        }
    }

    /// Checks de imports — NUEVO.
    /// Detecta imports de APIs de inyección de proceso.
    fn check_import_violations(map: &ArchitectureMap, violations: &mut Vec<Violation>) {
        if !map.import_export_map.process_injection_apis.is_empty() {
            violations.push(Violation {
                kind: ViolationType::ProcessInjectionImports,
                instruction_index: None,
                description: format!(
                    "Binary imports {} process injection API(s): {}",
                    map.import_export_map.process_injection_apis.len(),
                    map.import_export_map.process_injection_apis.join(", ")
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bg::capability::CapabilityMapper;
    use crate::isa::*;

    #[test]
    fn test_safe_approved() {
        let ops = vec![
            ADeadOp::Push {
                src: Operand::Reg(Reg::RBP),
            },
            ADeadOp::Mov {
                dst: Operand::Reg(Reg::RBP),
                src: Operand::Reg(Reg::RSP),
            },
            ADeadOp::Ret,
        ];
        let map = CapabilityMapper::analyze(&ops);
        let verdict = PolicyEngine::evaluate(&map, &SecurityPolicy::user());
        assert!(verdict.is_approved());
    }

    #[test]
    fn test_kernel_denied_for_user() {
        let ops = vec![ADeadOp::Cli, ADeadOp::Sti];
        let map = CapabilityMapper::analyze(&ops);
        let verdict = PolicyEngine::evaluate(&map, &SecurityPolicy::user());
        assert!(verdict.is_denied());
    }

    #[test]
    fn test_kernel_approved_for_kernel() {
        let ops = vec![
            ADeadOp::Cli,
            ADeadOp::MovToCr {
                cr: 0,
                src: Reg::RAX,
            },
            ADeadOp::Wrmsr,
        ];
        let map = CapabilityMapper::analyze(&ops);
        let verdict = PolicyEngine::evaluate(&map, &SecurityPolicy::kernel());
        assert!(verdict.is_approved());
    }

    #[test]
    fn test_infer_levels() {
        let user_ops = vec![ADeadOp::Add {
            dst: Operand::Reg(Reg::RAX),
            src: Operand::Imm8(1),
        }];
        assert_eq!(
            PolicyEngine::infer_minimum_level(&CapabilityMapper::analyze(&user_ops)),
            SecurityLevel::User
        );

        let kern_ops = vec![ADeadOp::MovToCr {
            cr: 3,
            src: Reg::RAX,
        }];
        assert_eq!(
            PolicyEngine::infer_minimum_level(&CapabilityMapper::analyze(&kern_ops)),
            SecurityLevel::Kernel
        );
    }

    #[test]
    fn test_structural_integrity_violation() {
        let ops = vec![ADeadOp::Ret];
        let mut map = CapabilityMapper::analyze(&ops);

        // Simular entry point inválido
        map.integrity.entry_point_checked = true;
        map.integrity.entry_point_valid = false;
        map.integrity.overlapping_sections = true;

        let verdict = PolicyEngine::evaluate(&map, &SecurityPolicy::user());
        assert!(verdict.is_denied());
        assert!(verdict
            .violations()
            .iter()
            .any(|v| v.kind == ViolationType::InvalidEntryPoint));
        assert!(verdict
            .violations()
            .iter()
            .any(|v| v.kind == ViolationType::OverlappingSections));
    }

    #[test]
    fn test_injection_api_violation() {
        let ops = vec![ADeadOp::Ret];
        let mut map = CapabilityMapper::analyze(&ops);

        // Simular import de API de inyección
        map.import_export_map
            .process_injection_apis
            .push("WriteProcessMemory".into());

        let verdict = PolicyEngine::evaluate(&map, &SecurityPolicy::user());
        assert!(verdict.is_denied());
        assert!(verdict
            .violations()
            .iter()
            .any(|v| v.kind == ViolationType::ProcessInjectionImports));
    }

    #[test]
    fn test_kernel_allows_everything() {
        let ops = vec![ADeadOp::Ret];
        let mut map = CapabilityMapper::analyze(&ops);

        // Even with structural issues and injection APIs, kernel allows everything
        map.integrity.entry_point_checked = true;
        map.integrity.entry_point_valid = false;
        map.import_export_map
            .process_injection_apis
            .push("WriteProcessMemory".into());

        let verdict = PolicyEngine::evaluate(&map, &SecurityPolicy::kernel());
        assert!(verdict.is_approved());
    }
}
