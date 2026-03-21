// ============================================================
// ADead-BIB Register Allocator — Basic Temporary Allocator
// ============================================================
// Reduces unnecessary push/pop operations by tracking available
// registers for temporary values during expression evaluation.
//
// Before: Every binary op does push/pop (2 bytes each)
// After:  Uses available registers, spills only when necessary
//
// Pipeline: emit_expression() → TempAllocator → fewer stack ops
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use super::Reg;

/// Registers available for temporary allocation
/// Excludes: RAX (accumulator), RSP (stack), RBP (frame)
const TEMP_REGS: [Reg; 13] = [
    Reg::RBX, // Callee-saved
    Reg::RCX, // Caller-saved (arg 4 Windows, arg 1 Linux)
    Reg::RDX, // Caller-saved (arg 3 Windows, arg 2 Linux)
    Reg::RSI, // Caller-saved (arg 2 Linux)
    Reg::RDI, // Caller-saved (arg 1 Linux)
    Reg::R8,  // Caller-saved
    Reg::R9,  // Caller-saved
    Reg::R10, // Caller-saved (scratch)
    Reg::R11, // Caller-saved (scratch)
    Reg::R12, // Callee-saved
    Reg::R13, // Callee-saved
    Reg::R14, // Callee-saved
    Reg::R15, // Callee-saved
];

/// Callee-saved registers that must be preserved across calls
const CALLEE_SAVED: [Reg; 5] = [Reg::RBX, Reg::R12, Reg::R13, Reg::R14, Reg::R15];

/// Basic register allocator for temporary values
#[derive(Debug, Clone)]
pub struct TempAllocator {
    /// Registers currently available for allocation
    available: Vec<Reg>,
    /// Registers currently in use
    in_use: Vec<Reg>,
    /// Number of spills to stack (for metrics)
    spill_count: usize,
    /// Maximum registers used simultaneously
    max_used: usize,
}

impl TempAllocator {
    /// Create a new allocator with all temp registers available
    pub fn new() -> Self {
        Self {
            available: TEMP_REGS.to_vec(),
            in_use: Vec::new(),
            spill_count: 0,
            max_used: 0,
        }
    }

    /// Allocate a temporary register
    /// Returns None if all registers are in use (caller should spill to stack)
    pub fn alloc(&mut self) -> Option<Reg> {
        if let Some(reg) = self.available.pop() {
            self.in_use.push(reg);
            self.max_used = self.max_used.max(self.in_use.len());
            Some(reg)
        } else {
            self.spill_count += 1;
            None // Signal to use push/pop
        }
    }

    /// Allocate a specific register if available
    pub fn alloc_specific(&mut self, reg: Reg) -> bool {
        if let Some(pos) = self.available.iter().position(|r| *r == reg) {
            self.available.remove(pos);
            self.in_use.push(reg);
            self.max_used = self.max_used.max(self.in_use.len());
            true
        } else {
            false
        }
    }

    /// Free a register, making it available again
    pub fn free(&mut self, reg: Reg) {
        if let Some(pos) = self.in_use.iter().position(|r| *r == reg) {
            self.in_use.remove(pos);
            self.available.push(reg);
        }
    }

    /// Free all registers (reset state)
    pub fn free_all(&mut self) {
        self.available.extend(self.in_use.drain(..));
    }

    /// Check if a register is currently in use
    pub fn is_in_use(&self, reg: Reg) -> bool {
        self.in_use.contains(&reg)
    }

    /// Check if a register is available
    pub fn is_available(&self, reg: Reg) -> bool {
        self.available.contains(&reg)
    }

    /// Get number of available registers
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Get number of registers in use
    pub fn in_use_count(&self) -> usize {
        self.in_use.len()
    }

    /// Get spill count (number of times we had to use stack)
    pub fn spill_count(&self) -> usize {
        self.spill_count
    }

    /// Get maximum registers used simultaneously
    pub fn max_used(&self) -> usize {
        self.max_used
    }

    /// Get list of callee-saved registers currently in use
    /// These need to be saved/restored in function prologue/epilogue
    pub fn callee_saved_in_use(&self) -> Vec<Reg> {
        self.in_use
            .iter()
            .filter(|r| CALLEE_SAVED.contains(r))
            .copied()
            .collect()
    }

    /// Reserve registers for function arguments (Windows x64)
    pub fn reserve_windows_args(&mut self, arg_count: usize) {
        let windows_args = [Reg::RCX, Reg::RDX, Reg::R8, Reg::R9];
        for i in 0..arg_count.min(4) {
            self.alloc_specific(windows_args[i]);
        }
    }

    /// Reserve registers for function arguments (Linux x64)
    pub fn reserve_linux_args(&mut self, arg_count: usize) {
        let linux_args = [Reg::RDI, Reg::RSI, Reg::RDX, Reg::RCX, Reg::R8, Reg::R9];
        for i in 0..arg_count.min(6) {
            self.alloc_specific(linux_args[i]);
        }
    }
}

impl Default for TempAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Stack frame calculator
/// Calculates actual stack space needed instead of fixed 128 bytes
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Local variables (name -> offset from RBP)
    locals: Vec<(String, i32)>,
    /// Current stack offset (grows negative from RBP)
    offset: i32,
    /// Alignment requirement (16 bytes for x64)
    alignment: i32,
}

impl StackFrame {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            offset: 0,
            alignment: 16,
        }
    }

    /// Allocate space for a local variable
    /// Returns the offset from RBP (negative)
    pub fn alloc_local(&mut self, name: String, size: i32) -> i32 {
        self.offset -= size;
        // Align to natural boundary
        let align = size.min(8);
        self.offset = (self.offset / align) * align;
        self.locals.push((name, self.offset));
        self.offset
    }

    /// Get offset for a local variable
    pub fn get_local(&self, name: &str) -> Option<i32> {
        self.locals
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, offset)| *offset)
    }

    /// Get total stack space needed (aligned to 16 bytes)
    pub fn total_size(&self) -> i32 {
        let raw_size = -self.offset;
        // Align to 16 bytes
        ((raw_size + self.alignment - 1) / self.alignment) * self.alignment
    }

    /// Get number of local variables
    pub fn local_count(&self) -> usize {
        self.locals.len()
    }
}

impl Default for StackFrame {
    fn default() -> Self {
        Self::new()
    }
}

/// Liveness interval for a variable
#[derive(Debug, Clone)]
pub struct LiveInterval {
    pub var_name: String,
    pub start: usize,
    pub end: usize,
    pub assigned_reg: Option<Reg>,
    pub spill_slot: Option<i32>,
}

/// Linear-scan register allocator with liveness analysis
#[derive(Debug)]
pub struct LinearScanAllocator {
    intervals: Vec<LiveInterval>,
    active: Vec<usize>,
    free_regs: Vec<Reg>,
    spill_offset: i32,
    max_spill_slots: usize,
}

impl LinearScanAllocator {
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
            active: Vec::new(),
            free_regs: TEMP_REGS.to_vec(),
            spill_offset: 0,
            max_spill_slots: 0,
        }
    }

    /// Add a liveness interval for a variable
    pub fn add_interval(&mut self, var_name: String, start: usize, end: usize) {
        self.intervals.push(LiveInterval {
            var_name,
            start,
            end,
            assigned_reg: None,
            spill_slot: None,
        });
    }

    /// Run linear-scan allocation
    pub fn allocate(&mut self) {
        // Sort intervals by start point
        self.intervals.sort_by_key(|i| i.start);

        for i in 0..self.intervals.len() {
            // Expire old intervals
            self.expire_old_intervals(self.intervals[i].start);

            if let Some(reg) = self.free_regs.pop() {
                self.intervals[i].assigned_reg = Some(reg);
                self.active.push(i);
                // Keep active sorted by end point
                self.active.sort_by_key(|&idx| self.intervals[idx].end);
            } else {
                // Spill: pick the interval that ends last
                self.spill_at_interval(i);
            }
        }
    }

    fn expire_old_intervals(&mut self, current_point: usize) {
        let mut to_remove = Vec::new();
        for (pos, &idx) in self.active.iter().enumerate() {
            if self.intervals[idx].end <= current_point {
                to_remove.push(pos);
                if let Some(reg) = self.intervals[idx].assigned_reg {
                    self.free_regs.push(reg);
                }
            }
        }
        for pos in to_remove.into_iter().rev() {
            self.active.remove(pos);
        }
    }

    fn spill_at_interval(&mut self, i: usize) {
        if let Some(&last_active) = self.active.last() {
            if self.intervals[last_active].end > self.intervals[i].end {
                // Spill the active interval that ends latest
                self.intervals[i].assigned_reg = self.intervals[last_active].assigned_reg;
                self.spill_offset -= 8;
                self.intervals[last_active].assigned_reg = None;
                self.intervals[last_active].spill_slot = Some(self.spill_offset);
                self.max_spill_slots += 1;
                self.active.pop();
                self.active.push(i);
                self.active.sort_by_key(|&idx| self.intervals[idx].end);
            } else {
                // Spill current interval
                self.spill_offset -= 8;
                self.intervals[i].spill_slot = Some(self.spill_offset);
                self.max_spill_slots += 1;
            }
        } else {
            self.spill_offset -= 8;
            self.intervals[i].spill_slot = Some(self.spill_offset);
            self.max_spill_slots += 1;
        }
    }

    /// Get the allocation result for a variable
    pub fn get_allocation(&self, var_name: &str) -> Option<&LiveInterval> {
        self.intervals.iter().find(|i| i.var_name == var_name)
    }

    /// Get all intervals
    pub fn intervals(&self) -> &[LiveInterval] {
        &self.intervals
    }

    /// Get number of spill slots used
    pub fn spill_slots_used(&self) -> usize {
        self.max_spill_slots
    }

    /// Get total extra stack space needed for spills (aligned to 16)
    pub fn spill_stack_size(&self) -> i32 {
        let raw = -self.spill_offset;
        ((raw + 15) / 16) * 16
    }
}

impl Default for LinearScanAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_allocator_basic() {
        let mut alloc = TempAllocator::new();

        // Should be able to allocate multiple registers
        let r1 = alloc.alloc();
        let r2 = alloc.alloc();
        let r3 = alloc.alloc();

        assert!(r1.is_some());
        assert!(r2.is_some());
        assert!(r3.is_some());
        assert_ne!(r1, r2);
        assert_ne!(r2, r3);

        // Free one and reallocate
        alloc.free(r2.unwrap());
        let r4 = alloc.alloc();
        assert_eq!(r4, r2); // Should get the same register back
    }

    #[test]
    fn test_temp_allocator_exhaustion() {
        let mut alloc = TempAllocator::new();

        // Allocate all registers
        for _ in 0..TEMP_REGS.len() {
            assert!(alloc.alloc().is_some());
        }

        // Next allocation should fail (spill)
        assert!(alloc.alloc().is_none());
        assert_eq!(alloc.spill_count(), 1);
    }

    #[test]
    fn test_stack_frame() {
        let mut frame = StackFrame::new();

        let off1 = frame.alloc_local("x".to_string(), 8);
        let off2 = frame.alloc_local("y".to_string(), 4);
        let off3 = frame.alloc_local("z".to_string(), 1);

        assert_eq!(off1, -8);
        assert!(off2 < off1);
        assert!(off3 < off2);

        assert_eq!(frame.get_local("x"), Some(-8));
        assert!(frame.total_size() >= 13); // At least 8+4+1
        assert_eq!(frame.total_size() % 16, 0); // Aligned to 16
    }

    #[test]
    fn test_linear_scan_basic() {
        let mut alloc = LinearScanAllocator::new();
        alloc.add_interval("x".to_string(), 0, 10);
        alloc.add_interval("y".to_string(), 2, 8);
        alloc.add_interval("z".to_string(), 5, 15);
        alloc.allocate();

        assert!(alloc.get_allocation("x").unwrap().assigned_reg.is_some());
        assert!(alloc.get_allocation("y").unwrap().assigned_reg.is_some());
        assert!(alloc.get_allocation("z").unwrap().assigned_reg.is_some());
        assert_eq!(alloc.spill_slots_used(), 0);
    }

    #[test]
    fn test_linear_scan_spill() {
        let mut alloc = LinearScanAllocator::new();
        // Create more intervals than available registers
        for i in 0..15 {
            alloc.add_interval(format!("v{}", i), 0, 20);
        }
        alloc.allocate();
        // 13 regs available, 15 intervals → 2 spills
        assert_eq!(alloc.spill_slots_used(), 2);
    }
}
