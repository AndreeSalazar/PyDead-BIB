// ============================================================
// C++ AST → IR Lowering
// ============================================================

use crate::middle::ir::Module;

/// Lower C++ to IR (entry point)
pub fn lower_cpp_to_ir(_source: &str) -> Module {
    Module::new("cpp_module")
}
