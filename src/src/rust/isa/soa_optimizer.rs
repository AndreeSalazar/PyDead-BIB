// ============================================================
// ADead-BIB v8.0 — SoA Optimizer
// ============================================================
// Detecta patrones Structure-of-Arrays (SoA) en el IR y los
// marca para vectorización con YMM registers (256-bit).
//
// Patrón detectado:
//   float arr[8];     → 8 × float32 = 256 bits = 1 YMM register
//   int   data[8];    → 8 × int32   = 256 bits = 1 YMM register
//   double vals[4];   → 4 × float64 = 256 bits = 1 YMM register
//
// Operaciones vectorizables:
//   arr[i] += val[i]  → VADDPS  ymm0, ymm0, ymm1  (8 sumas en 1 ciclo)
//   arr[i] *= val[i]  → VMULPS  ymm0, ymm0, ymm1  (8 multiplicaciones)
//   dot(a, b)         → VFMADD231PS + horizontal    (FMA)
//
// Autor: Eddi Andreé Salazar Matos — Lima, Perú
// ADead-BIB — Binary Is Binary — SoA natural 256-bit
// ============================================================

use super::bit_resolver::{BitResolver, BitTarget, SoaElementType};

// ============================================================
// SoA Candidate — Detected before assignment
// ============================================================

/// A candidate array that might be vectorizable
#[derive(Debug, Clone)]
pub struct SoaCandidate {
    /// Variable name
    pub name: String,
    /// Element type
    pub elem_type: SoaElementType,
    /// Element count
    pub count: usize,
    /// Source line number (for diagnostics)
    pub line: u32,
    /// Whether it's accessed in a loop (makes it higher priority)
    pub in_loop: bool,
    /// Whether all accesses are sequential (required for SoA)
    pub sequential_access: bool,
}

/// Result of SoA analysis
#[derive(Debug, Clone)]
pub struct SoaAnalysis {
    /// Candidates that were successfully mapped to YMM registers
    pub vectorized: Vec<SoaVectorized>,
    /// Candidates that couldn't be vectorized (wrong size, etc.)
    pub skipped: Vec<SoaSkipped>,
    /// Total YMM registers used
    pub ymm_count: u8,
}

/// A successfully vectorized array
#[derive(Debug, Clone)]
pub struct SoaVectorized {
    pub name: String,
    pub elem_type: SoaElementType,
    pub count: usize,
    pub ymm_slot: u8,
    pub line: u32,
}

/// A candidate that was skipped
#[derive(Debug, Clone)]
pub struct SoaSkipped {
    pub name: String,
    pub reason: SoaSkipReason,
    pub line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoaSkipReason {
    /// Array size not a multiple of YMM lane count
    WrongCount,
    /// Target doesn't support 256-bit
    TargetTooNarrow,
    /// All YMM registers already allocated
    RegistersExhausted,
    /// Non-sequential access pattern
    NonSequential,
    /// Element type not vectorizable
    UnsupportedType,
}

impl std::fmt::Display for SoaSkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SoaSkipReason::WrongCount => write!(f, "count not multiple of lane width"),
            SoaSkipReason::TargetTooNarrow => write!(f, "target < 256-bit"),
            SoaSkipReason::RegistersExhausted => write!(f, "YMM registers exhausted"),
            SoaSkipReason::NonSequential => write!(f, "non-sequential access"),
            SoaSkipReason::UnsupportedType => write!(f, "unsupported element type"),
        }
    }
}

// ============================================================
// SoaOptimizer — Main analyzer
// ============================================================

/// Analyzes IR to find SoA patterns and maps them to YMM registers.
///
/// # Usage
/// ```text
/// let mut opt = SoaOptimizer::new(BitTarget::Bits256);
/// opt.add_candidate("enemy_x", SoaElementType::Float32, 8, 10, true, true);
/// opt.add_candidate("enemy_y", SoaElementType::Float32, 8, 11, true, true);
/// let analysis = opt.analyze();
/// // analysis.vectorized: [enemy_x→YMM0, enemy_y→YMM1]
/// ```
pub struct SoaOptimizer {
    target: BitTarget,
    candidates: Vec<SoaCandidate>,
}

impl SoaOptimizer {
    pub fn new(target: BitTarget) -> Self {
        Self {
            target,
            candidates: Vec::new(),
        }
    }

    /// Add a candidate array for SoA analysis
    pub fn add_candidate(
        &mut self,
        name: &str,
        elem_type: SoaElementType,
        count: usize,
        line: u32,
        in_loop: bool,
        sequential_access: bool,
    ) {
        self.candidates.push(SoaCandidate {
            name: name.to_string(),
            elem_type,
            count,
            line,
            in_loop,
            sequential_access,
        });
    }

    /// Detect SoA pattern from a C type declaration
    ///
    /// Returns the element type if the declaration is a vectorizable array.
    /// `type_name`: "float", "double", "int", etc.
    /// `array_size`: number of elements
    pub fn detect_from_type(type_name: &str, array_size: usize) -> Option<SoaElementType> {
        if array_size == 0 {
            return None;
        }

        let elem = match type_name {
            "float" => SoaElementType::Float32,
            "double" => SoaElementType::Float64,
            "int" | "int32_t" => SoaElementType::Int32,
            "long" | "long long" | "int64_t" => SoaElementType::Int64,
            "short" | "int16_t" => SoaElementType::Int16,
            "char" | "int8_t" | "uint8_t" => SoaElementType::Int8,
            _ => return None,
        };

        let lanes = elem.lanes_per_ymm();
        if array_size % lanes == 0 {
            Some(elem)
        } else {
            None
        }
    }

    /// Run the SoA analysis on all candidates
    pub fn analyze(&self) -> SoaAnalysis {
        let mut resolver = BitResolver::new(self.target);
        let mut vectorized = Vec::new();
        let mut skipped = Vec::new();

        // Sort candidates: in-loop arrays first (higher priority)
        let mut sorted: Vec<&SoaCandidate> = self.candidates.iter().collect();
        sorted.sort_by(|a, b| b.in_loop.cmp(&a.in_loop));

        for candidate in &sorted {
            // Check target supports 256-bit
            if !self.target.should_soa_optimize() {
                skipped.push(SoaSkipped {
                    name: candidate.name.clone(),
                    reason: SoaSkipReason::TargetTooNarrow,
                    line: candidate.line,
                });
                continue;
            }

            // Check sequential access
            if !candidate.sequential_access {
                skipped.push(SoaSkipped {
                    name: candidate.name.clone(),
                    reason: SoaSkipReason::NonSequential,
                    line: candidate.line,
                });
                continue;
            }

            // Try to allocate
            match resolver.detect_soa_pattern(
                &candidate.name,
                candidate.elem_type,
                candidate.count,
            ) {
                Some(ymm_slot) => {
                    vectorized.push(SoaVectorized {
                        name: candidate.name.clone(),
                        elem_type: candidate.elem_type,
                        count: candidate.count,
                        ymm_slot,
                        line: candidate.line,
                    });
                }
                None => {
                    let lanes = candidate.elem_type.lanes_per_ymm();
                    let reason = if candidate.count % lanes != 0 {
                        SoaSkipReason::WrongCount
                    } else {
                        SoaSkipReason::RegistersExhausted
                    };
                    skipped.push(SoaSkipped {
                        name: candidate.name.clone(),
                        reason,
                        line: candidate.line,
                    });
                }
            }
        }

        SoaAnalysis {
            ymm_count: resolver.ymm_allocated(),
            vectorized,
            skipped,
        }
    }
}

// ============================================================
// VectorOp — Operations that can be vectorized
// ============================================================

/// A vectorizable operation on SoA arrays
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorOp {
    /// VADDPS/VADDPD — element-wise addition
    Add,
    /// VSUBPS/VSUBPD — element-wise subtraction
    Sub,
    /// VMULPS/VMULPD — element-wise multiplication
    Mul,
    /// VDIVPS/VDIVPD — element-wise division
    Div,
    /// VFMADD231PS — fused multiply-add (a = a + b*c)
    Fma,
    /// VMOVAPS/VMOVAPD — aligned load/store
    Load,
    /// VMOVAPS/VMOVAPD — aligned store
    Store,
    /// VPCMPEQD — parallel comparison (int)
    CmpEq,
    /// VRSQRTPS — reciprocal square root (approximate)
    Rsqrt,
    /// VPTEST — parallel test (for BG)
    Test,
}

impl VectorOp {
    /// Returns the VEX-encoded opcode for this operation (float32 variant)
    pub fn vex_opcode_ps(&self) -> Option<u8> {
        match self {
            VectorOp::Add => Some(0x58),   // VADDPS
            VectorOp::Sub => Some(0x5C),   // VSUBPS
            VectorOp::Mul => Some(0x59),   // VMULPS
            VectorOp::Div => Some(0x5E),   // VDIVPS
            VectorOp::Load => Some(0x28),  // VMOVAPS (load)
            VectorOp::Store => Some(0x29), // VMOVAPS (store)
            VectorOp::Rsqrt => Some(0x52), // VRSQRTPS
            _ => None,
        }
    }

    /// Returns the instruction mnemonic
    pub fn mnemonic_ps(&self) -> &'static str {
        match self {
            VectorOp::Add => "vaddps",
            VectorOp::Sub => "vsubps",
            VectorOp::Mul => "vmulps",
            VectorOp::Div => "vdivps",
            VectorOp::Fma => "vfmadd231ps",
            VectorOp::Load => "vmovaps",
            VectorOp::Store => "vmovaps",
            VectorOp::CmpEq => "vpcmpeqd",
            VectorOp::Rsqrt => "vrsqrtps",
            VectorOp::Test => "vptest",
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_from_type() {
        assert_eq!(
            SoaOptimizer::detect_from_type("float", 8),
            Some(SoaElementType::Float32)
        );
        assert_eq!(
            SoaOptimizer::detect_from_type("double", 4),
            Some(SoaElementType::Float64)
        );
        assert_eq!(
            SoaOptimizer::detect_from_type("int", 8),
            Some(SoaElementType::Int32)
        );
        // Wrong count
        assert_eq!(SoaOptimizer::detect_from_type("float", 7), None);
        // Unknown type
        assert_eq!(SoaOptimizer::detect_from_type("mystruct", 8), None);
    }

    #[test]
    fn test_soa_analyzer_basic() {
        let mut opt = SoaOptimizer::new(BitTarget::Bits256);
        opt.add_candidate("pos_x", SoaElementType::Float32, 8, 10, true, true);
        opt.add_candidate("pos_y", SoaElementType::Float32, 8, 11, true, true);
        opt.add_candidate("bad", SoaElementType::Float32, 7, 12, true, true);

        let analysis = opt.analyze();
        assert_eq!(analysis.vectorized.len(), 2);
        assert_eq!(analysis.skipped.len(), 1);
        assert_eq!(analysis.ymm_count, 2);
        assert_eq!(analysis.vectorized[0].ymm_slot, 0);
        assert_eq!(analysis.vectorized[1].ymm_slot, 1);
    }

    #[test]
    fn test_soa_analyzer_64bit_target() {
        let mut opt = SoaOptimizer::new(BitTarget::Bits64);
        opt.add_candidate("arr", SoaElementType::Float32, 8, 10, true, true);

        let analysis = opt.analyze();
        assert_eq!(analysis.vectorized.len(), 0);
        assert_eq!(analysis.skipped.len(), 1);
        assert_eq!(analysis.skipped[0].reason, SoaSkipReason::TargetTooNarrow);
    }

    #[test]
    fn test_vector_op_mnemonics() {
        assert_eq!(VectorOp::Add.mnemonic_ps(), "vaddps");
        assert_eq!(VectorOp::Fma.mnemonic_ps(), "vfmadd231ps");
        assert_eq!(VectorOp::Load.vex_opcode_ps(), Some(0x28));
    }
}
