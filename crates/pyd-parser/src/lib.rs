//! Frontend: Extrae metadatos reales de tensores del AST de Python.

use anyhow::Result;
use pyd_core::{Instruction, TensorDType, TensorMeta};
use rustpython_ast::{Constant, Expr, Mod, Stmt};
use rustpython_parser::parse;

pub fn parse_and_lower(source: &str) -> Result<Vec<Instruction>> {
    log::info!("📝 Parseando código Python en busca de tensores...");
    
    let ast = parse(source, rustpython_parser::Mode::Module, "<pydead>")?;
    let mut instructions = Vec::new();

    if let Mod::Module(module) = ast {
        for stmt in module.body {
            if let Stmt::Assign(assign) = stmt {
                // Obtenemos el nombre de la variable (ej. "a", "b", "c")
                let var_name = if let Some(target) = assign.targets.first() {
                    if let Expr::Name(name) = target {
                        name.id.as_str().to_string()
                    } else { continue; }
                } else { continue; };

                // Analizamos el valor asignado
                match assign.value.as_ref() {
                    // Caso 1: Creación de Tensor: a = Tensor([2, 2])
                    Expr::Call(call) => {
                        if let Expr::Name(func_name) = call.func.as_ref() {
                            if func_name.id.as_str() == "Tensor" {
                                // Extraemos la forma (shape) del primer argumento
                                if let Some(Expr::List(list)) = call.args.first() {
                                    let shape: Vec<usize> = list.elts.iter()
                                        .filter_map(|e| {
                                            if let Expr::Constant(const_val) = e {
                                                // Extraemos el valor del AST de RustPython
                                                match &const_val.value {
                                                    Constant::Int(i) => i.to_string().parse().ok(),
                                                    Constant::Float(f) => Some(*f as usize),
                                                    _ => None,
                                                }
                                            } else { None }
                                        })
                                        .collect();
                                    
                                    let meta = TensorMeta {
                                        name: var_name.clone(),
                                        shape,
                                        dtype: TensorDType::Float32,
                                    };
                                    instructions.push(Instruction::CreateTensor { meta });
                                }
                            }
                        }
                    }
                    // Caso 2: Operación MatMul: c = a @ b
                    Expr::BinOp(binop) => {
                        if let rustpython_ast::Operator::MatMult = binop.op {
                            if let (Expr::Name(left), Expr::Name(right)) = (binop.left.as_ref(), binop.right.as_ref()) {
                                instructions.push(Instruction::MatMul {
                                    dest_name: var_name,
                                    lhs_name: left.id.as_str().to_string(),
                                    rhs_name: right.id.as_str().to_string(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(instructions)
}