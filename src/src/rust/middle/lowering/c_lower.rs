// ============================================================
// C AST → IR Lowering
// ============================================================
// Converts C AST to typed SSA IR
// Inspired by LLVM's Clang CodeGen
// ============================================================

#![allow(dead_code)]

use crate::frontend::ast::{BinOp, CmpOp, Expr, Function as AstFunction, Stmt, UnaryOp};
use crate::frontend::types::Type as AstType;
use crate::middle::ir::basicblock::BasicBlockId;
use crate::middle::ir::{
    BinaryOp, CastOp, CompareOp, Constant, Function, GlobalVariable, Instruction, Module,
    Type as IRType, Value, ValueId,
};
use std::collections::HashMap;

/// C to IR Lowering Context
pub struct CLowering<'a> {
    /// Current module being built
    module: &'a mut Module,
    /// Current function being lowered
    current_func: Option<usize>,
    /// Current basic block
    current_block: Option<BasicBlockId>,
    /// Variable name to ValueId mapping
    variables: HashMap<String, ValueId>,
    /// Next value ID
    next_value: u32,
    /// Next block ID
    next_block: u32,
    /// Loop context for break/continue
    loop_stack: Vec<LoopContext>,
    /// Label definitions for goto
    labels: HashMap<String, BasicBlockId>,
}

/// Loop context for break/continue
#[derive(Clone)]
struct LoopContext {
    continue_block: BasicBlockId,
    break_block: BasicBlockId,
}

impl<'a> CLowering<'a> {
    pub fn new(module: &'a mut Module) -> Self {
        CLowering {
            module,
            current_func: None,
            current_block: None,
            variables: HashMap::new(),
            next_value: 0,
            next_block: 0,
            loop_stack: Vec::new(),
            labels: HashMap::new(),
        }
    }

    /// Allocate a new value ID
    fn new_value_id(&mut self) -> ValueId {
        let id = ValueId(self.next_value);
        self.next_value += 1;
        id
    }

    /// Create a new basic block
    fn new_block(&mut self, name: &str) -> BasicBlockId {
        let id = BasicBlockId(self.next_block);
        self.next_block += 1;

        if let Some(func_idx) = self.current_func {
            self.module.functions[func_idx].create_block(Some(name));
        }
        id
    }

    /// Get current block mutable
    fn current_block_mut(&mut self) -> Option<&mut crate::middle::ir::BasicBlock> {
        let func_idx = self.current_func?;
        let block_id = self.current_block?;
        self.module.functions[func_idx]
            .blocks
            .iter_mut()
            .find(|b| b.id == block_id)
    }

    /// Emit an instruction to current block
    fn emit(&mut self, inst: Instruction) {
        if let Some(block) = self.current_block_mut() {
            block.push(inst);
        }
    }

    /// Lower a complete function
    pub fn lower_function(&mut self, func: &AstFunction) {
        let return_type = lower_type(&func.resolved_return_type);
        let mut ir_func = Function::new(&func.name, return_type.clone());

        // Add parameters
        for param in &func.params {
            let param_type = lower_type(&param.param_type);
            ir_func.add_param(&param.name, param_type);
        }

        // Add function to module
        let func_idx = self.module.add_function(ir_func);
        self.current_func = Some(func_idx);

        // Reset state
        self.variables.clear();
        self.next_value = 0;
        self.next_block = 0;
        self.loop_stack.clear();
        self.labels.clear();

        // Create entry block
        let entry = self.new_block("entry");
        self.current_block = Some(entry);

        // Allocate parameters as local variables
        for (i, param) in func.params.iter().enumerate() {
            let param_type = lower_type(&param.param_type);
            let alloca_id = self.new_value_id();
            self.emit(Instruction::alloca(param_type.clone(), alloca_id));

            // Store parameter value
            // Store parameter value (argument index)
            let param_val = Value::Constant(Constant::i64(i as i64));
            self.emit(Instruction::store(param_val, Value::Instruction(alloca_id)));

            self.variables.insert(param.name.clone(), alloca_id);
        }

        // Lower function body
        for stmt in &func.body {
            self.lower_stmt(stmt);
        }

        // Add implicit return if needed
        if return_type == IRType::Void {
            self.emit(Instruction::ret(None));
        }
    }

    /// Lower a statement
    fn lower_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                var_type,
                name,
                value,
            } => {
                let ty = lower_type(var_type);
                let alloca_id = self.new_value_id();
                self.emit(Instruction::alloca(ty.clone(), alloca_id));
                self.variables.insert(name.clone(), alloca_id);

                if let Some(init) = value {
                    let val = self.lower_expr(init, &ty);
                    self.emit(Instruction::store(val, Value::Instruction(alloca_id)));
                }
            }

            Stmt::Assign { name, value } => {
                if let Some(&var_id) = self.variables.get(name) {
                    let val = self.lower_expr(value, &IRType::I64);
                    self.emit(Instruction::store(val, Value::Instruction(var_id)));
                }
            }

            Stmt::IndexAssign {
                object,
                index,
                value,
            } => {
                let ptr = self.lower_expr(object, &IRType::ptr(IRType::I64));
                let idx = self.lower_expr(index, &IRType::I64);
                let val = self.lower_expr(value, &IRType::I64);

                let gep_id = self.new_value_id();
                self.emit(Instruction::gep(IRType::I64, ptr, vec![idx], gep_id));
                self.emit(Instruction::store(val, Value::Instruction(gep_id)));
            }

            Stmt::Return(expr) => {
                let val = expr.as_ref().map(|e| self.lower_expr(e, &IRType::I64));
                self.emit(Instruction::ret(val));
            }

            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                let cond = self.lower_expr(condition, &IRType::Bool);

                let then_bb = self.new_block("if.then");
                let else_bb = self.new_block("if.else");
                let merge_bb = self.new_block("if.merge");

                self.emit(Instruction::cond_br(cond, then_bb.0, else_bb.0));

                // Then block
                self.current_block = Some(then_bb);
                for s in then_body {
                    self.lower_stmt(s);
                }
                self.emit(Instruction::br(merge_bb.0));

                // Else block
                self.current_block = Some(else_bb);
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.lower_stmt(s);
                    }
                }
                self.emit(Instruction::br(merge_bb.0));

                self.current_block = Some(merge_bb);
            }

            Stmt::While { condition, body } => {
                let cond_bb = self.new_block("while.cond");
                let body_bb = self.new_block("while.body");
                let exit_bb = self.new_block("while.exit");

                self.emit(Instruction::br(cond_bb.0));

                // Condition block
                self.current_block = Some(cond_bb);
                let cond = self.lower_expr(condition, &IRType::Bool);
                self.emit(Instruction::cond_br(cond, body_bb.0, exit_bb.0));

                // Body block
                self.loop_stack.push(LoopContext {
                    continue_block: cond_bb,
                    break_block: exit_bb,
                });

                self.current_block = Some(body_bb);
                for s in body {
                    self.lower_stmt(s);
                }
                self.emit(Instruction::br(cond_bb.0));

                self.loop_stack.pop();
                self.current_block = Some(exit_bb);
            }

            Stmt::For {
                var,
                start,
                end,
                body,
            } => {
                // Allocate loop variable
                let alloca_id = self.new_value_id();
                self.emit(Instruction::alloca(IRType::I64, alloca_id));
                self.variables.insert(var.clone(), alloca_id);

                // Initialize with start value
                let start_val = self.lower_expr(start, &IRType::I64);
                self.emit(Instruction::store(start_val, Value::Instruction(alloca_id)));

                let cond_bb = self.new_block("for.cond");
                let body_bb = self.new_block("for.body");
                let update_bb = self.new_block("for.update");
                let exit_bb = self.new_block("for.exit");

                self.emit(Instruction::br(cond_bb.0));

                // Condition: var < end
                self.current_block = Some(cond_bb);
                let load_id = self.new_value_id();
                self.emit(Instruction::load(
                    IRType::I64,
                    Value::Instruction(alloca_id),
                    load_id,
                ));
                let end_val = self.lower_expr(end, &IRType::I64);
                let cmp_id = self.new_value_id();
                self.emit(Instruction::icmp(
                    CompareOp::Slt,
                    Value::Instruction(load_id),
                    end_val,
                    cmp_id,
                ));
                self.emit(Instruction::cond_br(
                    Value::Instruction(cmp_id),
                    body_bb.0,
                    exit_bb.0,
                ));

                // Body
                self.loop_stack.push(LoopContext {
                    continue_block: update_bb,
                    break_block: exit_bb,
                });

                self.current_block = Some(body_bb);
                for s in body {
                    self.lower_stmt(s);
                }
                self.emit(Instruction::br(update_bb.0));

                // Update: var++
                self.current_block = Some(update_bb);
                let load2_id = self.new_value_id();
                self.emit(Instruction::load(
                    IRType::I64,
                    Value::Instruction(alloca_id),
                    load2_id,
                ));
                let one = Value::Constant(Constant::i64(1));
                let inc_id = self.new_value_id();
                self.emit(Instruction::binary(
                    BinaryOp::Add,
                    IRType::I64,
                    Value::Instruction(load2_id),
                    one,
                    inc_id,
                ));
                self.emit(Instruction::store(
                    Value::Instruction(inc_id),
                    Value::Instruction(alloca_id),
                ));
                self.emit(Instruction::br(cond_bb.0));

                self.loop_stack.pop();
                self.current_block = Some(exit_bb);
            }

            Stmt::DoWhile { body, condition } => {
                let body_bb = self.new_block("dowhile.body");
                let cond_bb = self.new_block("dowhile.cond");
                let exit_bb = self.new_block("dowhile.exit");

                self.emit(Instruction::br(body_bb.0));

                self.loop_stack.push(LoopContext {
                    continue_block: cond_bb,
                    break_block: exit_bb,
                });

                // Body
                self.current_block = Some(body_bb);
                for s in body {
                    self.lower_stmt(s);
                }
                self.emit(Instruction::br(cond_bb.0));

                // Condition
                self.current_block = Some(cond_bb);
                let cond = self.lower_expr(condition, &IRType::Bool);
                self.emit(Instruction::cond_br(cond, body_bb.0, exit_bb.0));

                self.loop_stack.pop();
                self.current_block = Some(exit_bb);
            }

            Stmt::Break => {
                if let Some(ctx) = self.loop_stack.last() {
                    let exit = ctx.break_block;
                    self.emit(Instruction::br(exit.0));
                }
            }

            Stmt::Continue => {
                if let Some(ctx) = self.loop_stack.last() {
                    let cont = ctx.continue_block;
                    self.emit(Instruction::br(cont.0));
                }
            }

            Stmt::Expr(expr) => {
                self.lower_expr(expr, &IRType::I64);
            }

            Stmt::CompoundAssign { name, op, value } => {
                if let Some(&var_id) = self.variables.get(name) {
                    let load_id = self.new_value_id();
                    self.emit(Instruction::load(
                        IRType::I64,
                        Value::Instruction(var_id),
                        load_id,
                    ));

                    let rhs = self.lower_expr(value, &IRType::I64);
                    let bin_op = compound_to_binary(op);
                    let result_id = self.new_value_id();
                    self.emit(Instruction::binary(
                        bin_op,
                        IRType::I64,
                        Value::Instruction(load_id),
                        rhs,
                        result_id,
                    ));

                    self.emit(Instruction::store(
                        Value::Instruction(result_id),
                        Value::Instruction(var_id),
                    ));
                }
            }

            Stmt::Increment {
                name,
                is_pre: _,
                is_increment,
            } => {
                if let Some(&var_id) = self.variables.get(name) {
                    let load_id = self.new_value_id();
                    self.emit(Instruction::load(
                        IRType::I64,
                        Value::Instruction(var_id),
                        load_id,
                    ));

                    let one = Value::Constant(Constant::i64(1));
                    let op = if *is_increment {
                        BinaryOp::Add
                    } else {
                        BinaryOp::Sub
                    };
                    let result_id = self.new_value_id();
                    self.emit(Instruction::binary(
                        op,
                        IRType::I64,
                        Value::Instruction(load_id),
                        one,
                        result_id,
                    ));

                    self.emit(Instruction::store(
                        Value::Instruction(result_id),
                        Value::Instruction(var_id),
                    ));
                }
            }

            Stmt::Switch {
                expr,
                cases,
                default,
            } => {
                let switch_val = self.lower_expr(expr, &IRType::I64);
                let exit_bb = self.new_block("switch.exit");

                // Create blocks for each case
                let mut case_blocks: Vec<(i64, BasicBlockId)> = Vec::new();
                for case in cases {
                    if let Expr::Number(n) = &case.value {
                        let bb = self.new_block("switch.case");
                        case_blocks.push((*n, bb));
                    }
                }

                let default_bb = self.new_block("switch.default");

                // Generate cascading comparisons (simple switch lowering)
                for (val, bb) in &case_blocks {
                    let cmp_id = self.new_value_id();
                    let const_val = Value::Constant(Constant::i64(*val));
                    self.emit(Instruction::icmp(
                        CompareOp::Eq,
                        switch_val.clone(),
                        const_val,
                        cmp_id,
                    ));

                    let next_bb = self.new_block("switch.next");
                    self.emit(Instruction::cond_br(
                        Value::Instruction(cmp_id),
                        bb.0,
                        next_bb.0,
                    ));
                    self.current_block = Some(next_bb);
                }

                self.emit(Instruction::br(default_bb.0));

                // Lower case bodies
                for (i, case) in cases.iter().enumerate() {
                    if i < case_blocks.len() {
                        self.current_block = Some(case_blocks[i].1);
                        for s in &case.body {
                            self.lower_stmt(s);
                        }
                        if case.has_break {
                            self.emit(Instruction::br(exit_bb.0));
                        } else if i + 1 < case_blocks.len() {
                            self.emit(Instruction::br(case_blocks[i + 1].1 .0));
                        } else {
                            self.emit(Instruction::br(default_bb.0));
                        }
                    }
                }

                // Default case
                self.current_block = Some(default_bb);
                if let Some(default_stmts) = default {
                    for s in default_stmts {
                        self.lower_stmt(s);
                    }
                }
                self.emit(Instruction::br(exit_bb.0));

                self.current_block = Some(exit_bb);
            }

            // Handle other statements as needed
            _ => {}
        }
    }

    /// Lower an expression
    fn lower_expr(&mut self, expr: &Expr, _expected_ty: &IRType) -> Value {
        match expr {
            Expr::Number(n) => Value::Constant(Constant::i64(*n)),

            Expr::Float(f) => Value::Constant(Constant::f64(*f)),

            Expr::Bool(b) => Value::Constant(Constant::bool(*b)),

            Expr::String(s) => {
                let idx = self.module.add_string(s);
                Value::Constant(Constant::i64(idx as i64))
            }

            Expr::Null => Value::Constant(Constant::null(IRType::ptr(IRType::I8))),

            Expr::Variable(name) => {
                if let Some(&var_id) = self.variables.get(name) {
                    let load_id = self.new_value_id();
                    self.emit(Instruction::load(
                        IRType::I64,
                        Value::Instruction(var_id),
                        load_id,
                    ));
                    Value::Instruction(load_id)
                } else {
                    Value::Constant(Constant::i64(0))
                }
            }

            Expr::BinaryOp { op, left, right } => {
                let lhs = self.lower_expr(left, &IRType::I64);
                let rhs = self.lower_expr(right, &IRType::I64);
                let bin_op = lower_binop(op);
                let result_id = self.new_value_id();
                self.emit(Instruction::binary(
                    bin_op,
                    IRType::I64,
                    lhs,
                    rhs,
                    result_id,
                ));
                Value::Instruction(result_id)
            }

            Expr::Comparison { op, left, right } => {
                let lhs = self.lower_expr(left, &IRType::I64);
                let rhs = self.lower_expr(right, &IRType::I64);
                let cmp_op = lower_cmpop(op);
                let result_id = self.new_value_id();
                self.emit(Instruction::icmp(cmp_op, lhs, rhs, result_id));
                Value::Instruction(result_id)
            }

            Expr::UnaryOp { op, expr: inner } => {
                let val = self.lower_expr(inner, &IRType::I64);
                match op {
                    UnaryOp::Neg => {
                        let zero = Value::Constant(Constant::i64(0));
                        let result_id = self.new_value_id();
                        self.emit(Instruction::binary(
                            BinaryOp::Sub,
                            IRType::I64,
                            zero,
                            val,
                            result_id,
                        ));
                        Value::Instruction(result_id)
                    }
                    UnaryOp::Not => {
                        let one = Value::Constant(Constant::i64(1));
                        let result_id = self.new_value_id();
                        self.emit(Instruction::binary(
                            BinaryOp::Xor,
                            IRType::I64,
                            val,
                            one,
                            result_id,
                        ));
                        Value::Instruction(result_id)
                    }
                }
            }

            Expr::Call { name, args } => {
                let arg_vals: Vec<Value> = args
                    .iter()
                    .map(|a| self.lower_expr(a, &IRType::I64))
                    .collect();
                let result_id = self.new_value_id();
                self.emit(Instruction::call(
                    IRType::I64,
                    name,
                    arg_vals,
                    Some(result_id),
                ));
                Value::Instruction(result_id)
            }

            Expr::Index { object, index } => {
                let ptr = self.lower_expr(object, &IRType::ptr(IRType::I64));
                let idx = self.lower_expr(index, &IRType::I64);
                let gep_id = self.new_value_id();
                self.emit(Instruction::gep(IRType::I64, ptr, vec![idx], gep_id));
                let load_id = self.new_value_id();
                self.emit(Instruction::load(
                    IRType::I64,
                    Value::Instruction(gep_id),
                    load_id,
                ));
                Value::Instruction(load_id)
            }

            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond = self.lower_expr(condition, &IRType::Bool);
                let then_val = self.lower_expr(then_expr, &IRType::I64);
                let else_val = self.lower_expr(else_expr, &IRType::I64);
                let result_id = self.new_value_id();
                self.emit(Instruction::select(
                    IRType::I64,
                    cond,
                    then_val,
                    else_val,
                    result_id,
                ));
                Value::Instruction(result_id)
            }

            Expr::Deref(inner) => {
                let ptr = self.lower_expr(inner, &IRType::ptr(IRType::I64));
                let load_id = self.new_value_id();
                self.emit(Instruction::load(IRType::I64, ptr, load_id));
                Value::Instruction(load_id)
            }

            Expr::AddressOf(inner) => {
                if let Expr::Variable(name) = inner.as_ref() {
                    if let Some(&var_id) = self.variables.get(name) {
                        return Value::Instruction(var_id);
                    }
                }
                Value::Constant(Constant::i64(0))
            }

            Expr::FieldAccess { object, field } => {
                let _obj = self.lower_expr(object, &IRType::I64);
                // Simplified: just return a placeholder
                // Full implementation would compute field offset
                let _ = field;
                Value::Constant(Constant::i64(0))
            }

            Expr::IntCast(inner) => {
                let val = self.lower_expr(inner, &IRType::F64);
                let result_id = self.new_value_id();
                self.emit(Instruction::cast(
                    CastOp::FPToSI,
                    IRType::I64,
                    val,
                    result_id,
                ));
                Value::Instruction(result_id)
            }

            Expr::FloatCast(inner) => {
                let val = self.lower_expr(inner, &IRType::I64);
                let result_id = self.new_value_id();
                self.emit(Instruction::cast(
                    CastOp::SIToFP,
                    IRType::F64,
                    val,
                    result_id,
                ));
                Value::Instruction(result_id)
            }

            _ => Value::Constant(Constant::i64(0)),
        }
    }
}

/// Lower a C function to IR (simple interface)
pub fn lower_function(func: &AstFunction) -> Function {
    let return_type = lower_type(&func.resolved_return_type);
    let mut ir_func = Function::new(&func.name, return_type);

    for param in &func.params {
        let param_type = lower_type(&param.param_type);
        ir_func.add_param(&param.name, param_type);
    }

    let _entry = ir_func.create_block(Some("entry"));
    ir_func
}

/// Lower a C global variable to IR
#[allow(dead_code)]
pub fn lower_global_var(name: &str, ty: &AstType) -> GlobalVariable {
    let ir_ty = lower_type(ty);
    GlobalVariable::new(name, ir_ty)
}

/// Lower frontend type to IR type
pub fn lower_type(ty: &AstType) -> IRType {
    match ty {
        AstType::Void => IRType::Void,
        AstType::Bool => IRType::Bool,
        AstType::I8 | AstType::U8 => IRType::I8,
        AstType::I16 | AstType::U16 => IRType::I16,
        AstType::I32 | AstType::U32 => IRType::I32,
        AstType::I64 | AstType::U64 => IRType::I64,
        AstType::F32 => IRType::F32,
        AstType::F64 => IRType::F64,
        AstType::Str => IRType::ptr(IRType::I8),
        AstType::Pointer(inner) => IRType::ptr(lower_type(inner)),
        AstType::Array(inner, size) => IRType::array(lower_type(inner), size.unwrap_or(0)),
        AstType::Named(name) => IRType::named_struct(name, vec![]),
        AstType::Struct(name) => IRType::named_struct(name, vec![]),
        AstType::Class(name) => IRType::named_struct(name, vec![]),
        AstType::Reference(inner) => IRType::ptr(lower_type(inner)),
        AstType::Function(params, ret) => {
            let param_types: Vec<IRType> = params.iter().map(|p| lower_type(p)).collect();
            let ret_type = lower_type(ret);
            IRType::function(ret_type, param_types, false)
        }
        AstType::Vec4 | AstType::Vec8 | AstType::Vec16 => IRType::I64,
        AstType::Auto | AstType::Unknown => IRType::I64,
    }
}

/// Convert AST BinOp to IR BinaryOp
fn lower_binop(op: &BinOp) -> BinaryOp {
    match op {
        BinOp::Add => BinaryOp::Add,
        BinOp::Sub => BinaryOp::Sub,
        BinOp::Mul => BinaryOp::Mul,
        BinOp::Div => BinaryOp::SDiv,
        BinOp::Mod => BinaryOp::SRem,
        BinOp::And => BinaryOp::And,
        BinOp::Or => BinaryOp::Or,
    }
}

/// Convert AST CmpOp to IR CompareOp
fn lower_cmpop(op: &CmpOp) -> CompareOp {
    match op {
        CmpOp::Eq => CompareOp::Eq,
        CmpOp::Ne => CompareOp::Ne,
        CmpOp::Lt => CompareOp::Slt,
        CmpOp::Le => CompareOp::Sle,
        CmpOp::Gt => CompareOp::Sgt,
        CmpOp::Ge => CompareOp::Sge,
    }
}

/// Convert CompoundOp to BinaryOp
fn compound_to_binary(op: &crate::frontend::ast::CompoundOp) -> BinaryOp {
    use crate::frontend::ast::CompoundOp;
    match op {
        CompoundOp::AddAssign => BinaryOp::Add,
        CompoundOp::SubAssign => BinaryOp::Sub,
        CompoundOp::MulAssign => BinaryOp::Mul,
        CompoundOp::DivAssign => BinaryOp::SDiv,
        CompoundOp::ModAssign => BinaryOp::SRem,
        CompoundOp::AndAssign => BinaryOp::And,
        CompoundOp::OrAssign => BinaryOp::Or,
        CompoundOp::XorAssign => BinaryOp::Xor,
        CompoundOp::ShlAssign => BinaryOp::Shl,
        CompoundOp::ShrAssign => BinaryOp::AShr,
    }
}

/// Lower C source to IR Module (entry point)
pub fn lower_c_to_ir(_source: &str) -> Module {
    Module::new("c_module")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lower_type() {
        assert_eq!(lower_type(&AstType::I32), IRType::I32);
        assert_eq!(lower_type(&AstType::Void), IRType::Void);
    }

    #[test]
    fn test_lower_binop() {
        assert_eq!(lower_binop(&BinOp::Add), BinaryOp::Add);
        assert_eq!(lower_binop(&BinOp::Mul), BinaryOp::Mul);
    }
}
