//! Frontend: Parsea código Python usando rustpython-parser y lo baja a nuestro IR.

use anyhow::Result;
use pyd_core::Instruction;
use rustpython_ast::Mod;
use rustpython_parser::parse;

pub fn parse_and_lower(source: &str) -> Result<Vec<Instruction>> {
    log::info!("📝 Parseando código Python...");
    
    // 1. Parsear a AST de RustPython
    let ast = parse(source, rustpython_parser::Mode::Module, "<pydead>")?;
    
    let mut instructions = Vec::new();

    // 2. Lowering (Traducción AST -> IR) - Simplificado para el ejemplo
    if let Mod::Module(module) = ast {
        for stmt in module.body {
            // Buscamos asignaciones simples para el ejemplo
            if let rustpython_ast::Stmt::Assign(rustpython_ast::StmtAssign { value, .. }) = stmt {
                // Usamos `..` para ignorar el campo `range` que exige el AST de RustPython
                if let rustpython_ast::Expr::BinOp(rustpython_ast::ExprBinOp { op, .. }) = value.as_ref() {
                    if let rustpython_ast::Operator::MatMult = op {
                        log::info!("🔍 Detectada operación de MatMul (@) en el AST.");
                        
                        // En un compilador real, aquí rastrearíamos los IDs de 'left' y 'right'
                        // Para este ejemplo, asumimos tensores en índices 0 y 1, resultado en 2.
                        instructions.push(Instruction::MatMul {
                            dest: 2,
                            lhs: 0,
                            rhs: 1,
                        });
                    }
                }
            }
        }
    }

    Ok(instructions)
}