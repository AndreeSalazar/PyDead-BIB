// ============================================================
// ADead-BIB AST → IR Lowering
// ============================================================
// Converts frontend AST to middle-end IR
// ============================================================

mod c_lower;
mod cpp_lower;

pub use c_lower::lower_c_to_ir;
pub use cpp_lower::lower_cpp_to_ir;

use crate::frontend::ast::Program;
use crate::middle::ir::Module;

/// Lower a Program AST to IR Module
pub fn lower_to_ir(program: &Program, name: &str) -> Module {
    let mut module = Module::new(name);

    // Set target for Windows x64
    #[cfg(target_os = "windows")]
    module.set_target("x86_64-pc-windows-msvc");
    #[cfg(target_os = "linux")]
    module.set_target("x86_64-unknown-linux-gnu");

    // Lower functions
    for func in &program.functions {
        let ir_func = c_lower::lower_function(func);
        module.add_function(ir_func);
    }

    // Note: Program doesn't have globals field, they're in functions
    // Global lowering would be done differently

    module
}
