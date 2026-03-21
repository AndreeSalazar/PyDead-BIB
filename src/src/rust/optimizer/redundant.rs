// ============================================================
// Redundant Operations Removal
// ============================================================

pub struct RedundantEliminator;

impl RedundantEliminator {
    pub fn new() -> Self {
        Self
    }

    /// Elimina instrucciones redundantes de un stream de bytes x86-64
    pub fn eliminate(&self, ops: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(ops.len());
        let mut i = 0;
        while i < ops.len() {
            // Pattern: mov rax, rax (redundant self-move)
            // REX.W MOV r/m64, r64: 48 89 C0 (mov rax, rax)
            if i + 3 <= ops.len() && ops[i] == 0x48 && ops[i + 1] == 0x89 && ops[i + 2] == 0xC0 {
                i += 3; // skip redundant mov rax, rax
                continue;
            }
            // Pattern: push rax; pop rax (cancels out)
            if i + 2 <= ops.len() && ops[i] == 0x50 && ops[i + 1] == 0x58 {
                i += 2; // skip push/pop pair
                continue;
            }
            result.push(ops[i]);
            i += 1;
        }
        result
    }

    /// Reports how many bytes were eliminated
    pub fn stats(&self, original: usize, optimized: usize) -> (usize, f64) {
        let saved = original.saturating_sub(optimized);
        let pct = if original > 0 {
            (saved as f64 / original as f64) * 100.0
        } else {
            0.0
        };
        (saved, pct)
    }
}

impl Default for RedundantEliminator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redundant_self_move() {
        let elim = RedundantEliminator::new();
        let ops = vec![0x48, 0x89, 0xC0]; // mov rax, rax
        let result = elim.eliminate(&ops);
        assert!(result.is_empty());
    }

    #[test]
    fn test_push_pop_cancel() {
        let elim = RedundantEliminator::new();
        let ops = vec![0x50, 0x58]; // push rax; pop rax
        let result = elim.eliminate(&ops);
        assert!(result.is_empty());
    }
}
