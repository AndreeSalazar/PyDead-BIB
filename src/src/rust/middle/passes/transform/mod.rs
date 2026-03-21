// ============================================================
// ADead-BIB Transformation Passes
// ============================================================
// Optimization passes that transform IR
// Inspired by LLVM transform passes
// ============================================================

mod constfold;
mod dce;
mod gvn;
mod inline;
mod licm;
mod merge_functions;
mod simplify_cfg;
mod unroll;
mod vectorize;

pub use constfold::ConstantFoldPass;
pub use dce::DeadCodeElimPass;
pub use gvn::GVNPass;
pub use inline::InlinePass;
pub use licm::LICMPass;
pub use merge_functions::MergeFunctionsPass;
pub use simplify_cfg::SimplifyCFGPass;
pub use unroll::LoopUnrollPass;
pub use vectorize::VectorizePass;
