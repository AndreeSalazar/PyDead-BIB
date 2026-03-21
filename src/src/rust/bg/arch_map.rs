// ============================================================
// BG — Binary Guardian: Architecture Map
// ============================================================
// Perfil estructural completo de un binario.
// Derivado determinísticamente del ABIB IR (ADeadOp).
//
// No heurísticas. No firmas. Análisis matemático puro.
//
// Se genera una vez (O(n)), se consulta en O(1).
//
// Autor: Eddi Andreé Salazar Matos
// ============================================================

use std::collections::HashMap;
use std::fmt;

// ============================================================
// Instruction Classification
// ============================================================

/// Clasificación determinista del nivel de privilegio de una instrucción.
/// Derivada estructuralmente del opcode — no de patrones ni heurísticas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionClass {
    /// Safe: sin efectos más allá de manipulación registro/memoria.
    /// mov, add, sub, xor, cmp, push, pop, jmp, jcc, call (directo), ret, nop
    Safe,
    /// Restricted: cruza frontera user/kernel o modifica contexto de ejecución.
    /// syscall, int N, sysret, iret
    Restricted,
    /// Privileged: requiere Ring 0 — control de hardware, tablas de descriptores, MSRs.
    /// cli, sti, hlt, lgdt, lidt, mov crN, rdmsr, wrmsr, invlpg, in, out
    Privileged,
}

impl fmt::Display for InstructionClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstructionClass::Safe => write!(f, "SAFE"),
            InstructionClass::Restricted => write!(f, "RESTRICTED"),
            InstructionClass::Privileged => write!(f, "PRIVILEGED"),
        }
    }
}

// ============================================================
// Instruction Map
// ============================================================

/// Clasifica cada instrucción del binario por nivel de privilegio.
#[derive(Debug, Clone)]
pub struct InstructionMap {
    pub total: usize,
    pub safe_count: usize,
    pub restricted_count: usize,
    pub privileged_count: usize,
    /// (instruction_index, class) solo para instrucciones non-safe
    pub flagged: Vec<(usize, InstructionClass)>,
}

impl InstructionMap {
    pub fn new() -> Self {
        Self {
            total: 0,
            safe_count: 0,
            restricted_count: 0,
            privileged_count: 0,
            flagged: Vec::new(),
        }
    }

    /// True si el binario contiene cero instrucciones privilegiadas.
    pub fn is_unprivileged(&self) -> bool {
        self.privileged_count == 0
    }

    /// Porcentaje de instrucciones safe.
    pub fn safe_ratio(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        self.safe_count as f64 / self.total as f64
    }
}

// ============================================================
// Memory Map
// ============================================================

/// Tipo de región de memoria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// Execute-only code
    Code,
    /// Read-write data
    Data,
    /// Read-only data
    ReadOnly,
    /// Read-Write-Execute — sospechoso, posible inyección de código
    RWX,
}

impl fmt::Display for RegionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegionType::Code => write!(f, "CODE"),
            RegionType::Data => write!(f, "DATA"),
            RegionType::ReadOnly => write!(f, "RODATA"),
            RegionType::RWX => write!(f, "RWX"),
        }
    }
}

/// Una región de memoria detectada en el binario.
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub region_type: RegionType,
    pub offset: usize,
    pub size: usize,
    pub name: String,
}

/// Perfil de layout de memoria.
#[derive(Debug, Clone)]
pub struct MemoryMap {
    pub regions: Vec<MemoryRegion>,
    pub rwx_count: usize,
    pub self_modifying_code: bool,
    pub total_code_size: usize,
    pub total_data_size: usize,
}

impl MemoryMap {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            rwx_count: 0,
            self_modifying_code: false,
            total_code_size: 0,
            total_data_size: 0,
        }
    }

    /// True si no hay regiones RWX ni código auto-modificante.
    pub fn is_clean(&self) -> bool {
        self.rwx_count == 0 && !self.self_modifying_code
    }
}

// ============================================================
// Syscall Map
// ============================================================

/// Catálogo de uso de llamadas al sistema.
#[derive(Debug, Clone)]
pub struct SyscallMap {
    pub syscall_count: usize,
    pub interrupt_vectors: Vec<u8>,
    pub uses_syscall_instruction: bool,
    /// Índices de instrucciones donde ocurren syscalls/interrupciones
    pub call_sites: Vec<usize>,
}

impl SyscallMap {
    pub fn new() -> Self {
        Self {
            syscall_count: 0,
            interrupt_vectors: Vec::new(),
            uses_syscall_instruction: false,
            call_sites: Vec::new(),
        }
    }

    pub fn has_syscalls(&self) -> bool {
        self.syscall_count > 0 || !self.interrupt_vectors.is_empty()
    }
}

// ============================================================
// IO Map
// ============================================================

/// Dirección de una operación de puerto IO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IODirection {
    In,
    Out,
}

/// Un acceso a puerto IO detectado.
#[derive(Debug, Clone)]
pub struct IOAccess {
    /// Número de puerto estático, o None si dinámico (vía registro DX)
    pub port: Option<u16>,
    pub direction: IODirection,
    pub instruction_index: usize,
}

/// Perfil de acceso a puertos IO.
#[derive(Debug, Clone)]
pub struct IOMap {
    pub accesses: Vec<IOAccess>,
}

impl IOMap {
    pub fn new() -> Self {
        Self {
            accesses: Vec::new(),
        }
    }

    pub fn has_io(&self) -> bool {
        !self.accesses.is_empty()
    }

    pub fn static_ports(&self) -> Vec<u16> {
        self.accesses.iter().filter_map(|a| a.port).collect()
    }

    /// Puertos únicos usados.
    pub fn unique_ports(&self) -> Vec<u16> {
        let mut ports = self.static_ports();
        ports.sort();
        ports.dedup();
        ports
    }
}

// ============================================================
// Control Flow Map
// ============================================================

/// Perfil estructural de control de flujo.
#[derive(Debug, Clone)]
pub struct ControlFlowMap {
    pub direct_jumps: usize,
    pub indirect_jumps: usize,
    pub direct_calls: usize,
    pub indirect_calls: usize,
    pub conditional_branches: usize,
    pub far_jumps: usize,
    /// Índices de instrucciones con jumps/calls indirectos (potenciales gadgets)
    pub indirect_sites: Vec<usize>,
}

impl ControlFlowMap {
    pub fn new() -> Self {
        Self {
            direct_jumps: 0,
            indirect_jumps: 0,
            direct_calls: 0,
            indirect_calls: 0,
            conditional_branches: 0,
            far_jumps: 0,
            indirect_sites: Vec::new(),
        }
    }

    pub fn total_branches(&self) -> usize {
        self.direct_jumps + self.indirect_jumps + self.conditional_branches + self.far_jumps
    }

    pub fn has_indirect_control(&self) -> bool {
        self.indirect_jumps > 0 || self.indirect_calls > 0
    }
}

// ============================================================
// Structural Integrity — NUEVO
// ============================================================

/// Perfil de integridad estructural del binario.
///
/// Análisis puramente determinista — detecta anomalías estructurales
/// que ningún binario legítimo debería tener.
#[derive(Debug, Clone)]
pub struct StructuralIntegrity {
    /// ¿El entry point apunta a una sección de código válida?
    pub entry_point_valid: bool,
    /// ¿Se pudo validar el entry point? (false si no hay info de secciones)
    pub entry_point_checked: bool,
    /// Proporción código/datos (ratio extremo = anomalía estructural)
    pub code_to_data_ratio: f64,
    /// ¿Hay secciones que se solapan? (anomalía en PE/ELF)
    pub overlapping_sections: bool,
    /// Número de secciones con permisos anormales (ej: data+execute sin code flag)
    pub anomalous_permissions: usize,
    /// ¿El entry point está en el inicio de una sección? (normal en binarios legítimos)
    pub entry_at_section_start: bool,
    /// Tamaño del header vs tamaño del binario (headers absurdamente grandes = sospechoso)
    pub header_ratio: f64,
}

impl StructuralIntegrity {
    pub fn new() -> Self {
        Self {
            entry_point_valid: true,
            entry_point_checked: false,
            code_to_data_ratio: 0.0,
            overlapping_sections: false,
            anomalous_permissions: 0,
            entry_at_section_start: true,
            header_ratio: 0.0,
        }
    }

    /// True si la estructura del binario es completamente limpia.
    pub fn is_clean(&self) -> bool {
        (!self.entry_point_checked || self.entry_point_valid)
            && !self.overlapping_sections
            && self.anomalous_permissions == 0
    }
}

// ============================================================
// Import/Export Map — NUEVO
// ============================================================

/// Perfil de imports/exports del binario — análisis estructural de la tabla
/// de importaciones (IAT/PLT) y exportaciones (EAT).
///
/// No es heurístico: clasifica APIs por categoría funcional determinista.
#[derive(Debug, Clone)]
pub struct ImportExportMap {
    /// Total de funciones importadas
    pub import_count: usize,
    /// Total de funciones exportadas
    pub export_count: usize,
    /// Imports agrupados por DLL/library
    pub imports_by_library: HashMap<String, Vec<String>>,
    /// Exports como lista de nombres
    pub exports: Vec<String>,
    /// APIs de manipulación de memoria/proceso (clasificación determinista)
    pub memory_manipulation_apis: Vec<String>,
    /// APIs de acceso a red/socket
    pub network_apis: Vec<String>,
    /// APIs de acceso a filesystem
    pub filesystem_apis: Vec<String>,
    /// APIs de criptografía
    pub crypto_apis: Vec<String>,
    /// APIs de inyección/hooking de proceso
    pub process_injection_apis: Vec<String>,
}

impl ImportExportMap {
    pub fn new() -> Self {
        Self {
            import_count: 0,
            export_count: 0,
            imports_by_library: HashMap::new(),
            exports: Vec::new(),
            memory_manipulation_apis: Vec::new(),
            network_apis: Vec::new(),
            filesystem_apis: Vec::new(),
            crypto_apis: Vec::new(),
            process_injection_apis: Vec::new(),
        }
    }

    /// True si no se detectaron imports de inyección/hooking.
    pub fn is_clean(&self) -> bool {
        self.process_injection_apis.is_empty()
    }

    /// True si el binario importa APIs de red.
    pub fn has_network(&self) -> bool {
        !self.network_apis.is_empty()
    }

    /// Categoriza determinísticamente un nombre de API importada.
    pub fn categorize_import(&mut self, api_name: &str) {
        let upper = api_name.to_uppercase();

        // Clasificación determinista: si el nombre contiene el patrón, pertenece a la categoría.
        // No hay scoring ni probabilidad — es una tabla de lookup.

        // Manipulación de memoria
        const MEMORY_APIS: &[&str] = &[
            "VIRTUALALLOC",
            "VIRTUALPROTECT",
            "VIRTUALFREE",
            "VIRTUALALLOCEX",
            "VIRTUALPROTECTEX",
            "HEAPALLOC",
            "HEAPFREE",
            "NTMAPVIEWOFSECTION",
            "NTUNMAPVIEWOFSECTION",
            "MMAP",
            "MPROTECT",
            "MUNMAP",
        ];
        for pat in MEMORY_APIS {
            if upper.contains(pat) {
                self.memory_manipulation_apis.push(api_name.to_string());
                return;
            }
        }

        // Inyección de proceso
        const INJECTION_APIS: &[&str] = &[
            "WRITEPROCESSMEMORY",
            "READPROCESSMEMORY",
            "CREATEREMOTETHREAD",
            "NTQUEUEAPCTHREAD",
            "SETWINDOWSHOOKEX",
            "SETTHREADCONTEXT",
            "NTWRITEVIRTUALMEMORY",
            "NTREADVIRTUALMEMORY",
            "PTRACE",
        ];
        for pat in INJECTION_APIS {
            if upper.contains(pat) {
                self.process_injection_apis.push(api_name.to_string());
                return;
            }
        }

        // Red/Socket
        const NETWORK_APIS: &[&str] = &[
            "WSASTARTUP",
            "SOCKET",
            "CONNECT",
            "SEND",
            "RECV",
            "BIND",
            "LISTEN",
            "ACCEPT",
            "GETADDRINFO",
            "INTERNETOPEN",
            "HTTPOPENREQUEST",
            "URLDOWNLOAD",
            "WINHTTP",
        ];
        for pat in NETWORK_APIS {
            if upper.contains(pat) {
                self.network_apis.push(api_name.to_string());
                return;
            }
        }

        // Filesystem
        const FS_APIS: &[&str] = &[
            "CREATEFILE",
            "WRITEFILE",
            "READFILE",
            "DELETEFILE",
            "MOVEFILE",
            "COPYFILE",
            "FINDFIRSTFILE",
            "FINDNEXTFILE",
            "OPEN",
            "WRITE",
            "READ",
            "UNLINK",
            "STAT",
            "FSTAT",
        ];
        for pat in FS_APIS {
            if upper.contains(pat) {
                self.filesystem_apis.push(api_name.to_string());
                return;
            }
        }

        // Criptografía
        const CRYPTO_APIS: &[&str] = &[
            "CRYPTACQUIRECONTEXT",
            "CRYPTENCRYPT",
            "CRYPTDECRYPT",
            "CRYPTGENRANDOM",
            "BCRYPT",
            "NCRYPT",
        ];
        for pat in CRYPTO_APIS {
            if upper.contains(pat) {
                self.crypto_apis.push(api_name.to_string());
                return;
            }
        }
    }
}

// ============================================================
// Capability Flags
// ============================================================

/// Resumen compacto de capacidades — qué PUEDE hacer el binario.
#[derive(Debug, Clone)]
pub struct Capabilities {
    pub privileged_instructions: bool,
    pub io_port_access: bool,
    pub syscalls: bool,
    pub interrupts: bool,
    pub indirect_control_flow: bool,
    pub self_modifying_code: bool,
    pub control_register_access: bool,
    pub interrupt_control: bool,
    pub msr_access: bool,
    pub descriptor_table_access: bool,
    pub far_jumps: bool,
}

impl Capabilities {
    pub fn none() -> Self {
        Self {
            privileged_instructions: false,
            io_port_access: false,
            syscalls: false,
            interrupts: false,
            indirect_control_flow: false,
            self_modifying_code: false,
            control_register_access: false,
            interrupt_control: false,
            msr_access: false,
            descriptor_table_access: false,
            far_jumps: false,
        }
    }

    /// True si el binario requiere Ring 0 para ejecutarse.
    pub fn requires_kernel(&self) -> bool {
        self.privileged_instructions
            || self.io_port_access
            || self.control_register_access
            || self.interrupt_control
            || self.msr_access
            || self.descriptor_table_access
    }

    /// True si el binario es puramente computacional (safe para Ring 3).
    pub fn is_pure_userspace(&self) -> bool {
        !self.requires_kernel() && !self.self_modifying_code && !self.far_jumps
    }

    /// Cuenta cuántas capacidades están activas.
    pub fn active_count(&self) -> usize {
        let flags = [
            self.privileged_instructions,
            self.io_port_access,
            self.syscalls,
            self.interrupts,
            self.indirect_control_flow,
            self.self_modifying_code,
            self.control_register_access,
            self.interrupt_control,
            self.msr_access,
            self.descriptor_table_access,
            self.far_jumps,
        ];
        flags.iter().filter(|&&f| f).count()
    }
}

// ============================================================
// Architecture Map — Perfil Completo del Binario
// ============================================================

/// Perfil estructural completo de un binario.
///
/// Se genera una vez via análisis estático. O(n) para construir, O(1) para consultar.
/// Este es el output core del pipeline de análisis de BG.
#[derive(Debug, Clone)]
pub struct ArchitectureMap {
    pub instruction_map: InstructionMap,
    pub memory_map: MemoryMap,
    pub syscall_map: SyscallMap,
    pub io_map: IOMap,
    pub control_flow_map: ControlFlowMap,
    pub capabilities: Capabilities,
    /// Integridad estructural del binario
    pub integrity: StructuralIntegrity,
    /// Imports/Exports del binario
    pub import_export_map: ImportExportMap,
    /// Nombre/ruta del binario analizado (si disponible)
    pub binary_name: Option<String>,
    /// Tamaño total del binario en bytes
    pub binary_size: usize,
}

impl ArchitectureMap {
    pub fn new() -> Self {
        Self {
            instruction_map: InstructionMap::new(),
            memory_map: MemoryMap::new(),
            syscall_map: SyscallMap::new(),
            io_map: IOMap::new(),
            control_flow_map: ControlFlowMap::new(),
            capabilities: Capabilities::none(),
            integrity: StructuralIntegrity::new(),
            import_export_map: ImportExportMap::new(),
            binary_name: None,
            binary_size: 0,
        }
    }
}

impl fmt::Display for ArchitectureMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "═══════════════════════════════════════════════")?;
        writeln!(f, "  BG — Binary Guardian: Architecture Map")?;
        writeln!(f, "═══════════════════════════════════════════════")?;
        if let Some(ref name) = self.binary_name {
            writeln!(f, "  Binary:           {}", name)?;
        }
        if self.binary_size > 0 {
            writeln!(f, "  Size:             {} bytes", self.binary_size)?;
        }
        writeln!(f)?;

        // Instruction Map
        writeln!(f, "  ┌─ Instruction Map ──────────────────────┐")?;
        writeln!(
            f,
            "  │ Total:          {:>8}               │",
            self.instruction_map.total
        )?;
        writeln!(
            f,
            "  │ Safe:           {:>8}  ({:.1}%)       │",
            self.instruction_map.safe_count,
            self.instruction_map.safe_ratio() * 100.0
        )?;
        writeln!(
            f,
            "  │ Restricted:     {:>8}               │",
            self.instruction_map.restricted_count
        )?;
        writeln!(
            f,
            "  │ Privileged:     {:>8}               │",
            self.instruction_map.privileged_count
        )?;
        writeln!(f, "  └────────────────────────────────────────┘")?;
        writeln!(f)?;

        // Memory Map
        writeln!(f, "  ┌─ Memory Map ────────────────────────────┐")?;
        writeln!(
            f,
            "  │ Regions:        {:>8}               │",
            self.memory_map.regions.len()
        )?;
        writeln!(
            f,
            "  │ Code size:      {:>8} bytes         │",
            self.memory_map.total_code_size
        )?;
        writeln!(
            f,
            "  │ Data size:      {:>8} bytes         │",
            self.memory_map.total_data_size
        )?;
        writeln!(
            f,
            "  │ RWX regions:    {:>8}               │",
            self.memory_map.rwx_count
        )?;
        writeln!(
            f,
            "  │ Self-modifying: {:>8}               │",
            if self.memory_map.self_modifying_code {
                "YES ⚠"
            } else {
                "no"
            }
        )?;
        writeln!(f, "  └────────────────────────────────────────┘")?;
        writeln!(f)?;

        // Syscall Map
        writeln!(f, "  ┌─ Syscall Map ───────────────────────────┐")?;
        writeln!(
            f,
            "  │ Syscalls:       {:>8}               │",
            self.syscall_map.syscall_count
        )?;
        writeln!(
            f,
            "  │ INT vectors:    {:?}",
            self.syscall_map.interrupt_vectors
        )?;
        writeln!(
            f,
            "  │ Uses SYSCALL:   {:>8}               │",
            if self.syscall_map.uses_syscall_instruction {
                "yes"
            } else {
                "no"
            }
        )?;
        writeln!(f, "  └────────────────────────────────────────┘")?;
        writeln!(f)?;

        // IO Map
        writeln!(f, "  ┌─ IO Map ────────────────────────────────┐")?;
        writeln!(
            f,
            "  │ Port accesses:  {:>8}               │",
            self.io_map.accesses.len()
        )?;
        writeln!(f, "  │ Unique ports:   {:?}", self.io_map.unique_ports())?;
        writeln!(f, "  └────────────────────────────────────────┘")?;
        writeln!(f)?;

        // Control Flow Map
        writeln!(f, "  ┌─ Control Flow Map ──────────────────────┐")?;
        writeln!(
            f,
            "  │ Direct jumps:   {:>8}               │",
            self.control_flow_map.direct_jumps
        )?;
        writeln!(
            f,
            "  │ Indirect jumps: {:>8}               │",
            self.control_flow_map.indirect_jumps
        )?;
        writeln!(
            f,
            "  │ Direct calls:   {:>8}               │",
            self.control_flow_map.direct_calls
        )?;
        writeln!(
            f,
            "  │ Indirect calls: {:>8}               │",
            self.control_flow_map.indirect_calls
        )?;
        writeln!(
            f,
            "  │ Conditionals:   {:>8}               │",
            self.control_flow_map.conditional_branches
        )?;
        writeln!(
            f,
            "  │ Far jumps:      {:>8}               │",
            self.control_flow_map.far_jumps
        )?;
        writeln!(f, "  └────────────────────────────────────────┘")?;
        writeln!(f)?;

        // Structural Integrity — NUEVO
        writeln!(f, "  ┌─ Structural Integrity ──────────────────┐")?;
        if self.integrity.entry_point_checked {
            writeln!(
                f,
                "  │ Entry valid:    {:>8}               │",
                if self.integrity.entry_point_valid {
                    "yes ✓ "
                } else {
                    "NO ⚠ "
                }
            )?;
        }
        writeln!(
            f,
            "  │ Code/data:      {:>7.1}%               │",
            self.integrity.code_to_data_ratio * 100.0
        )?;
        writeln!(
            f,
            "  │ Overlapping:    {:>8}               │",
            if self.integrity.overlapping_sections {
                "YES ⚠ "
            } else {
                "no    "
            }
        )?;
        writeln!(
            f,
            "  │ Anomalous perms:{:>8}               │",
            self.integrity.anomalous_permissions
        )?;
        writeln!(f, "  └────────────────────────────────────────┘")?;
        writeln!(f)?;

        // Import/Export Map — NUEVO
        if self.import_export_map.import_count > 0 || self.import_export_map.export_count > 0 {
            writeln!(f, "  ┌─ Import/Export Map ─────────────────────┐")?;
            writeln!(
                f,
                "  │ Imports:        {:>8}               │",
                self.import_export_map.import_count
            )?;
            writeln!(
                f,
                "  │ Exports:        {:>8}               │",
                self.import_export_map.export_count
            )?;
            writeln!(
                f,
                "  │ Libraries:      {:>8}               │",
                self.import_export_map.imports_by_library.len()
            )?;
            if !self.import_export_map.memory_manipulation_apis.is_empty() {
                writeln!(
                    f,
                    "  │ Memory APIs:    {:>8}               │",
                    self.import_export_map.memory_manipulation_apis.len()
                )?;
            }
            if !self.import_export_map.process_injection_apis.is_empty() {
                writeln!(
                    f,
                    "  │ Injection APIs: {:>8} ⚠             │",
                    self.import_export_map.process_injection_apis.len()
                )?;
            }
            if !self.import_export_map.network_apis.is_empty() {
                writeln!(
                    f,
                    "  │ Network APIs:   {:>8}               │",
                    self.import_export_map.network_apis.len()
                )?;
            }
            writeln!(f, "  └────────────────────────────────────────┘")?;
            writeln!(f)?;
        }

        // Capabilities
        writeln!(f, "  ┌─ Capabilities ──────────────────────────┐")?;
        writeln!(
            f,
            "  │ Requires kernel:  {}                    │",
            if self.capabilities.requires_kernel() {
                "YES ⚠"
            } else {
                "no    "
            }
        )?;
        writeln!(
            f,
            "  │ Pure userspace:   {}                    │",
            if self.capabilities.is_pure_userspace() {
                "yes ✓ "
            } else {
                "NO    "
            }
        )?;
        writeln!(
            f,
            "  │ IO access:        {}                    │",
            if self.capabilities.io_port_access {
                "YES ⚠"
            } else {
                "no    "
            }
        )?;
        writeln!(
            f,
            "  │ CR access:        {}                    │",
            if self.capabilities.control_register_access {
                "YES ⚠"
            } else {
                "no    "
            }
        )?;
        writeln!(
            f,
            "  │ MSR access:       {}                    │",
            if self.capabilities.msr_access {
                "YES ⚠"
            } else {
                "no    "
            }
        )?;
        writeln!(
            f,
            "  │ INT control:      {}                    │",
            if self.capabilities.interrupt_control {
                "YES ⚠"
            } else {
                "no    "
            }
        )?;
        writeln!(
            f,
            "  │ Desc tables:      {}                    │",
            if self.capabilities.descriptor_table_access {
                "YES ⚠"
            } else {
                "no    "
            }
        )?;
        writeln!(
            f,
            "  │ Struct clean:     {}                    │",
            if self.integrity.is_clean() {
                "yes ✓ "
            } else {
                "NO ⚠ "
            }
        )?;
        writeln!(
            f,
            "  │ Import clean:     {}                    │",
            if self.import_export_map.is_clean() {
                "yes ✓ "
            } else {
                "NO ⚠ "
            }
        )?;
        writeln!(
            f,
            "  │ Active caps:      {:>3}                    │",
            self.capabilities.active_count()
        )?;
        writeln!(f, "  └────────────────────────────────────────┘")?;
        writeln!(f, "═══════════════════════════════════════════════")?;
        Ok(())
    }
}
