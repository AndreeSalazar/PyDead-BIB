// ============================================================
// ADead-BIB Program Analysis
// ============================================================
// Analysis passes for IR optimization
// Inspired by LLVM Analysis passes
// ============================================================

mod cfg;
mod domtree;
mod loops;

pub use cfg::CFGAnalysis;
pub use domtree::DominatorTree;
pub use loops::LoopAnalysis;
