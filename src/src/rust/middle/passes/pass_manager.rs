// ============================================================
// ADead-BIB Pass Manager
// ============================================================
// Manages and schedules optimization passes
// Inspired by LLVM PassManager
// ============================================================

use crate::middle::ir::{Function, Module};

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    /// No optimization (debug)
    O0,
    /// Basic optimizations
    O1,
    /// Standard optimizations
    O2,
    /// Aggressive optimizations
    O3,
    /// Optimize for size
    Os,
    /// Optimize for minimum size
    Oz,
}

impl Default for OptLevel {
    fn default() -> Self {
        OptLevel::O0
    }
}

/// Pass kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    /// Module-level pass
    Module,
    /// Function-level pass
    Function,
    /// Basic block-level pass
    BasicBlock,
    /// Analysis pass (doesn't modify IR)
    Analysis,
}

/// Pass trait - All optimization passes implement this
pub trait Pass: Send + Sync {
    /// Pass name
    fn name(&self) -> &'static str;

    /// Pass kind
    fn kind(&self) -> PassKind;

    /// Run pass on module
    fn run_on_module(&self, _module: &mut Module) -> bool {
        false
    }

    /// Run pass on function
    fn run_on_function(&self, _func: &mut Function) -> bool {
        false
    }
}

/// Pass Manager - Schedules and runs passes
pub struct PassManager {
    /// Registered passes
    passes: Vec<Box<dyn Pass>>,

    /// Optimization level
    opt_level: OptLevel,

    /// Enable debug output
    debug: bool,

    /// Statistics
    stats: PassStats,
}

/// Pass statistics
#[derive(Debug, Default)]
pub struct PassStats {
    pub passes_run: usize,
    pub functions_modified: usize,
    pub instructions_removed: usize,
    pub instructions_added: usize,
}

impl PassManager {
    pub fn new(opt_level: OptLevel) -> Self {
        PassManager {
            passes: Vec::new(),
            opt_level,
            debug: false,
            stats: PassStats::default(),
        }
    }

    /// Enable debug output
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Add a pass
    pub fn add_pass<P: Pass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    /// Add standard passes for optimization level
    pub fn add_standard_passes(&mut self) {
        use super::transform::*;

        match self.opt_level {
            OptLevel::O0 => {
                // No optimization passes
            }
            OptLevel::O1 => {
                // Basic optimizations
                self.add_pass(ConstantFoldPass);
                self.add_pass(DeadCodeElimPass);
                self.add_pass(SimplifyCFGPass);
            }
            OptLevel::O2 => {
                // Standard optimizations
                self.add_pass(ConstantFoldPass);
                self.add_pass(DeadCodeElimPass);
                self.add_pass(SimplifyCFGPass);
                self.add_pass(InlinePass::new(225)); // Inline threshold
                self.add_pass(GVNPass);
                self.add_pass(LICMPass);
            }
            OptLevel::O3 => {
                // Aggressive optimizations
                self.add_pass(ConstantFoldPass);
                self.add_pass(DeadCodeElimPass);
                self.add_pass(SimplifyCFGPass);
                self.add_pass(InlinePass::new(275)); // Higher threshold
                self.add_pass(GVNPass);
                self.add_pass(LICMPass);
                self.add_pass(LoopUnrollPass::new(4)); // Unroll factor
                self.add_pass(VectorizePass);
            }
            OptLevel::Os | OptLevel::Oz => {
                // Size optimizations
                self.add_pass(ConstantFoldPass);
                self.add_pass(DeadCodeElimPass);
                self.add_pass(SimplifyCFGPass);
                self.add_pass(MergeFunctionsPass);
            }
        }
    }

    /// Run all passes on module
    pub fn run(&mut self, module: &mut Module) -> bool {
        let mut changed = false;

        for pass in &self.passes {
            self.stats.passes_run += 1;

            if self.debug {
                println!("[PassManager] Running pass: {}", pass.name());
            }

            match pass.kind() {
                PassKind::Module | PassKind::Analysis => {
                    if pass.run_on_module(module) {
                        changed = true;
                    }
                }
                PassKind::Function | PassKind::BasicBlock => {
                    for func in module.iter_functions_mut() {
                        if pass.run_on_function(func) {
                            changed = true;
                            self.stats.functions_modified += 1;
                        }
                    }
                }
            }
        }

        changed
    }

    /// Get statistics
    pub fn stats(&self) -> &PassStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = PassStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyPass;

    impl Pass for DummyPass {
        fn name(&self) -> &'static str {
            "dummy"
        }
        fn kind(&self) -> PassKind {
            PassKind::Function
        }
        fn run_on_function(&self, _func: &mut Function) -> bool {
            false
        }
    }

    #[test]
    fn test_pass_manager_creation() {
        let pm = PassManager::new(OptLevel::O2);
        assert_eq!(pm.opt_level, OptLevel::O2);
    }

    #[test]
    fn test_add_pass() {
        let mut pm = PassManager::new(OptLevel::O0);
        pm.add_pass(DummyPass);
        assert_eq!(pm.passes.len(), 1);
    }
}
