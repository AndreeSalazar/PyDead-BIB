// ============================================================
// ADead-BIB v8.0 — Bit Resolver
// ============================================================
// Decide la anchura de bits del output según --target.
// No existe en ningún otro compilador del mundo.
//
// --target boot16    → 16-bit flat binary
// --target boot32    → 32-bit flat binary
// --target windows   → 64-bit PE estándar
// --target linux     → 64-bit ELF estándar
// --target fastos64  → 64-bit Po v1
// --target fastos128 → 128-bit Po v2 (XMM)
// --target fastos256 → 256-bit Po v8.0 (YMM) ★ ALIEN
//
// La entrada NUNCA cambia. La salida define TODO.
// Programador escribe C/C++ igual siempre — --target decide los bits.
//
// Autor: Eddi Andreé Salazar Matos — Lima, Perú
// ADead-BIB — Binary Is Binary — 16 → 256 bits
// ============================================================

use std::fmt;

// ============================================================
// BitTarget — Arquitectura de salida
// ============================================================

/// Target de bits que define la anchura del binario generado.
///
/// - `Bits16`: stage1 bootloader, flat binary 512 bytes
/// - `Bits32`: stage2 protected mode, transición
/// - `Bits64`: Windows PE / Linux ELF / FastOS Po v1
/// - `Bits128`: FastOS Po v2 — XMM registers — piso natural x86-64
/// - `Bits256`: FastOS Po v8.0 — YMM registers — SoA — alien 🛸
/// - `BitsAuto`: detecta CPU en runtime — gradual
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BitTarget {
    /// 16-bit flat binary (stage1 boot)
    Bits16,
    /// 32-bit protected mode (stage2 transition)
    Bits32,
    /// 64-bit standard (Windows PE / Linux ELF / FastOS Po v1)
    Bits64,
    /// 128-bit SSE (FastOS Po v2 — XMM registers)
    Bits128,
    /// 256-bit AVX2 (FastOS Po v8.0 — YMM registers — SoA)
    Bits256,
    /// Auto-detect CPU capabilities at runtime
    BitsAuto,
}

impl BitTarget {
    /// Parse from CLI --target string
    pub fn from_target_str(s: &str) -> Option<Self> {
        match s {
            "boot16" => Some(BitTarget::Bits16),
            "boot32" => Some(BitTarget::Bits32),
            "windows" | "win" | "pe" => Some(BitTarget::Bits64),
            "linux" | "elf" => Some(BitTarget::Bits64),
            "fastos" | "fastos64" | "po" => Some(BitTarget::Bits64),
            "fastos128" => Some(BitTarget::Bits128),
            "fastos256" => Some(BitTarget::Bits256),
            "auto" => Some(BitTarget::BitsAuto),
            "all" => Some(BitTarget::Bits64), // default for multi-target
            _ => None,
        }
    }

    /// Returns the register width in bits
    pub fn register_width(&self) -> u32 {
        match self {
            BitTarget::Bits16 => 16,
            BitTarget::Bits32 => 32,
            BitTarget::Bits64 => 64,
            BitTarget::Bits128 => 128,
            BitTarget::Bits256 => 256,
            BitTarget::BitsAuto => 256, // assume best case
        }
    }

    /// Returns true if this target uses YMM registers (256-bit)
    pub fn uses_ymm(&self) -> bool {
        matches!(self, BitTarget::Bits256 | BitTarget::BitsAuto)
    }

    /// Returns true if this target uses XMM registers (128-bit)
    pub fn uses_xmm(&self) -> bool {
        matches!(
            self,
            BitTarget::Bits128 | BitTarget::Bits256 | BitTarget::BitsAuto
        )
    }

    /// Returns true if this target can use VEX prefix instructions
    pub fn uses_vex(&self) -> bool {
        matches!(
            self,
            BitTarget::Bits128 | BitTarget::Bits256 | BitTarget::BitsAuto
        )
    }

    /// Returns the required data alignment in bytes
    pub fn data_alignment(&self) -> usize {
        match self {
            BitTarget::Bits16 => 2,
            BitTarget::Bits32 => 4,
            BitTarget::Bits64 => 8,
            BitTarget::Bits128 => 16,
            BitTarget::Bits256 => 32,
            BitTarget::BitsAuto => 32,
        }
    }

    /// Returns true if SoA optimization should be attempted
    pub fn should_soa_optimize(&self) -> bool {
        matches!(self, BitTarget::Bits256 | BitTarget::BitsAuto)
    }

    /// Returns the Po format version for this target
    pub fn po_version(&self) -> u8 {
        match self {
            BitTarget::Bits16 => 0x10,  // v1.0 boot
            BitTarget::Bits32 => 0x10,  // v1.0 boot
            BitTarget::Bits64 => 0x10,  // v1.0
            BitTarget::Bits128 => 0x20, // v2.0
            BitTarget::Bits256 => 0x80, // v8.0
            BitTarget::BitsAuto => 0x80,
        }
    }

    /// Returns the Po section names for code/data
    pub fn section_names(&self) -> (&'static str, &'static str) {
        match self {
            BitTarget::Bits256 | BitTarget::BitsAuto => (".text256", ".data256"),
            BitTarget::Bits128 => (".text128", ".data128"),
            _ => (".text", ".data"),
        }
    }
}

impl fmt::Display for BitTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitTarget::Bits16 => write!(f, "16-bit (boot)"),
            BitTarget::Bits32 => write!(f, "32-bit (protected)"),
            BitTarget::Bits64 => write!(f, "64-bit (standard)"),
            BitTarget::Bits128 => write!(f, "128-bit (XMM/SSE)"),
            BitTarget::Bits256 => write!(f, "256-bit (YMM/AVX2)"),
            BitTarget::BitsAuto => write!(f, "auto-detect"),
        }
    }
}

// ============================================================
// SoA Pattern — Detected array patterns for vectorization
// ============================================================

/// A detected Structure-of-Arrays pattern suitable for YMM vectorization.
///
/// When the compiler sees `float arr[8]` with `--target fastos256`,
/// it detects this as a SoA pattern and maps it to YMM registers.
#[derive(Debug, Clone)]
pub struct SoaPattern {
    /// Name of the array variable
    pub name: String,
    /// Element type (float, int, etc.)
    pub elem_type: SoaElementType,
    /// Number of elements (must be multiple of lane width)
    pub count: usize,
    /// Which YMM register is assigned (0-15)
    pub ymm_slot: Option<u8>,
    /// Byte offset in the .data256 section (32B aligned)
    pub data_offset: Option<u32>,
}

/// Element types that can be vectorized in 256-bit mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoaElementType {
    /// float (32-bit) — 8 per YMM register
    Float32,
    /// double (64-bit) — 4 per YMM register
    Float64,
    /// int32_t (32-bit) — 8 per YMM register
    Int32,
    /// int64_t (64-bit) — 4 per YMM register
    Int64,
    /// int16_t (16-bit) — 16 per YMM register
    Int16,
    /// int8_t (8-bit) — 32 per YMM register
    Int8,
}

impl SoaElementType {
    /// Returns the number of elements that fit in a YMM register (256 bits)
    pub fn lanes_per_ymm(&self) -> usize {
        match self {
            SoaElementType::Float32 => 8,
            SoaElementType::Float64 => 4,
            SoaElementType::Int32 => 8,
            SoaElementType::Int64 => 4,
            SoaElementType::Int16 => 16,
            SoaElementType::Int8 => 32,
        }
    }

    /// Returns the number of elements that fit in an XMM register (128 bits)
    pub fn lanes_per_xmm(&self) -> usize {
        match self {
            SoaElementType::Float32 => 4,
            SoaElementType::Float64 => 2,
            SoaElementType::Int32 => 4,
            SoaElementType::Int64 => 2,
            SoaElementType::Int16 => 8,
            SoaElementType::Int8 => 16,
        }
    }

    /// Returns element size in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            SoaElementType::Float32 => 4,
            SoaElementType::Float64 => 8,
            SoaElementType::Int32 => 4,
            SoaElementType::Int64 => 8,
            SoaElementType::Int16 => 2,
            SoaElementType::Int8 => 1,
        }
    }
}

// ============================================================
// BitResolverResult — Output of the resolver
// ============================================================

/// Result of BitResolver analysis — guides ISA compiler and output
#[derive(Debug, Clone)]
pub struct BitResolverResult {
    /// Target bit width
    pub target: BitTarget,
    /// Detected SoA patterns
    pub soa_patterns: Vec<SoaPattern>,
    /// YMM registers used (bitmask: bit 0 = YMM0, bit 15 = YMM15)
    pub ymm_used: u16,
    /// XMM registers used (bitmask: bit 0 = XMM0, bit 15 = XMM15)
    pub xmm_used: u16,
    /// Total data section size (aligned to target alignment)
    pub data256_size: u32,
    /// Whether VEX prefix encoding is required
    pub requires_vex: bool,
    /// BG stamp for Po header
    pub bg_stamp: u32,
}

impl BitResolverResult {
    /// Count of YMM registers used
    pub fn ymm_count(&self) -> u32 {
        self.ymm_used.count_ones()
    }

    /// Count of XMM registers used
    pub fn xmm_count(&self) -> u32 {
        self.xmm_used.count_ones()
    }
}

// ============================================================
// BitResolver — The Core v8.0 Component
// ============================================================

/// BitResolver: decides 16/64/128/256-bit output based on --target.
///
/// Unique to ADead-BIB. No other compiler in the world has this.
///
/// # Pipeline position
/// ```text
/// Register Allocator → BitResolver → ISA Compiler → BG Stamp → Output
/// ```
///
/// # What it does
/// 1. Analyzes IR for SoA patterns (arrays of float/int with known sizes)
/// 2. Assigns YMM/XMM registers to detected patterns
/// 3. Decides VEX prefix encoding requirements
/// 4. Enforces alignment (32B for 256-bit, 16B for 128-bit)
/// 5. Generates BG stamp hash for Po header
pub struct BitResolver {
    target: BitTarget,
    soa_patterns: Vec<SoaPattern>,
    ymm_next: u8,   // next available YMM register (0-15)
    xmm_next: u8,   // next available XMM register (0-15)
    ymm_used: u16,  // bitmask
    xmm_used: u16,  // bitmask
    data_offset: u32, // current offset in data section
}

impl BitResolver {
    /// Create a new BitResolver for the given target
    pub fn new(target: BitTarget) -> Self {
        Self {
            target,
            soa_patterns: Vec::new(),
            ymm_next: 0,
            xmm_next: 0,
            ymm_used: 0,
            xmm_used: 0,
            data_offset: 0,
        }
    }

    /// Detect and register a SoA pattern from an array declaration.
    ///
    /// Returns `Some(ymm_slot)` if the pattern was successfully mapped
    /// to a YMM register, `None` if not eligible or registers exhausted.
    ///
    /// # Example
    /// ```text
    /// float enemy_x[8];  // → detect_soa_pattern("enemy_x", Float32, 8)
    ///                     // → Some(0) — mapped to YMM0
    /// ```
    pub fn detect_soa_pattern(
        &mut self,
        name: &str,
        elem_type: SoaElementType,
        count: usize,
    ) -> Option<u8> {
        // Only 256-bit targets do SoA optimization
        if !self.target.should_soa_optimize() {
            return None;
        }

        let lanes = elem_type.lanes_per_ymm();

        // Array must be exactly one YMM register wide (or multiple)
        if count == 0 || count % lanes != 0 {
            return None;
        }

        // How many YMM registers needed?
        let ymm_count = count / lanes;

        // Check if we have enough YMM registers
        if (self.ymm_next as usize + ymm_count) > 16 {
            return None;
        }

        let first_ymm = self.ymm_next;

        // Allocate YMM registers and data section space
        for i in 0..ymm_count {
            let ymm_idx = self.ymm_next;
            self.ymm_used |= 1u16 << ymm_idx;
            self.ymm_next += 1;

            // Align data offset to 32 bytes
            let alignment = self.target.data_alignment() as u32;
            if self.data_offset % alignment != 0 {
                self.data_offset = (self.data_offset + alignment - 1) & !(alignment - 1);
            }

            let pattern = SoaPattern {
                name: if ymm_count == 1 {
                    name.to_string()
                } else {
                    format!("{}[{}]", name, i)
                },
                elem_type,
                count: lanes,
                ymm_slot: Some(ymm_idx),
                data_offset: Some(self.data_offset),
            };

            self.data_offset += 32; // 256 bits = 32 bytes
            self.soa_patterns.push(pattern);
        }

        Some(first_ymm)
    }

    /// Allocate an XMM register for 128-bit operations
    pub fn allocate_xmm(&mut self) -> Option<u8> {
        if !self.target.uses_xmm() {
            return None;
        }
        if self.xmm_next >= 16 {
            return None;
        }
        let idx = self.xmm_next;
        self.xmm_used |= 1u16 << idx;
        self.xmm_next += 1;
        Some(idx)
    }

    /// Compute a BG stamp hash for the given code bytes
    pub fn compute_bg_stamp(code: &[u8]) -> u32 {
        // FNV-1a 32-bit hash — same as cache/fastos.bib
        let mut hash: u32 = 0x811C9DC5;
        for &byte in code {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(0x01000193);
        }
        // Mix in Po magic for BG signature
        hash ^= 0x506F4F53; // 'PoOS'
        hash
    }

    /// Resolve the analysis into a final result
    pub fn resolve(&self) -> BitResolverResult {
        BitResolverResult {
            target: self.target,
            soa_patterns: self.soa_patterns.clone(),
            ymm_used: self.ymm_used,
            xmm_used: self.xmm_used,
            data256_size: self.data_offset,
            requires_vex: self.target.uses_vex() && self.ymm_used != 0,
            bg_stamp: 0, // computed later with actual code bytes
        }
    }

    /// Get the current target
    pub fn target(&self) -> BitTarget {
        self.target
    }

    /// Get detected SoA patterns
    pub fn soa_patterns(&self) -> &[SoaPattern] {
        &self.soa_patterns
    }

    /// Number of YMM registers allocated so far
    pub fn ymm_allocated(&self) -> u8 {
        self.ymm_next
    }
}

impl fmt::Display for BitResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BitResolver(target={}, ymm={}/{}, xmm={}/{}, soa={}, data={}B)",
            self.target,
            self.ymm_next,
            16,
            self.xmm_next,
            16,
            self.soa_patterns.len(),
            self.data_offset,
        )
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_target_from_str() {
        assert_eq!(
            BitTarget::from_target_str("fastos256"),
            Some(BitTarget::Bits256)
        );
        assert_eq!(
            BitTarget::from_target_str("windows"),
            Some(BitTarget::Bits64)
        );
        assert_eq!(
            BitTarget::from_target_str("boot16"),
            Some(BitTarget::Bits16)
        );
        assert_eq!(
            BitTarget::from_target_str("fastos128"),
            Some(BitTarget::Bits128)
        );
        assert_eq!(BitTarget::from_target_str("garbage"), None);
    }

    #[test]
    fn test_bit_target_properties() {
        assert_eq!(BitTarget::Bits256.register_width(), 256);
        assert!(BitTarget::Bits256.uses_ymm());
        assert!(BitTarget::Bits256.uses_xmm());
        assert!(BitTarget::Bits256.uses_vex());
        assert_eq!(BitTarget::Bits256.data_alignment(), 32);
        assert!(BitTarget::Bits256.should_soa_optimize());
        assert_eq!(BitTarget::Bits256.po_version(), 0x80);

        assert_eq!(BitTarget::Bits64.register_width(), 64);
        assert!(!BitTarget::Bits64.uses_ymm());
        assert!(!BitTarget::Bits64.should_soa_optimize());
    }

    #[test]
    fn test_soa_element_lanes() {
        assert_eq!(SoaElementType::Float32.lanes_per_ymm(), 8);
        assert_eq!(SoaElementType::Float64.lanes_per_ymm(), 4);
        assert_eq!(SoaElementType::Int32.lanes_per_ymm(), 8);
        assert_eq!(SoaElementType::Int8.lanes_per_ymm(), 32);
    }

    #[test]
    fn test_bit_resolver_soa_detection() {
        let mut resolver = BitResolver::new(BitTarget::Bits256);

        // float enemy_x[8] → should map to YMM0
        let result = resolver.detect_soa_pattern("enemy_x", SoaElementType::Float32, 8);
        assert_eq!(result, Some(0));

        // float enemy_y[8] → should map to YMM1
        let result = resolver.detect_soa_pattern("enemy_y", SoaElementType::Float32, 8);
        assert_eq!(result, Some(1));

        assert_eq!(resolver.ymm_allocated(), 2);
        assert_eq!(resolver.soa_patterns().len(), 2);
    }

    #[test]
    fn test_bit_resolver_soa_not_eligible_64bit() {
        let mut resolver = BitResolver::new(BitTarget::Bits64);

        // 64-bit target should NOT do SoA optimization
        let result = resolver.detect_soa_pattern("arr", SoaElementType::Float32, 8);
        assert_eq!(result, None);
    }

    #[test]
    fn test_bit_resolver_soa_wrong_count() {
        let mut resolver = BitResolver::new(BitTarget::Bits256);

        // float arr[7] — not a multiple of 8 — should fail
        let result = resolver.detect_soa_pattern("arr", SoaElementType::Float32, 7);
        assert_eq!(result, None);

        // float arr[0] — empty — should fail
        let result = resolver.detect_soa_pattern("arr", SoaElementType::Float32, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_bit_resolver_resolve() {
        let mut resolver = BitResolver::new(BitTarget::Bits256);
        resolver.detect_soa_pattern("pos_x", SoaElementType::Float32, 8);
        resolver.detect_soa_pattern("pos_y", SoaElementType::Float32, 8);

        let result = resolver.resolve();
        assert_eq!(result.target, BitTarget::Bits256);
        assert_eq!(result.ymm_count(), 2);
        assert_eq!(result.data256_size, 64); // 2 × 32 bytes
        assert!(result.requires_vex);
    }

    #[test]
    fn test_bg_stamp() {
        let code = b"hello world";
        let stamp = BitResolver::compute_bg_stamp(code);
        assert_ne!(stamp, 0);

        // Same input → same stamp
        let stamp2 = BitResolver::compute_bg_stamp(code);
        assert_eq!(stamp, stamp2);

        // Different input → different stamp
        let stamp3 = BitResolver::compute_bg_stamp(b"different");
        assert_ne!(stamp, stamp3);
    }

    #[test]
    fn test_section_names() {
        assert_eq!(
            BitTarget::Bits256.section_names(),
            (".text256", ".data256")
        );
        assert_eq!(
            BitTarget::Bits128.section_names(),
            (".text128", ".data128")
        );
        assert_eq!(BitTarget::Bits64.section_names(), (".text", ".data"));
    }
}
