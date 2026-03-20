// ============================================================
// C AST → ADead-BIB IR Converter (v2.0 — Robust)
// ============================================================
// Lowers C99 AST to ADead-BIB's Program/Function/Stmt/Expr
// This is the bridge: C enters here, ADead-BIB IR exits.
//
// Pipeline: C Source → CLexer → CParser → CTranslationUnit
//           → CToIR → Program → ISA Compiler → PE/ELF
//
// Fixed: string duplication, printf format handling,
//        VarDecl with types, compound assigns, globals,
//        assignment expressions, NULL handling.
// ============================================================

use super::c_ast::*;
use crate::frontend::ast::{
    self, BinOp, BitwiseOp, CmpOp, Expr, Function, FunctionAttributes, Param, Program,
    ProgramAttributes, Stmt, Struct, StructField, UnaryOp,
};
use crate::frontend::types::Type;

use std::sync::atomic::{AtomicU64, Ordering};

/// Temp variable counter for synthesized names
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn fresh_temp(prefix: &str) -> String {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("__{}{}", prefix, id)
}

pub struct CToIR {
    /// Accumulated enum constants as global assigns
    enum_constants: Vec<(String, i64)>,
    /// Typedef aliases (new_name → original CType)
    typedefs: Vec<(String, CType)>,
}

impl CToIR {
    pub fn new() -> Self {
        Self {
            enum_constants: Vec::new(),
            typedefs: Vec::new(),
        }
    }

    /// Main entry: Convert entire C translation unit → ADead-BIB Program
    pub fn convert(&mut self, unit: &CTranslationUnit) -> Result<Program, String> {
        let mut program = Program::new();
        program.attributes = ProgramAttributes::default();

        // First pass: collect structs, enums, typedefs
        for decl in &unit.declarations {
            match decl {
                CTopLevel::StructDef { name, fields } => {
                    program.structs.push(self.convert_struct(name, fields));
                }
                CTopLevel::EnumDef { name: _, values } => {
                    self.collect_enum_constants(values);
                }
                CTopLevel::TypedefDecl { original, new_name } => {
                    self.typedefs.push((new_name.clone(), original.clone()));
                }
                _ => {}
            }
        }

        // Second pass: functions and global vars
        // Pre-scan: collect static locals from function bodies as globals
        for decl in &unit.declarations {
            if let CTopLevel::FunctionDef { body, .. } = decl {
                self.collect_static_locals(body, &mut program.statements);
            }
        }

        for decl in &unit.declarations {
            match decl {
                CTopLevel::FunctionDef {
                    return_type,
                    name,
                    params,
                    body,
                } => {
                    let func = self.convert_function(return_type, name, params, body)?;
                    program.functions.push(func);
                }
                CTopLevel::FunctionDecl { .. } => {
                    // Prototypes — skip (resolved at link time)
                }
                CTopLevel::GlobalVar {
                    type_spec,
                    declarators,
                } => {
                    for decl_item in declarators {
                        let var_type =
                            self.resolve_declarator_type(type_spec, &decl_item.derived_type);
                        let init_val = if let Some(ref init) = decl_item.initializer {
                            Some(self.convert_expr(init)?)
                        } else {
                            None
                        };
                        program.statements.push(Stmt::VarDecl {
                            var_type,
                            name: decl_item.name.clone(),
                            value: init_val,
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(program)
    }

    // ========== Type conversion ==========

    fn convert_type(&self, cty: &CType) -> Type {
        match cty {
            CType::Void => Type::Void,
            CType::Char => Type::I8,
            CType::Short => Type::I16,
            CType::Int => Type::I32,
            CType::Long => Type::I64,
            CType::LongLong => Type::I64,
            CType::Float => Type::F32,
            CType::Double => Type::F64,
            CType::Bool => Type::Bool,
            CType::Unsigned(inner) => match inner.as_ref() {
                CType::Char => Type::U8,
                CType::Short => Type::U16,
                CType::Int => Type::U32,
                CType::Long | CType::LongLong => Type::U64,
                _ => Type::U32,
            },
            CType::Signed(inner) => self.convert_type(inner),
            CType::Pointer(inner) => Type::Pointer(Box::new(self.convert_type(inner))),
            CType::Array(inner, size) => Type::Array(Box::new(self.convert_type(inner)), *size),
            CType::Struct(name) => Type::Struct(name.clone()),
            CType::Enum(_) => Type::I32,
            CType::Typedef(name) => {
                if let Some((_, original)) = self.typedefs.iter().find(|(n, _)| n == name) {
                    self.convert_type(original)
                } else {
                    Type::Named(name.clone())
                }
            }
            CType::Function {
                return_type,
                params,
            } => {
                let ret = self.convert_type(return_type);
                let args: Vec<Type> = params.iter().map(|p| self.convert_type(p)).collect();
                Type::Function(args, Box::new(ret))
            }
            CType::Const(inner) | CType::Volatile(inner) | CType::Complex(inner) => self.convert_type(inner),
        }
    }

    /// Resolve full type including declarator modifiers (pointer/array)
    fn resolve_declarator_type(&self, base: &CType, derived: &Option<CDerivedType>) -> Type {
        let base_type = self.convert_type(base);
        match derived {
            None => base_type,
            Some(CDerivedType::Pointer(inner)) => {
                let inner_type =
                    self.resolve_declarator_type(base, &inner.as_ref().map(|b| *b.clone()));
                Type::Pointer(Box::new(inner_type))
            }
            Some(CDerivedType::Array(size, inner)) => {
                let inner_type =
                    self.resolve_declarator_type(base, &inner.as_ref().map(|b| *b.clone()));
                Type::Array(Box::new(inner_type), *size)
            }
        }
    }

    fn type_name(cty: &CType) -> Option<String> {
        match cty {
            CType::Void => Some("void".to_string()),
            CType::Char => Some("char".to_string()),
            CType::Short => Some("short".to_string()),
            CType::Int => Some("int".to_string()),
            CType::Long => Some("long".to_string()),
            CType::Float => Some("float".to_string()),
            CType::Double => Some("double".to_string()),
            CType::Bool => Some("bool".to_string()),
            _ => None,
        }
    }

    // ========== Struct conversion ==========

    fn convert_struct(&self, name: &str, fields: &[CStructField]) -> Struct {
        Struct {
            name: name.to_string(),
            fields: fields
                .iter()
                .map(|f| StructField {
                    name: f.name.clone(),
                    field_type: self.convert_type(&f.field_type),
                })
                .collect(),
            is_packed: false,
        }
    }

    // ========== Enum constants ==========

    fn collect_enum_constants(&mut self, values: &[(String, Option<i64>)]) {
        let mut next_val: i64 = 0;
        for (name, explicit_val) in values {
            let val = explicit_val.unwrap_or(next_val);
            self.enum_constants.push((name.clone(), val));
            next_val = val + 1;
        }
    }

    /// Collect static local variables from function body and add them as globals
    fn collect_static_locals(&self, stmts: &[CStmt], globals: &mut Vec<Stmt>) {
        for stmt in stmts {
            match stmt {
                CStmt::VarDecl {
                    type_spec,
                    declarators,
                    is_static,
                } if *is_static => {
                    for decl in declarators {
                        let var_type = self.resolve_declarator_type(type_spec, &decl.derived_type);
                        let init_val = if let Some(ref init) = decl.initializer {
                            self.convert_expr(init).ok()
                        } else {
                            None
                        };
                        globals.push(Stmt::VarDecl {
                            var_type,
                            name: decl.name.clone(),
                            value: init_val,
                        });
                    }
                }
                CStmt::Block(inner) => self.collect_static_locals(inner, globals),
                CStmt::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    if let CStmt::Block(inner) = then_body.as_ref() {
                        self.collect_static_locals(inner, globals);
                    }
                    if let Some(eb) = else_body {
                        if let CStmt::Block(inner) = eb.as_ref() {
                            self.collect_static_locals(inner, globals);
                        }
                    }
                }
                CStmt::While { body, .. } | CStmt::DoWhile { body, .. } => {
                    if let CStmt::Block(inner) = body.as_ref() {
                        self.collect_static_locals(inner, globals);
                    }
                }
                CStmt::For { body, .. } => {
                    if let CStmt::Block(inner) = body.as_ref() {
                        self.collect_static_locals(inner, globals);
                    }
                }
                _ => {}
            }
        }
    }

    // ========== Function conversion ==========

    fn convert_function(
        &self,
        return_type: &CType,
        name: &str,
        params: &[CParam],
        body: &[CStmt],
    ) -> Result<Function, String> {
        let adead_params: Vec<Param> = params
            .iter()
            .map(|p| {
                let pname = p.name.clone().unwrap_or_else(|| "_".to_string());
                Param {
                    name: pname,
                    param_type: self.convert_type(&p.param_type),
                    default_value: None,
                }
            })
            .collect();

        let mut adead_body = Vec::new();
        for stmt in body {
            let converted = self.convert_stmt(stmt)?;
            adead_body.extend(converted);
        }

        Ok(Function {
            name: name.to_string(),
            params: adead_params,
            return_type: Self::type_name(return_type),
            resolved_return_type: self.convert_type(return_type),
            body: adead_body,
            attributes: FunctionAttributes::default(),
        })
    }

    // ========== Statement conversion ==========

    fn convert_stmt(&self, stmt: &CStmt) -> Result<Vec<Stmt>, String> {
        match stmt {
            CStmt::LineMarker(l) => Ok(vec![Stmt::LineMarker(*l)]),
            CStmt::Expr(expr) => self.convert_expr_to_stmt(expr),

            CStmt::Return(None) => Ok(vec![Stmt::Return(None)]),
            CStmt::Return(Some(expr)) => {
                let val = self.convert_expr(expr)?;
                Ok(vec![Stmt::Return(Some(val))])
            }

            CStmt::VarDecl {
                type_spec,
                declarators,
                is_static,
            } => {
                let mut stmts = Vec::new();
                for decl in declarators {
                    let var_type = self.resolve_declarator_type(type_spec, &decl.derived_type);

                    let init_val = if let Some(ref init) = decl.initializer {
                        Some(self.convert_expr(init)?)
                    } else {
                        None
                    };

                    if *is_static {
                        // Static local: emit nothing in function body.
                        // The variable is pre-registered as a global in program.statements
                        // by convert_function, so the ISA compiler will find it there.
                        // Skip — no local VarDecl needed.
                    } else {
                        // Use VarDecl with full type info — not bare Assign
                        stmts.push(Stmt::VarDecl {
                            var_type,
                            name: decl.name.clone(),
                            value: init_val,
                        });
                    }
                }
                Ok(stmts)
            }

            CStmt::Block(stmts) => {
                let mut result = Vec::new();
                for s in stmts {
                    result.extend(self.convert_stmt(s)?);
                }
                Ok(result)
            }

            CStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                let cond = self.convert_expr(condition)?;
                let then_stmts = self.convert_stmt(then_body)?;
                let else_stmts = if let Some(else_b) = else_body {
                    Some(self.convert_stmt(else_b)?)
                } else {
                    None
                };
                Ok(vec![Stmt::If {
                    condition: cond,
                    then_body: then_stmts,
                    else_body: else_stmts,
                }])
            }

            CStmt::While { condition, body } => {
                let cond = self.convert_expr(condition)?;
                let body_stmts = self.convert_stmt(body)?;
                Ok(vec![Stmt::While {
                    condition: cond,
                    body: body_stmts,
                }])
            }

            CStmt::DoWhile { body, condition } => {
                let cond = self.convert_expr(condition)?;
                let mut body_stmts = self.convert_stmt(body)?;
                body_stmts.push(Stmt::If {
                    condition: Expr::UnaryOp {
                        op: UnaryOp::Not,
                        expr: Box::new(cond),
                    },
                    then_body: vec![Stmt::Break],
                    else_body: None,
                });
                Ok(vec![Stmt::While {
                    condition: Expr::Bool(true),
                    body: body_stmts,
                }])
            }

            CStmt::For {
                init,
                condition,
                update,
                body,
            } => {
                let mut result = Vec::new();

                if let Some(init_stmt) = init {
                    result.extend(self.convert_stmt(init_stmt)?);
                }

                let mut loop_body = self.convert_stmt(body)?;

                if let Some(upd) = update {
                    let upd_stmts = self.convert_expr_to_stmt(upd)?;
                    loop_body.extend(upd_stmts);
                }

                let cond = if let Some(c) = condition {
                    self.convert_expr(c)?
                } else {
                    Expr::Bool(true)
                };

                result.push(Stmt::While {
                    condition: cond,
                    body: loop_body,
                });

                Ok(result)
            }

            CStmt::Switch { expr, cases } => {
                let switch_val = self.convert_expr(expr)?;
                let switch_var = fresh_temp("sw");
                let mut result = vec![Stmt::Assign {
                    name: switch_var.clone(),
                    value: switch_val,
                }];

                let mut last_else: Option<Vec<Stmt>> = None;

                for case in cases.iter().rev() {
                    let mut case_body: Vec<Stmt> = Vec::new();
                    for s in &case.body {
                        let stmts = self.convert_stmt(s)?;
                        for st in stmts {
                            if matches!(st, Stmt::Break) {
                                break;
                            }
                            case_body.push(st);
                        }
                    }

                    if let Some(ref val) = case.value {
                        let cond = Expr::Comparison {
                            op: CmpOp::Eq,
                            left: Box::new(Expr::Variable(switch_var.clone())),
                            right: Box::new(self.convert_expr(val)?),
                        };
                        let if_stmt = Stmt::If {
                            condition: cond,
                            then_body: case_body,
                            else_body: last_else.take(),
                        };
                        last_else = Some(vec![if_stmt]);
                    } else {
                        last_else = Some(case_body);
                    }
                }

                if let Some(stmts) = last_else {
                    result.extend(stmts);
                }

                Ok(result)
            }

            CStmt::Break => Ok(vec![Stmt::Break]),
            CStmt::Continue => Ok(vec![Stmt::Continue]),
            CStmt::Goto(_label) => Ok(vec![]),
            CStmt::Label(_name, inner) => self.convert_stmt(inner),
            CStmt::Empty => Ok(vec![]),
        }
    }

    // ========== Expression as statement ==========

    fn convert_expr_to_stmt(&self, expr: &CExpr) -> Result<Vec<Stmt>, String> {
        match expr {
            CExpr::Call { func, args } => {
                if let CExpr::Identifier(name) = func.as_ref() {
                    match name.as_str() {
                        "printf" => return self.convert_printf(args),
                        "fprintf" => {
                            // fprintf(stderr, fmt, ...) → skip first arg, treat like printf
                            if args.len() >= 2 {
                                return self.convert_printf(&args[1..]);
                            }
                            return Ok(vec![]);
                        }
                        "sprintf" | "snprintf" => {
                            // sprintf(buf, fmt, ...) → just print for now
                            if args.len() >= 2 {
                                return self.convert_printf(&args[1..]);
                            }
                            return Ok(vec![]);
                        }
                        "puts" => {
                            if let Some(arg) = args.first() {
                                let val = self.convert_expr(arg)?;
                                return Ok(vec![Stmt::Println(val)]);
                            }
                            return Ok(vec![Stmt::Println(Expr::String(String::new()))]);
                        }
                        "putchar" | "putc" => {
                            if let Some(arg) = args.first() {
                                let val = self.convert_expr(arg)?;
                                return Ok(vec![Stmt::Print(val)]);
                            }
                            return Ok(vec![]);
                        }
                        "free" => {
                            if let Some(arg) = args.first() {
                                let val = self.convert_expr(arg)?;
                                return Ok(vec![Stmt::Free(val)]);
                            }
                            return Ok(vec![]);
                        }
                        "exit" => {
                            let code = if let Some(arg) = args.first() {
                                self.convert_expr(arg)?
                            } else {
                                Expr::Number(0)
                            };
                            return Ok(vec![Stmt::Return(Some(code))]);
                        }
                        "memset" | "memcpy" | "memmove" | "strcpy" | "strncpy" | "strcat"
                        | "strlen" | "strcmp" | "atoi" | "atof" => {
                            // Map to generic call
                            let a: Result<Vec<Expr>, String> =
                                args.iter().map(|a| self.convert_expr(a)).collect();
                            return Ok(vec![Stmt::Expr(Expr::Call {
                                name: name.clone(),
                                args: a?,
                            })]);
                        }
                        _ => {}
                    }
                }
                // General function call as statement
                let call_expr = self.convert_expr(expr)?;
                Ok(vec![Stmt::Expr(call_expr)])
            }

            CExpr::Assign { op, target, value } => self.convert_assignment(op, target, value),

            CExpr::UnaryOp {
                op, expr: inner, ..
            } => match op {
                CUnaryOp::PreInc | CUnaryOp::PostInc => {
                    if let CExpr::Identifier(name) = inner.as_ref() {
                        Ok(vec![Stmt::Increment {
                            name: name.clone(),
                            is_pre: matches!(op, CUnaryOp::PreInc),
                            is_increment: true,
                        }])
                    } else {
                        let e = self.convert_expr(expr)?;
                        Ok(vec![Stmt::Expr(e)])
                    }
                }
                CUnaryOp::PreDec | CUnaryOp::PostDec => {
                    if let CExpr::Identifier(name) = inner.as_ref() {
                        Ok(vec![Stmt::Increment {
                            name: name.clone(),
                            is_pre: matches!(op, CUnaryOp::PreDec),
                            is_increment: false,
                        }])
                    } else {
                        let e = self.convert_expr(expr)?;
                        Ok(vec![Stmt::Expr(e)])
                    }
                }
                _ => {
                    let e = self.convert_expr(expr)?;
                    Ok(vec![Stmt::Expr(e)])
                }
            },

            CExpr::Comma(exprs) => {
                let mut stmts = Vec::new();
                for e in exprs {
                    stmts.extend(self.convert_expr_to_stmt(e)?);
                }
                Ok(stmts)
            }

            _ => {
                let e = self.convert_expr(expr)?;
                Ok(vec![Stmt::Expr(e)])
            }
        }
    }

    // ========== Assignment conversion ==========

    fn convert_assignment(
        &self,
        op: &CAssignOp,
        target: &CExpr,
        value: &CExpr,
    ) -> Result<Vec<Stmt>, String> {
        let rhs = self.convert_expr(value)?;

        match target {
            CExpr::Identifier(name) => {
                let final_value = self.apply_compound_op(op, &Expr::Variable(name.clone()), rhs);
                Ok(vec![Stmt::Assign {
                    name: name.clone(),
                    value: final_value,
                }])
            }
            CExpr::Index { array, index } => {
                let obj = self.convert_expr(array)?;
                let idx = self.convert_expr(index)?;

                // For compound ops: read-modify-write
                let final_rhs = if *op == CAssignOp::Assign {
                    rhs
                } else {
                    // Read current value
                    let current = Expr::Index {
                        object: Box::new(obj.clone()),
                        index: Box::new(idx.clone()),
                    };
                    self.apply_compound_op(op, &current, rhs)
                };

                Ok(vec![Stmt::IndexAssign {
                    object: obj,
                    index: idx,
                    value: final_rhs,
                }])
            }
            CExpr::Member { object, field } => {
                let obj = self.convert_expr(object)?;
                let final_rhs = if *op == CAssignOp::Assign {
                    rhs
                } else {
                    let current = Expr::FieldAccess {
                        object: Box::new(obj.clone()),
                        field: field.clone(),
                    };
                    self.apply_compound_op(op, &current, rhs)
                };
                Ok(vec![Stmt::FieldAssign {
                    object: obj,
                    field: field.clone(),
                    value: final_rhs,
                }])
            }
            CExpr::ArrowMember { pointer, field } => {
                let ptr = self.convert_expr(pointer)?;
                let final_rhs = if *op == CAssignOp::Assign {
                    rhs
                } else {
                    let current = Expr::ArrowAccess {
                        pointer: Box::new(ptr.clone()),
                        field: field.clone(),
                    };
                    self.apply_compound_op(op, &current, rhs)
                };
                Ok(vec![Stmt::ArrowAssign {
                    pointer: ptr,
                    field: field.clone(),
                    value: final_rhs,
                }])
            }
            CExpr::Deref(inner) => {
                let ptr = self.convert_expr(inner)?;
                let final_rhs = if *op == CAssignOp::Assign {
                    rhs
                } else {
                    let current = Expr::Deref(Box::new(ptr.clone()));
                    self.apply_compound_op(op, &current, rhs)
                };
                Ok(vec![Stmt::DerefAssign {
                    pointer: ptr,
                    value: final_rhs,
                }])
            }
            _ => Ok(vec![Stmt::Expr(rhs)]),
        }
    }

    /// Apply compound assignment operator: lhs <op>= rhs
    fn apply_compound_op(&self, op: &CAssignOp, lhs: &Expr, rhs: Expr) -> Expr {
        match op {
            CAssignOp::Assign => rhs,
            CAssignOp::AddAssign => Expr::BinaryOp {
                op: BinOp::Add,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::SubAssign => Expr::BinaryOp {
                op: BinOp::Sub,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::MulAssign => Expr::BinaryOp {
                op: BinOp::Mul,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::DivAssign => Expr::BinaryOp {
                op: BinOp::Div,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::ModAssign => Expr::BinaryOp {
                op: BinOp::Mod,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::AndAssign => Expr::BitwiseOp {
                op: BitwiseOp::And,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::OrAssign => Expr::BitwiseOp {
                op: BitwiseOp::Or,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::XorAssign => Expr::BitwiseOp {
                op: BitwiseOp::Xor,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::ShlAssign => Expr::BitwiseOp {
                op: BitwiseOp::LeftShift,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
            CAssignOp::ShrAssign => Expr::BitwiseOp {
                op: BitwiseOp::RightShift,
                left: Box::new(lhs.clone()),
                right: Box::new(rhs),
            },
        }
    }

    // ========== printf conversion (robust) ==========

    fn convert_printf(&self, args: &[CExpr]) -> Result<Vec<Stmt>, String> {
        if args.is_empty() {
            return Ok(vec![]);
        }

        // First arg is format string
        let fmt = match &args[0] {
            CExpr::StringLiteral(s) => s.clone(),
            other => {
                // Non-string format: just print
                let val = self.convert_expr(other)?;
                return Ok(vec![Stmt::Print(val)]);
            }
        };

        // Simple case: no format specifiers
        if !fmt.contains('%') {
            return Ok(self.emit_string_segments(&fmt));
        }

        let mut stmts = Vec::new();
        let mut arg_idx = 1;
        let mut current_str = String::new();
        let chars: Vec<char> = fmt.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '%' && i + 1 < chars.len() {
                i += 1;

                // %% → literal %
                if i < chars.len() && chars[i] == '%' {
                    current_str.push('%');
                    i += 1;
                    continue;
                }

                // Skip flags: -, +, 0, space, #
                while i < chars.len() && matches!(chars[i], '-' | '+' | '0' | ' ' | '#') {
                    i += 1;
                }
                // Skip width
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                // Skip precision
                if i < chars.len() && chars[i] == '.' {
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                // Skip length modifiers: h, hh, l, ll, L, z, j, t
                while i < chars.len() && matches!(chars[i], 'h' | 'l' | 'L' | 'z' | 'j' | 't') {
                    i += 1;
                }

                if i >= chars.len() {
                    break;
                }

                // Flush text before format specifier
                if !current_str.is_empty() {
                    stmts.push(Stmt::Print(Expr::String(current_str.clone())));
                    current_str.clear();
                }

                // Process conversion specifier
                match chars[i] {
                    'd' | 'i' | 'u' | 'x' | 'X' | 'o' | 'c' | 'f' | 'e' | 'E' | 'g' | 'G' | 's'
                    | 'p' => {
                        if arg_idx < args.len() {
                            let val = self.convert_expr(&args[arg_idx])?;
                            stmts.push(Stmt::Print(val));
                            arg_idx += 1;
                        }
                    }
                    'n' => {
                        // %n — skip (dangerous)
                        arg_idx += 1;
                    }
                    _ => {}
                }
                i += 1;
            } else {
                current_str.push(chars[i]);
                i += 1;
            }
        }

        // Flush remaining text
        if !current_str.is_empty() {
            stmts.extend(self.emit_string_segments(&current_str));
        }

        // Fallback: if nothing was emitted, print the raw format string
        if stmts.is_empty() {
            stmts.push(Stmt::Print(Expr::String(fmt)));
        }

        // Post-process: merge trailing newlines into the previous Print
        // e.g. [Print(x), Println("")] → [Println(x)]
        self.merge_trailing_newlines(&mut stmts);

        Ok(stmts)
    }

    /// Merge trailing Println("") into previous Print → Println
    /// [Print("hello"), Print(x), Println("")] → [Print("hello"), Println(x)]
    fn merge_trailing_newlines(&self, stmts: &mut Vec<Stmt>) {
        while stmts.len() >= 2 {
            let is_empty_println = matches!(
                stmts.last(),
                Some(Stmt::Println(Expr::String(s))) if s.is_empty()
            );
            if !is_empty_println {
                break;
            }

            // Check if previous is a Print we can upgrade
            let prev_idx = stmts.len() - 2;
            let can_merge = matches!(&stmts[prev_idx], Stmt::Print(_));
            if !can_merge {
                break;
            }

            stmts.pop(); // remove Println("")
            if let Some(Stmt::Print(expr)) = stmts.pop() {
                stmts.push(Stmt::Println(expr));
            }
            break;
        }
    }

    /// Split a string at newlines into Print/Println statements.
    /// "hello\nworld\n" → [Println("hello"), Println("world")]
    /// "hello" → [Print("hello")]
    fn emit_string_segments(&self, s: &str) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        let parts: Vec<&str> = s.split('\n').collect();

        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;

            if is_last {
                // Last segment: no trailing newline
                if !part.is_empty() {
                    stmts.push(Stmt::Print(Expr::String(part.to_string())));
                }
            } else {
                // Not last: this segment has a trailing newline
                stmts.push(Stmt::Println(Expr::String(part.to_string())));
            }
        }

        stmts
    }

    // ========== Expression conversion ==========

    fn convert_expr(&self, expr: &CExpr) -> Result<Expr, String> {
        match expr {
            CExpr::IntLiteral(n) => Ok(Expr::Number(*n)),
            CExpr::FloatLiteral(f) => Ok(Expr::Float(*f)),
            CExpr::StringLiteral(s) => Ok(Expr::String(s.clone())),
            CExpr::CharLiteral(c) => Ok(Expr::Number(*c as i64)),
            CExpr::Null => Ok(Expr::Nullptr),

            CExpr::Identifier(name) => {
                // Check enum constants
                if let Some((_, val)) = self.enum_constants.iter().find(|(n, _)| n == name) {
                    Ok(Expr::Number(*val))
                } else if name == "NULL" || name == "nullptr" {
                    Ok(Expr::Nullptr)
                } else if name == "true" || name == "TRUE" {
                    Ok(Expr::Bool(true))
                } else if name == "false" || name == "FALSE" {
                    Ok(Expr::Bool(false))
                } else if name == "stdin" || name == "stdout" || name == "stderr" {
                    // File handles → treat as integer constants
                    Ok(Expr::Number(match name.as_str() {
                        "stdin" => 0,
                        "stdout" => 1,
                        "stderr" => 2,
                        _ => 0,
                    }))
                } else {
                    Ok(Expr::Variable(name.clone()))
                }
            }

            CExpr::BinaryOp { op, left, right } => {
                let l = self.convert_expr(left)?;
                let r = self.convert_expr(right)?;
                Ok(match op {
                    CBinOp::Add => Expr::BinaryOp {
                        op: BinOp::Add,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Sub => Expr::BinaryOp {
                        op: BinOp::Sub,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Mul => Expr::BinaryOp {
                        op: BinOp::Mul,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Div => Expr::BinaryOp {
                        op: BinOp::Div,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Mod => Expr::BinaryOp {
                        op: BinOp::Mod,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Eq => Expr::Comparison {
                        op: CmpOp::Eq,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Ne => Expr::Comparison {
                        op: CmpOp::Ne,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Lt => Expr::Comparison {
                        op: CmpOp::Lt,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Gt => Expr::Comparison {
                        op: CmpOp::Gt,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Le => Expr::Comparison {
                        op: CmpOp::Le,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Ge => Expr::Comparison {
                        op: CmpOp::Ge,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::LogAnd => Expr::BinaryOp {
                        op: BinOp::And,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::LogOr => Expr::BinaryOp {
                        op: BinOp::Or,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::BitAnd => Expr::BitwiseOp {
                        op: BitwiseOp::And,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::BitOr => Expr::BitwiseOp {
                        op: BitwiseOp::Or,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::BitXor => Expr::BitwiseOp {
                        op: BitwiseOp::Xor,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Shl => Expr::BitwiseOp {
                        op: BitwiseOp::LeftShift,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                    CBinOp::Shr => Expr::BitwiseOp {
                        op: BitwiseOp::RightShift,
                        left: Box::new(l),
                        right: Box::new(r),
                    },
                })
            }

            CExpr::UnaryOp {
                op, expr: inner, ..
            } => {
                let e = self.convert_expr(inner)?;
                Ok(match op {
                    CUnaryOp::Neg => Expr::UnaryOp {
                        op: UnaryOp::Neg,
                        expr: Box::new(e),
                    },
                    CUnaryOp::LogNot => Expr::UnaryOp {
                        op: UnaryOp::Not,
                        expr: Box::new(e),
                    },
                    CUnaryOp::BitNot => Expr::BitwiseNot(Box::new(e)),
                    CUnaryOp::PreInc => Expr::PreIncrement(Box::new(e)),
                    CUnaryOp::PreDec => Expr::PreDecrement(Box::new(e)),
                    CUnaryOp::PostInc => Expr::PostIncrement(Box::new(e)),
                    CUnaryOp::PostDec => Expr::PostDecrement(Box::new(e)),
                })
            }

            CExpr::Call { func, args } => {
                let name = match func.as_ref() {
                    CExpr::Identifier(n) => n.clone(),
                    _ => {
                        // Function pointer call — not yet supported, emit as error marker
                        let a: Result<Vec<Expr>, String> =
                            args.iter().map(|a| self.convert_expr(a)).collect();
                        return Ok(Expr::Call {
                            name: "__fptr_call".to_string(),
                            args: a?,
                        });
                    }
                };

                match name.as_str() {
                    "malloc" | "calloc" => {
                        if let Some(size_arg) = args.first() {
                            let size = self.convert_expr(size_arg)?;
                            Ok(Expr::Malloc(Box::new(size)))
                        } else {
                            Ok(Expr::Nullptr)
                        }
                    }
                    "realloc" => {
                        if args.len() >= 2 {
                            let ptr = self.convert_expr(&args[0])?;
                            let size = self.convert_expr(&args[1])?;
                            Ok(Expr::Realloc {
                                ptr: Box::new(ptr),
                                new_size: Box::new(size),
                            })
                        } else {
                            Ok(Expr::Nullptr)
                        }
                    }
                    "sizeof" => {
                        if let Some(arg) = args.first() {
                            let e = self.convert_expr(arg)?;
                            Ok(Expr::SizeOf(Box::new(ast::SizeOfArg::Expr(e))))
                        } else {
                            Ok(Expr::Number(0))
                        }
                    }
                    "abs" => {
                        // abs(x) → x < 0 ? -x : x
                        if let Some(arg) = args.first() {
                            let e = self.convert_expr(arg)?;
                            Ok(Expr::Ternary {
                                condition: Box::new(Expr::Comparison {
                                    op: CmpOp::Lt,
                                    left: Box::new(e.clone()),
                                    right: Box::new(Expr::Number(0)),
                                }),
                                then_expr: Box::new(Expr::UnaryOp {
                                    op: UnaryOp::Neg,
                                    expr: Box::new(e.clone()),
                                }),
                                else_expr: Box::new(e),
                            })
                        } else {
                            Ok(Expr::Number(0))
                        }
                    }
                    // printf as expression (returns int) — convert args to call
                    "printf" | "puts" | "putchar" => {
                        let a: Result<Vec<Expr>, String> =
                            args.iter().map(|a| self.convert_expr(a)).collect();
                        Ok(Expr::Call { name, args: a? })
                    }
                    _ => {
                        let a: Result<Vec<Expr>, String> =
                            args.iter().map(|a| self.convert_expr(a)).collect();
                        Ok(Expr::Call { name, args: a? })
                    }
                }
            }

            CExpr::Index { array, index } => {
                let obj = self.convert_expr(array)?;
                let idx = self.convert_expr(index)?;
                Ok(Expr::Index {
                    object: Box::new(obj),
                    index: Box::new(idx),
                })
            }

            CExpr::Member { object, field } => {
                let obj = self.convert_expr(object)?;
                Ok(Expr::FieldAccess {
                    object: Box::new(obj),
                    field: field.clone(),
                })
            }

            CExpr::ArrowMember { pointer, field } => {
                let ptr = self.convert_expr(pointer)?;
                Ok(Expr::ArrowAccess {
                    pointer: Box::new(ptr),
                    field: field.clone(),
                })
            }

            CExpr::Cast {
                target_type,
                expr: inner,
            } => {
                let e = self.convert_expr(inner)?;
                let ty = self.convert_type(target_type);
                Ok(Expr::Cast {
                    target_type: ty,
                    expr: Box::new(e),
                })
            }

            CExpr::SizeofType(ty) => {
                let aty = self.convert_type(ty);
                Ok(Expr::SizeOf(Box::new(ast::SizeOfArg::Type(aty))))
            }
            CExpr::SizeofExpr(inner) => {
                let e = self.convert_expr(inner)?;
                Ok(Expr::SizeOf(Box::new(ast::SizeOfArg::Expr(e))))
            }

            CExpr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let c = self.convert_expr(condition)?;
                let t = self.convert_expr(then_expr)?;
                let e = self.convert_expr(else_expr)?;
                Ok(Expr::Ternary {
                    condition: Box::new(c),
                    then_expr: Box::new(t),
                    else_expr: Box::new(e),
                })
            }

            CExpr::AddressOf(inner) => {
                let e = self.convert_expr(inner)?;
                Ok(Expr::AddressOf(Box::new(e)))
            }

            CExpr::Deref(inner) => {
                let e = self.convert_expr(inner)?;
                Ok(Expr::Deref(Box::new(e)))
            }

            CExpr::Assign { op, target, value } => {
                // Assignment as expression: evaluate RHS, the value IS the result
                // Side effect (the actual store) is lost in pure expression context,
                // but works correctly when the Assign appears at statement level
                // via convert_expr_to_stmt → convert_assignment.
                let rhs = self.convert_expr(value)?;
                let lhs = match target.as_ref() {
                    CExpr::Identifier(_) => self.convert_expr(target)?,
                    _ => self.convert_expr(target)?,
                };
                Ok(self.apply_compound_op(op, &lhs, rhs))
            }

            CExpr::Comma(exprs) => {
                if let Some(last) = exprs.last() {
                    self.convert_expr(last)
                } else {
                    Ok(Expr::Number(0))
                }
            }

            CExpr::InitList(elements) => {
                let mut converted = Vec::new();
                for e in elements {
                    converted.push(self.convert_expr(e)?);
                }
                Ok(Expr::Array(converted))
            }
        }
    }

    #[allow(dead_code)]
    fn default_value(&self, cty: &CType) -> Expr {
        match cty {
            CType::Float | CType::Double => Expr::Float(0.0),
            CType::Pointer(_) => Expr::Nullptr,
            CType::Bool => Expr::Bool(false),
            _ => Expr::Number(0),
        }
    }
}

/// Convenience: parse C source → ADead-BIB Program in one call
/// Full pipeline: Preprocessor → Lexer → Parser → IR
pub fn compile_c_to_program(source: &str) -> Result<Program, String> {
    use super::c_lexer::CLexer;
    use super::c_parser::CParser;
    use super::c_preprocessor::CPreprocessor;

    // Phase 1: Preprocess — resolve #include, skip #define/#ifdef
    let mut preprocessor = CPreprocessor::new();
    let preprocessed = preprocessor.process(source);

    // Phase 2: Lex — tokenize preprocessed source
    let (tokens, lines) = CLexer::new(&preprocessed).tokenize();

    // Phase 3: Parse — tokens → C AST
    let unit = CParser::new(tokens, lines).parse_translation_unit()?;

    // Phase 4: Lower — C AST → ADead-BIB IR
    let mut converter = CToIR::new();
    converter.convert(&unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        let program = compile_c_to_program(
            r#"
            #include <stdio.h>
            int main() {
                printf("Hello from C!\n");
                return 0;
            }
        "#,
        )
        .unwrap();

        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
        assert!(!program.functions[0].body.is_empty());
    }

    #[test]
    fn test_no_string_duplication() {
        let program = compile_c_to_program(
            r#"
            int main() {
                printf("test\n");
                return 0;
            }
        "#,
        )
        .unwrap();

        // Should generate Println("test"), not Println("testtest")
        let body = &program.functions[0].body;
        let found_println = body
            .iter()
            .any(|s| matches!(s, Stmt::Println(Expr::String(s)) if s == "test"));
        assert!(found_println, "Expected Println(\"test\"), got: {:?}", body);
    }

    #[test]
    fn test_printf_format_specifiers() {
        let program = compile_c_to_program(
            r#"
            int main() {
                int x = 42;
                printf("value=%d\n", x);
                return 0;
            }
        "#,
        )
        .unwrap();

        // Should NOT produce Println("") for trailing newline
        let body = &program.functions[0].body;
        let has_empty_println = body
            .iter()
            .any(|s| matches!(s, Stmt::Println(Expr::String(s)) if s.is_empty()));
        assert!(
            !has_empty_println,
            "Should not have empty Println: {:?}",
            body
        );
    }

    #[test]
    fn test_vardecl_has_type() {
        let program = compile_c_to_program(
            r#"
            int main() {
                int x = 5;
                char c = 'A';
                return 0;
            }
        "#,
        )
        .unwrap();

        let body = &program.functions[0].body;
        let has_vardecl = body.iter().any(|s| matches!(s, Stmt::VarDecl { .. }));
        assert!(has_vardecl, "Expected VarDecl with type info: {:?}", body);
    }

    #[test]
    fn test_variables_and_math() {
        let program = compile_c_to_program(
            r#"
            int add(int a, int b) {
                return a + b;
            }
            int main() {
                int x = 10;
                int y = 20;
                int z = add(x, y);
                return z;
            }
        "#,
        )
        .unwrap();

        assert_eq!(program.functions.len(), 2);
        assert_eq!(program.functions[0].name, "add");
        assert_eq!(program.functions[1].name, "main");
    }

    #[test]
    fn test_control_flow() {
        let program = compile_c_to_program(
            r#"
            int main() {
                int sum = 0;
                for (int i = 0; i < 10; i++) {
                    sum += i;
                }
                if (sum > 30) {
                    return 1;
                } else {
                    return 0;
                }
            }
        "#,
        )
        .unwrap();

        assert_eq!(program.functions.len(), 1);
    }

    #[test]
    fn test_struct() {
        let program = compile_c_to_program(
            r#"
            struct Point {
                int x;
                int y;
            };
            int main() {
                return 0;
            }
        "#,
        )
        .unwrap();

        assert_eq!(program.structs.len(), 1);
        assert_eq!(program.structs[0].name, "Point");
        assert_eq!(program.structs[0].fields.len(), 2);
    }

    #[test]
    fn test_null_handling() {
        let program = compile_c_to_program(
            r#"
            int main() {
                int *p = NULL;
                return 0;
            }
        "#,
        )
        .unwrap();

        assert_eq!(program.functions.len(), 1);
    }

    #[test]
    fn test_global_var_uninitialized() {
        let program = compile_c_to_program(
            r#"
            int counter;
            int main() {
                return counter;
            }
        "#,
        )
        .unwrap();

        // Should have a VarDecl at top level for 'counter'
        let has_global = program
            .statements
            .iter()
            .any(|s| matches!(s, Stmt::VarDecl { name, .. } if name == "counter"));
        assert!(
            has_global,
            "Uninitialized global should be declared: {:?}",
            program.statements
        );
    }

    #[test]
    fn test_compound_assign_array() {
        let program = compile_c_to_program(
            r#"
            int main() {
                int arr[5];
                arr[0] = 10;
                arr[0] += 5;
                return 0;
            }
        "#,
        )
        .unwrap();

        assert_eq!(program.functions.len(), 1);
    }

    #[test]
    fn test_example_hello_c() {
        let source = std::fs::read_to_string("examples/c/hello.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(result.is_ok(), "hello.c failed: {}", result.unwrap_err());
        let prog = result.unwrap();
        assert!(prog.functions.len() > 0, "hello.c should have functions");
    }

    #[test]
    fn test_example_c_algorithms() {
        let source = std::fs::read_to_string("examples/c/c_algorithms.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_algorithms.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_bitwise() {
        let source = std::fs::read_to_string("examples/c/c_bitwise.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_bitwise.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_compression() {
        let source = std::fs::read_to_string("examples/c/c_compression.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_compression.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_crypto() {
        let source = std::fs::read_to_string("examples/c/c_crypto.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(result.is_ok(), "c_crypto.c failed: {}", result.unwrap_err());
    }

    #[test]
    fn test_example_c_database() {
        let source = std::fs::read_to_string("examples/c/c_database.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_database.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_fastos_base() {
        let source = std::fs::read_to_string("examples/c/c_fastos_base.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_fastos_base.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_fastos_complete() {
        let source = std::fs::read_to_string("examples/c/c_fastos_complete.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_fastos_complete.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_math() {
        let source = std::fs::read_to_string("examples/c/c_math.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(result.is_ok(), "c_math.c failed: {}", result.unwrap_err());
    }

    #[test]
    fn test_example_c_memory() {
        let source = std::fs::read_to_string("examples/c/c_memory.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(result.is_ok(), "c_memory.c failed: {}", result.unwrap_err());
    }

    #[test]
    fn test_example_c_network() {
        let source = std::fs::read_to_string("examples/c/c_network.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_network.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_structs() {
        let source = std::fs::read_to_string("examples/c/c_structs.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_structs.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_threading() {
        let source = std::fs::read_to_string("examples/c/c_threading.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_threading.c failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_c_stdlib_long() {
        let source = std::fs::read_to_string("examples/c/c_stdlib_long.c").unwrap();
        let result = compile_c_to_program(&source);
        assert!(
            result.is_ok(),
            "c_stdlib_long.c failed: {}",
            result.unwrap_err()
        );
        let prog = result.unwrap();
        assert!(
            prog.functions.len() > 20,
            "c_stdlib_long.c should have many functions, got {}",
            prog.functions.len()
        );
    }

    #[test]
    fn test_printf_percent_d_newline() {
        // This was the core "duplication" bug:
        // printf("Result: %d\n", x) should produce:
        //   Print("Result: "), Print(x), Println("")  ← OLD (bad)
        //   Print("Result: "), Print(x), Print("\n")   ← STILL BAD
        //   Print("Result: "), Print(x), <nothing — the \n is absorbed by string segment logic>
        let program = compile_c_to_program(
            r#"
            int main() {
                int x = 42;
                printf("Result: %d done\n", x);
                return 0;
            }
        "#,
        )
        .unwrap();

        let body = &program.functions[0].body;
        // Count total Print/Println statements (excluding VarDecl/Return)
        let print_count = body
            .iter()
            .filter(|s| matches!(s, Stmt::Print(_) | Stmt::Println(_)))
            .count();
        // Should be: Print("Result: "), Print(x), Println(" done") = 3
        assert_eq!(
            print_count, 3,
            "Expected 3 print stmts, got {}: {:?}",
            print_count, body
        );
    }

    // ================================================================
    // GCC-STYLE COMPREHENSIVE C TESTS — Full Feature Coverage
    // ================================================================
    // Inspired by GCC testsuite: verify ADead-BIB parses + compiles
    // every standard C feature correctly. Each test verifies:
    //   1. Parsing succeeds (no panics)
    //   2. IR generation produces correct structure
    //   3. Functions/structs count matches expectations
    // ================================================================

    #[test]
    fn test_example_dowhile() {
        let source = std::fs::read_to_string("examples/c/test_dowhile.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_dowhile.c failed");
        assert!(prog.functions.len() >= 1, "should have main");
    }

    #[test]
    fn test_example_switch() {
        let source = std::fs::read_to_string("examples/c/test_switch.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_switch.c failed");
        assert!(prog.functions.len() >= 2, "should have classify + main");
    }

    #[test]
    fn test_example_nested_loops() {
        let source = std::fs::read_to_string("examples/c/test_nested_loops.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_nested_loops.c failed");
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_example_pointers() {
        let source = std::fs::read_to_string("examples/c/test_pointers.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_pointers.c failed");
        assert!(prog.functions.len() >= 2, "should have increment + main");
    }

    #[test]
    fn test_example_recursion() {
        let source = std::fs::read_to_string("examples/c/test_recursion.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_recursion.c failed");
        assert!(prog.functions.len() >= 3, "should have fib + power + main");
    }

    #[test]
    fn test_example_enum() {
        let source = std::fs::read_to_string("examples/c/test_enum.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_enum.c failed");
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_example_typedef() {
        let source = std::fs::read_to_string("examples/c/test_typedef.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_typedef.c failed");
        assert!(prog.functions.len() >= 2, "should have add + main");
    }

    #[test]
    fn test_example_global_vars() {
        let source = std::fs::read_to_string("examples/c/test_global_vars.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_global_vars.c failed");
        assert!(prog.functions.len() >= 3, "should have inc + get + main");
        assert!(prog.statements.len() >= 2, "should have global vars");
    }

    #[test]
    fn test_example_cast() {
        let source = std::fs::read_to_string("examples/c/test_cast.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_cast.c failed");
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_example_sizeof() {
        let source = std::fs::read_to_string("examples/c/test_sizeof.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_sizeof.c failed");
        assert!(prog.structs.len() >= 1, "should have struct Pair");
    }

    #[test]
    fn test_example_multivar_decl() {
        let source = std::fs::read_to_string("examples/c/test_multivar_decl.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_multivar_decl.c failed");
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_example_compound_assign() {
        let source = std::fs::read_to_string("examples/c/test_compound_assign.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_compound_assign.c failed");
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_example_string_ops() {
        let source = std::fs::read_to_string("examples/c/test_string_ops.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_string_ops.c failed");
        assert!(prog.functions.len() >= 2, "should have my_strlen + main");
    }

    #[test]
    fn test_example_bitwise_ops() {
        let source = std::fs::read_to_string("examples/c/test_bitwise.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_bitwise.c failed");
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_example_struct_nested() {
        let source = std::fs::read_to_string("examples/c/test_struct_nested.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_struct_nested.c failed");
        assert!(prog.structs.len() >= 2, "should have Point + Rect");
    }

    #[test]
    fn test_example_array_init() {
        let source = std::fs::read_to_string("examples/c/test_array_init.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_array_init.c failed");
        assert!(prog.functions.len() >= 2, "should have sum + main");
    }

    #[test]
    fn test_example_void_func() {
        let source = std::fs::read_to_string("examples/c/test_void_func.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_void_func.c failed");
        assert!(
            prog.functions.len() >= 4,
            "should have set_x + get_x + print_separator + main"
        );
    }

    #[test]
    fn test_example_complex_expr() {
        let source = std::fs::read_to_string("examples/c/test_complex_expr.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_complex_expr.c failed");
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_example_multi_func() {
        let source = std::fs::read_to_string("examples/c/test_multi_func.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_multi_func.c failed");
        assert!(
            prog.functions.len() >= 7,
            "should have 6 helpers + apply_twice + main"
        );
    }

    #[test]
    fn test_example_increment() {
        let source = std::fs::read_to_string("examples/c/test_increment.c").unwrap();
        let prog = compile_c_to_program(&source).expect("test_increment.c failed");
        assert_eq!(prog.functions.len(), 1);
    }

    // ================================================================
    // INLINE C FEATURE TESTS — No external files needed
    // ================================================================

    #[test]
    fn test_dowhile_conversion() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int x = 0;
                do {
                    x = x + 1;
                } while (x < 10);
                return x;
            }
        "#,
        )
        .unwrap();
        let body = &prog.functions[0].body;
        // do-while converts to while(true) { body; if(!cond) break; }
        let has_while = body.iter().any(|s| matches!(s, Stmt::While { .. }));
        assert!(has_while, "do-while should convert to While: {:?}", body);
    }

    #[test]
    fn test_switch_conversion() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int x = 2;
                switch (x) {
                    case 1: return 10;
                    case 2: return 20;
                    default: return 0;
                }
            }
        "#,
        )
        .unwrap();
        let body = &prog.functions[0].body;
        // switch converts to chained if-else
        let has_if = body.iter().any(|s| matches!(s, Stmt::If { .. }));
        assert!(has_if, "switch should convert to If chain: {:?}", body);
    }

    #[test]
    fn test_for_loop_conversion() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int sum = 0;
                for (int i = 0; i < 100; i++) {
                    sum += i;
                }
                return sum;
            }
        "#,
        )
        .unwrap();
        let body = &prog.functions[0].body;
        let has_while = body.iter().any(|s| matches!(s, Stmt::While { .. }));
        assert!(has_while, "for should convert to While: {:?}", body);
    }

    #[test]
    fn test_nested_if_else() {
        let prog = compile_c_to_program(
            r#"
            int classify(int n) {
                if (n < 0) {
                    return -1;
                } else if (n == 0) {
                    return 0;
                } else if (n < 100) {
                    return 1;
                } else {
                    return 2;
                }
            }
            int main() { return classify(50); }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
    }

    #[test]
    fn test_multiple_return_paths() {
        let prog = compile_c_to_program(
            r#"
            int abs_val(int x) {
                if (x < 0) return -x;
                return x;
            }
            int main() { return abs_val(-42); }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
        assert_eq!(prog.functions[0].name, "abs_val");
    }

    #[test]
    fn test_empty_function() {
        let prog = compile_c_to_program(
            r#"
            void noop(void) {}
            int main() { noop(); return 0; }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
    }

    #[test]
    fn test_chained_comparison() {
        let prog = compile_c_to_program(
            r#"
            int in_range(int x, int lo, int hi) {
                return (x >= lo) && (x <= hi);
            }
            int main() { return in_range(5, 1, 10); }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
    }

    #[test]
    fn test_complex_for_update() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int total = 0;
                for (int i = 0; i < 20; i += 3) {
                    total += i;
                }
                return total;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_array_as_param() {
        let prog = compile_c_to_program(
            r#"
            int sum(int arr[], int n) {
                int total = 0;
                for (int i = 0; i < n; i++) {
                    total += arr[i];
                }
                return total;
            }
            int main() {
                int data[] = {1, 2, 3, 4, 5};
                return sum(data, 5);
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
    }

    #[test]
    fn test_unsigned_types() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                unsigned int a = 0xFFFFFFFF;
                unsigned char b = 255;
                unsigned short c = 65535;
                unsigned long d = 0;
                unsigned long long e = 0;
                return 0;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_const_volatile() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                const int x = 42;
                volatile int y = 0;
                const char *msg = "hello";
                return x;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_long_long_types() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                long a = 100;
                long long b = 200;
                long int c = 300;
                long long int d = 400;
                return 0;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_hex_octal_literals() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int hex = 0xFF;
                int hex2 = 0xDEAD;
                int dec = 42;
                return hex + dec;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_string_concatenation() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                printf("Hello" " " "World" "\n");
                return 0;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_comma_in_for() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int sum = 0;
                for (int i = 0; i < 10; i++) {
                    sum = sum + i;
                }
                return sum;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_break_continue() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int total = 0;
                for (int i = 0; i < 100; i++) {
                    if (i % 2 == 0) continue;
                    if (i > 10) break;
                    total += i;
                }
                return total;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_ternary_nested() {
        let prog = compile_c_to_program(
            r#"
            int clamp(int x, int lo, int hi) {
                return (x < lo) ? lo : (x > hi) ? hi : x;
            }
            int main() { return clamp(150, 0, 100); }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
    }

    #[test]
    fn test_sizeof_types() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int a = sizeof(int);
                int b = sizeof(char);
                int c = sizeof(long long);
                int x = 42;
                int d = sizeof(x);
                return a + b + c + d;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_cast_expressions() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int x = 65;
                char c = (char)x;
                int *p = (int *)0;
                long y = (long)x;
                return (int)c;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_pointer_to_pointer() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int x = 42;
                int *p = &x;
                int **pp = &p;
                return **pp;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_function_prototype() {
        let prog = compile_c_to_program(
            r#"
            int add(int a, int b);
            int add(int a, int b) {
                return a + b;
            }
            int main() { return add(3, 4); }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
    }

    #[test]
    fn test_static_extern_inline() {
        let prog = compile_c_to_program(
            r#"
            static int counter = 0;
            extern int printf(const char *fmt, ...);
            static inline int double_it(int x) { return x * 2; }
            int main() { return double_it(21); }
        "#,
        )
        .unwrap();
        assert!(prog.functions.len() >= 2);
    }

    #[test]
    fn test_enum_with_expression_values() {
        let prog = compile_c_to_program(
            r#"
            enum Sizes {
                BYTE_SIZE = 1,
                WORD_SIZE = 2,
                DWORD_SIZE = 4,
                QWORD_SIZE = 8
            };
            int main() { return QWORD_SIZE; }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_struct_with_array_field() {
        let prog = compile_c_to_program(
            r#"
            struct Buffer {
                int size;
                char data[256];
            };
            int main() {
                struct Buffer buf;
                buf.size = 10;
                return buf.size;
            }
        "#,
        )
        .unwrap();
        assert!(prog.structs.len() >= 1);
    }

    #[test]
    fn test_many_params() {
        let prog = compile_c_to_program(
            r#"
            int sum6(int a, int b, int c, int d, int e, int f) {
                return a + b + c + d + e + f;
            }
            int main() { return sum6(1, 2, 3, 4, 5, 6); }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
        assert_eq!(prog.functions[0].params.len(), 6);
    }

    #[test]
    fn test_mutual_recursion() {
        let prog = compile_c_to_program(
            r#"
            int is_even(int n);
            int is_odd(int n);
            int is_even(int n) {
                if (n == 0) return 1;
                return is_odd(n - 1);
            }
            int is_odd(int n) {
                if (n == 0) return 0;
                return is_even(n - 1);
            }
            int main() { return is_even(10); }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 3);
    }

    #[test]
    fn test_while_with_assignment() {
        let prog = compile_c_to_program(
            r#"
            int main() {
                int n = 100;
                int sum = 0;
                while (n > 0) {
                    sum += n;
                    n = n - 1;
                }
                return sum;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
    }

    #[test]
    fn test_multiple_structs() {
        let prog = compile_c_to_program(
            r#"
            struct Vec2 { int x; int y; };
            struct Vec3 { int x; int y; int z; };
            struct Color { unsigned char r; unsigned char g; unsigned char b; };
            int main() { return 0; }
        "#,
        )
        .unwrap();
        assert_eq!(prog.structs.len(), 3);
    }
}
