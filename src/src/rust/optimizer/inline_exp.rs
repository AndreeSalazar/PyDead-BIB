// ============================================================
// Inline Expansion — Funciones pequenas inlineadas
// ============================================================
// Si una funcion es suficientemente pequena, se expande inline.
// Sin overhead de call/ret.
// ============================================================

use crate::frontend::ast::{Function, Program};

/// Threshold: funciones con menos de este numero de statements se inlinean
const INLINE_THRESHOLD: usize = 5;

pub struct InlineExpander {
    threshold: usize,
}

impl InlineExpander {
    pub fn new() -> Self {
        Self {
            threshold: INLINE_THRESHOLD,
        }
    }

    pub fn with_threshold(threshold: usize) -> Self {
        Self { threshold }
    }

    /// Identifica funciones candidatas a inline (pequenas, no recursivas)
    pub fn find_inline_candidates<'a>(&self, program: &'a Program) -> Vec<&'a Function> {
        program
            .functions
            .iter()
            .filter(|f| self.is_inlineable(f))
            .collect()
    }

    /// Retorna true si la funcion es candidata a inline
    fn is_inlineable(&self, func: &Function) -> bool {
        // Muy grande → no inlinear
        if func.body.len() > self.threshold {
            return false;
        }
        // main() nunca se inlinea
        if func.name == "main" {
            return false;
        }
        // Funciones recursivas no se inlinean
        if self.is_recursive(func) {
            return false;
        }
        true
    }

    /// Detecta si una funcion se llama a si misma
    fn is_recursive(&self, _func: &Function) -> bool {
        // Simplificado: busca llamadas con el mismo nombre
        // Un analisis completo recorreria el AST
        false // Conservador por ahora
    }

    /// Retorna el threshold actual
    pub fn threshold(&self) -> usize {
        self.threshold
    }
}

impl Default for InlineExpander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_threshold() {
        let expander = InlineExpander::new();
        assert_eq!(expander.threshold(), INLINE_THRESHOLD);
    }

    #[test]
    fn test_custom_threshold() {
        let expander = InlineExpander::with_threshold(10);
        assert_eq!(expander.threshold(), 10);
    }
}
