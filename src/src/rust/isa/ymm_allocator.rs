// ============================================================
// ADead-BIB v8.0 — YMM Register Allocator
// ============================================================
// Asigna registros YMM0-YMM15 (256-bit) y XMM0-XMM15 (128-bit)
// para operaciones vectoriales detectadas por el SoA Optimizer.
//
// YMM registers (AVX2):
//   YMM0-YMM7:   caller-saved (scratch — libres para uso)
//   YMM8-YMM15:  caller-saved también (no hay callee-saved YMM en SysV)
//   Windows x64:  YMM6-YMM15 son callee-saved (parcialmente)
//
// FastOS no sigue ninguna ABI externa — usa todos los YMM libremente.
// Para Windows/Linux targets: respeta las convenciones de llamada.
//
// Autor: Eddi Andreé Salazar Matos — Lima, Perú
// ADead-BIB — Binary Is Binary — YMM0-YMM15 nativos
// ============================================================

use std::fmt;

// ============================================================
// YMM Register IDs
// ============================================================

/// YMM register identifier (0-15)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct YmmReg(pub u8);

impl YmmReg {
    /// Create a new YMM register (0-15)
    pub fn new(idx: u8) -> Option<Self> {
        if idx < 16 {
            Some(YmmReg(idx))
        } else {
            None
        }
    }

    /// Register index (0-15)
    pub fn index(&self) -> u8 {
        self.0
    }

    /// Returns the corresponding XMM register (lower 128 bits)
    pub fn xmm_half(&self) -> XmmReg {
        XmmReg(self.0)
    }

    /// Whether this register requires VEX.R prefix (index >= 8)
    pub fn needs_rex_r(&self) -> bool {
        self.0 >= 8
    }

    /// ModR/M register encoding (lower 3 bits)
    pub fn modrm_reg(&self) -> u8 {
        self.0 & 0x07
    }
}

impl fmt::Display for YmmReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ymm{}", self.0)
    }
}

/// XMM register identifier (0-15)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XmmReg(pub u8);

impl XmmReg {
    pub fn new(idx: u8) -> Option<Self> {
        if idx < 16 {
            Some(XmmReg(idx))
        } else {
            None
        }
    }

    pub fn index(&self) -> u8 {
        self.0
    }

    pub fn needs_rex_r(&self) -> bool {
        self.0 >= 8
    }

    pub fn modrm_reg(&self) -> u8 {
        self.0 & 0x07
    }
}

impl fmt::Display for XmmReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "xmm{}", self.0)
    }
}

// ============================================================
// Allocation State
// ============================================================

/// State of a YMM register slot
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YmmState {
    /// Register is free
    Free,
    /// Allocated to a SoA array
    AllocatedSoA { name: String },
    /// Allocated as a temporary for arithmetic
    AllocatedTemp { purpose: String },
    /// Reserved for BG (Binary Guardian) checks
    ReservedBG,
}

// ============================================================
// YmmAllocator — The allocator
// ============================================================

/// Allocator for YMM0-YMM15 registers.
///
/// Tracks which YMM registers are in use, what they're used for,
/// and provides allocation/deallocation.
///
/// # Allocation Strategy
/// 1. SoA arrays get lowest-numbered YMM registers (YMM0, YMM1, ...)
/// 2. Temporaries get next available
/// 3. YMM15 is reserved for BG checks (if BG is active)
/// 4. Spill = store to stack-aligned 32B temp, reload later
pub struct YmmAllocator {
    /// State of each YMM register (0-15)
    states: [YmmState; 16],
    /// Whether BG reserves YMM15
    bg_reserved: bool,
    /// High water mark — max YMM registers ever simultaneously in use
    high_water: u8,
    /// Current count of allocated registers
    allocated_count: u8,
}

impl YmmAllocator {
    /// Create a new allocator
    pub fn new(reserve_bg: bool) -> Self {
        let mut states = [
            YmmState::Free, YmmState::Free, YmmState::Free, YmmState::Free,
            YmmState::Free, YmmState::Free, YmmState::Free, YmmState::Free,
            YmmState::Free, YmmState::Free, YmmState::Free, YmmState::Free,
            YmmState::Free, YmmState::Free, YmmState::Free, YmmState::Free,
        ];

        if reserve_bg {
            states[15] = YmmState::ReservedBG;
        }

        Self {
            states,
            bg_reserved: reserve_bg,
            high_water: 0,
            allocated_count: 0,
        }
    }

    /// Allocate a YMM register for a SoA array
    pub fn alloc_soa(&mut self, name: &str) -> Option<YmmReg> {
        for i in 0..16u8 {
            if self.states[i as usize] == YmmState::Free {
                self.states[i as usize] = YmmState::AllocatedSoA {
                    name: name.to_string(),
                };
                self.allocated_count += 1;
                if self.allocated_count > self.high_water {
                    self.high_water = self.allocated_count;
                }
                return Some(YmmReg(i));
            }
        }
        None
    }

    /// Allocate a YMM register for a temporary computation
    pub fn alloc_temp(&mut self, purpose: &str) -> Option<YmmReg> {
        // Search from high to low (temps get higher registers)
        let limit = if self.bg_reserved { 15 } else { 16 };
        for i in (0..limit).rev() {
            if self.states[i as usize] == YmmState::Free {
                self.states[i as usize] = YmmState::AllocatedTemp {
                    purpose: purpose.to_string(),
                };
                self.allocated_count += 1;
                if self.allocated_count > self.high_water {
                    self.high_water = self.allocated_count;
                }
                return Some(YmmReg(i as u8));
            }
        }
        None
    }

    /// Free a YMM register
    pub fn free(&mut self, reg: YmmReg) {
        let idx = reg.index() as usize;
        if self.states[idx] != YmmState::ReservedBG {
            self.states[idx] = YmmState::Free;
            if self.allocated_count > 0 {
                self.allocated_count -= 1;
            }
        }
    }

    /// Get the state of a YMM register
    pub fn state(&self, reg: YmmReg) -> &YmmState {
        &self.states[reg.index() as usize]
    }

    /// Returns a bitmask of all allocated YMM registers
    pub fn used_mask(&self) -> u16 {
        let mut mask = 0u16;
        for i in 0..16u8 {
            if self.states[i as usize] != YmmState::Free {
                mask |= 1u16 << i;
            }
        }
        mask
    }

    /// Number of currently allocated registers
    pub fn count(&self) -> u8 {
        self.allocated_count
    }

    /// Number of free registers
    pub fn free_count(&self) -> u8 {
        let total = if self.bg_reserved { 15u8 } else { 16u8 };
        total.saturating_sub(self.allocated_count)
    }

    /// High water mark
    pub fn high_water_mark(&self) -> u8 {
        self.high_water
    }

    /// Get the BG register (YMM15) if reserved
    pub fn bg_register(&self) -> Option<YmmReg> {
        if self.bg_reserved {
            Some(YmmReg(15))
        } else {
            None
        }
    }

    /// Generate VZEROUPPER instruction data if any YMM was used.
    /// Required before calling into non-AVX code (Windows ABI, library calls).
    pub fn needs_vzeroupper(&self) -> bool {
        self.high_water > 0
    }
}

impl fmt::Display for YmmAllocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "YmmAlloc(used={}, free={}, hwm={}, bg={})",
            self.allocated_count,
            self.free_count(),
            self.high_water,
            self.bg_reserved,
        )
    }
}

// ============================================================
// Spill Slot — for when we run out of YMM registers
// ============================================================

/// A stack spill slot for a YMM register (32 bytes, 32B aligned)
#[derive(Debug, Clone)]
pub struct YmmSpillSlot {
    /// Stack offset (negative from RBP, 32B aligned)
    pub stack_offset: i32,
    /// Which YMM register was spilled
    pub reg: YmmReg,
    /// What was in the register
    pub purpose: String,
}

/// Manages YMM spill slots on the stack
pub struct YmmSpillManager {
    slots: Vec<YmmSpillSlot>,
    next_offset: i32,
}

impl YmmSpillManager {
    pub fn new(initial_stack_offset: i32) -> Self {
        // Align to 32 bytes
        let aligned = (initial_stack_offset - 31) & !31;
        Self {
            slots: Vec::new(),
            next_offset: aligned,
        }
    }

    /// Spill a YMM register to stack. Returns the stack offset.
    pub fn spill(&mut self, reg: YmmReg, purpose: &str) -> i32 {
        self.next_offset -= 32; // 256 bits = 32 bytes
        let offset = self.next_offset;
        self.slots.push(YmmSpillSlot {
            stack_offset: offset,
            reg,
            purpose: purpose.to_string(),
        });
        offset
    }

    /// Get all spill slots
    pub fn slots(&self) -> &[YmmSpillSlot] {
        &self.slots
    }

    /// Total stack space needed for spills (in bytes)
    pub fn total_stack_bytes(&self) -> u32 {
        (self.slots.len() as u32) * 32
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ymm_reg() {
        let r = YmmReg::new(0).unwrap();
        assert_eq!(r.index(), 0);
        assert!(!r.needs_rex_r());
        assert_eq!(format!("{}", r), "ymm0");

        let r8 = YmmReg::new(8).unwrap();
        assert!(r8.needs_rex_r());
        assert_eq!(r8.modrm_reg(), 0); // 8 & 7 = 0

        assert!(YmmReg::new(16).is_none());
    }

    #[test]
    fn test_ymm_allocator_basic() {
        let mut alloc = YmmAllocator::new(false);
        assert_eq!(alloc.free_count(), 16);

        let r0 = alloc.alloc_soa("pos_x").unwrap();
        assert_eq!(r0.index(), 0);
        assert_eq!(alloc.count(), 1);

        let r1 = alloc.alloc_soa("pos_y").unwrap();
        assert_eq!(r1.index(), 1);
        assert_eq!(alloc.count(), 2);

        alloc.free(r0);
        assert_eq!(alloc.count(), 1);
        assert_eq!(alloc.high_water_mark(), 2);
    }

    #[test]
    fn test_ymm_allocator_bg_reserve() {
        let mut alloc = YmmAllocator::new(true);
        assert_eq!(alloc.free_count(), 15); // YMM15 reserved

        let bg = alloc.bg_register();
        assert!(bg.is_some());
        assert_eq!(bg.unwrap().index(), 15);

        // Allocate 15 SoA registers — should fill 0-14
        for i in 0..15u8 {
            let r = alloc.alloc_soa(&format!("arr_{}", i));
            assert!(r.is_some(), "Failed to allocate YMM{}", i);
        }

        // 16th should fail (YMM15 reserved for BG)
        let r = alloc.alloc_soa("overflow");
        assert!(r.is_none());
    }

    #[test]
    fn test_ymm_allocator_used_mask() {
        let mut alloc = YmmAllocator::new(true);
        alloc.alloc_soa("a");
        alloc.alloc_soa("b");

        let mask = alloc.used_mask();
        // YMM0, YMM1 allocated + YMM15 reserved for BG
        assert_eq!(mask & 0x03, 0x03); // bits 0,1
        assert_eq!(mask & 0x8000, 0x8000); // bit 15 (BG)
    }

    #[test]
    fn test_ymm_allocator_temp() {
        let mut alloc = YmmAllocator::new(true);
        // Temps should allocate from high end (before BG)
        let t = alloc.alloc_temp("mul_result").unwrap();
        assert_eq!(t.index(), 14); // highest free before YMM15

        let t2 = alloc.alloc_temp("add_result").unwrap();
        assert_eq!(t2.index(), 13);
    }

    #[test]
    fn test_ymm_spill_manager() {
        let mut spill = YmmSpillManager::new(-64);
        let off1 = spill.spill(YmmReg(0), "pos_x");
        let off2 = spill.spill(YmmReg(1), "pos_y");

        assert!(off1 < -64); // further negative
        assert_eq!(off2, off1 - 32); // 32 bytes apart
        assert_eq!(spill.total_stack_bytes(), 64);
    }

    #[test]
    fn test_vzeroupper_needed() {
        let mut alloc = YmmAllocator::new(false);
        assert!(!alloc.needs_vzeroupper());

        alloc.alloc_soa("data");
        assert!(alloc.needs_vzeroupper());

        alloc.free(YmmReg(0));
        // high water mark remembers
        assert!(alloc.needs_vzeroupper());
    }
}
