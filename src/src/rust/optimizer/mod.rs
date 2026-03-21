pub mod binary_optimizer;
pub mod branch_detector;
pub mod branchless;
pub mod const_fold;
pub mod const_prop;
pub mod dead_code;
pub mod inline_exp;
pub mod redundant;
pub mod simd;

pub use binary_optimizer::{BinaryOptimizer, OptLevel, OptimizationStats, PESizeOptimizer};
pub use const_fold::ConstFolder;
pub use const_prop::ConstPropagator;
pub use dead_code::DeadCodeEliminator;
pub use inline_exp::InlineExpander;
pub use redundant::RedundantEliminator;
