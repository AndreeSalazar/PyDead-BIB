// ============================================================
// ADead-BIB C++ Frontend â€” C++ AST â†’ ADead-BIB IR
// ============================================================
// Converts C++ AST to ADead-BIB IR (Program/Function/Stmt/Expr)
// Handles: classes â†’ structs, templates â†’ monomorphized, vtable elimination
//
// ADead-BIB Philosophy:
//   - vtables â†’ resolved at compile time (devirtualization)
//   - RTTI â†’ eliminated
//   - exceptions â†’ error codes
//   - templates â†’ only instantiated code survives
//
// Sin GCC. Sin LLVM. Sin Clang. Solo ADead-BIB. ðŸ’€ðŸ¦ˆ
// ============================================================

use super::cpp_ast::*;
use crate::frontend::ast::{
    BinOp, BitwiseOp as IrBitwiseOp, CmpOp, CompoundOp, Expr, Function, FunctionAttributes, Param,
    Program, ProgramAttributes, SizeOfArg, Stmt, Struct as IrStruct, StructField, SwitchCase,
    UnaryOp as IrUnaryOp,
};
use crate::frontend::types::Type;

fn flatten_namespaces(decls: Vec<CppTopLevel>, prefix: &str) -> Vec<CppTopLevel> {
    let mut flat = Vec::new();
    for decl in decls {
        match decl {
            CppTopLevel::Namespace { name, declarations } => {
                let new_prefix = if prefix.is_empty() { name.clone() } else { format!("{}::{}", prefix, name) };
                flat.extend(flatten_namespaces(declarations, &new_prefix));
            }
            CppTopLevel::ClassDef { name, template_params, bases, members, is_struct } => {
                let new_name = if prefix.is_empty() { name } else { format!("{}::{}", prefix, name) };
                flat.push(CppTopLevel::ClassDef {
                    name: new_name,
                    template_params,
                    bases,
                    members,
                    is_struct,
                });
            }
            CppTopLevel::FunctionDef { return_type, name, template_params, params, qualifiers, body } => {
                let new_name = if prefix.is_empty() { name } else { format!("{}::{}", prefix, name) };
                flat.push(CppTopLevel::FunctionDef {
                    return_type,
                    name: new_name,
                    template_params,
                    params,
                    qualifiers,
                    body,
                });
            }
            CppTopLevel::FunctionDecl { return_type, name, template_params, params, qualifiers } => {
                let new_name = if prefix.is_empty() { name } else { format!("{}::{}", prefix, name) };
                flat.push(CppTopLevel::FunctionDecl {
                    return_type,
                    name: new_name,
                    template_params,
                    params,
                    qualifiers,
                });
            }
            CppTopLevel::EnumDef { name, is_class, underlying_type, values } => {
                let new_name = if prefix.is_empty() { name } else { format!("{}::{}", prefix, name) };
                flat.push(CppTopLevel::EnumDef {
                    name: new_name,
                    is_class,
                    underlying_type,
                    values,
                });
            }
            CppTopLevel::GlobalVar { type_spec, declarators } => {
                let new_declarators = declarators.into_iter().map(|mut d| {
                    if !prefix.is_empty() {
                        d.name = format!("{}::{}", prefix, d.name);
                    }
                    d
                }).collect();
                flat.push(CppTopLevel::GlobalVar {
                    type_spec,
                    declarators: new_declarators,
                });
            }
            CppTopLevel::TypeAlias { new_name, original, template_params } => {
                let new_new_name = if prefix.is_empty() { new_name } else { format!("{}::{}", prefix, new_name) };
                flat.push(CppTopLevel::TypeAlias {
                    new_name: new_new_name,
                    original,
                    template_params,
                });
            }
            other => flat.push(other),
        }
    }
    flat
}

pub struct CppToIR {
    type_aliases: Vec<(String, CppType)>,
    class_methods: Vec<(String, String, Vec<CppParam>, CppType)>, // (class, method, params, ret)
    current_namespace: Option<String>, // Track current namespace for unqualified calls
    namespace_functions: Vec<String>,  // All function names in current namespace
    current_class: Option<String>,     // Track current class for this-> resolution
    class_fields: Vec<(String, Vec<String>)>, // (class_name, field_names)
    /// variable_name â†’ class_name for stack-allocated objects in current scope
    variable_types: Vec<(String, String)>,
    /// (class_name, [(field_name, field_index)]) for flat-slot field access
    class_field_order: Vec<(String, Vec<String>)>,
    /// (class_name, ctor_params, [(field_name, init_expr)], body_stmts) for all ctors
    class_ctor_inits: Vec<(String, Vec<String>, Vec<(String, CppExpr)>, Vec<CppStmt>)>,
    /// (class_name, method_name, param_names, body_stmts) for inline expansion
    class_method_bodies: Vec<(String, String, Vec<String>, Vec<CppStmt>)>,
    /// func_name → vec of bools indicating which params are references
    func_ref_params: Vec<(String, Vec<bool>)>,
    /// Classes that have array-typed fields (skip inline ctor expansion)
    classes_with_array_fields: Vec<String>,
    /// (class_name, field_name) → CppType for accurate field type lookup
    class_field_type_map: Vec<(String, String, CppType)>,
}

impl CppToIR {
    pub fn new() -> Self {
        Self {
            type_aliases: Vec::new(),
            class_methods: Vec::new(),
            current_namespace: None,
            namespace_functions: Vec::new(),
            current_class: None,
            class_fields: Vec::new(),
            variable_types: Vec::new(),
            class_field_order: Vec::new(),
            class_ctor_inits: Vec::new(),
            class_method_bodies: Vec::new(),
            func_ref_params: Vec::new(),
            classes_with_array_fields: Vec::new(),
            class_field_type_map: Vec::new(),
        }
    }

    // ---- Helper: look up which class a variable belongs to ----
    fn class_for_var(&self, var_name: &str) -> Option<String> {
        for (vn, cn) in &self.variable_types {
            if vn == var_name {
                return Some(cn.clone());
            }
        }
        None
    }

    // ---- Helper: field index in a class (for flat slot offset) ----
    fn field_index_in_class(&self, class_name: &str, field_name: &str) -> Option<usize> {
        // Check class_field_order, then class_fields
        for (cn, fields) in &self.class_field_order {
            if cn == class_name {
                return fields.iter().position(|f| f == field_name);
            }
        }
        for (cn, fields) in &self.class_fields {
            if cn == class_name {
                return fields.iter().position(|f| f == field_name);
            }
        }
        None
    }

    /// Check if an identifier is a field of the current class
    fn is_class_field(&self, name: &str) -> bool {
        if let Some(ref class_name) = self.current_class {
            for (cn, fields) in &self.class_fields {
                if cn == class_name && fields.contains(&name.to_string()) {
                    return true;
                }
            }
        }
        false
    }

    /// Substitute a parameter name with its actual argument expression in a CppExpr
    fn subst_param(expr: CppExpr, param: &str, arg: &CppExpr) -> CppExpr {
        match expr {
            CppExpr::Identifier(ref name) if name == param => arg.clone(),
            CppExpr::BinaryOp { op, left, right } => CppExpr::BinaryOp {
                op,
                left: Box::new(Self::subst_param(*left, param, arg)),
                right: Box::new(Self::subst_param(*right, param, arg)),
            },
            CppExpr::UnaryOp {
                op,
                expr: inner,
                is_prefix,
            } => CppExpr::UnaryOp {
                op,
                expr: Box::new(Self::subst_param(*inner, param, arg)),
                is_prefix,
            },
            CppExpr::Call { callee, args } => CppExpr::Call {
                callee: Box::new(Self::subst_param(*callee, param, arg)),
                args: args
                    .into_iter()
                    .map(|a| Self::subst_param(a, param, arg))
                    .collect(),
            },
            CppExpr::MemberAccess { object, member } => CppExpr::MemberAccess {
                object: Box::new(Self::subst_param(*object, param, arg)),
                member,
            },
            CppExpr::ArrowAccess { pointer, member } => CppExpr::ArrowAccess {
                pointer: Box::new(Self::subst_param(*pointer, param, arg)),
                member,
            },
            CppExpr::Index { object, index } => CppExpr::Index {
                object: Box::new(Self::subst_param(*object, param, arg)),
                index: Box::new(Self::subst_param(*index, param, arg)),
            },
            CppExpr::Assign { target, value } => CppExpr::Assign {
                target: Box::new(Self::subst_param(*target, param, arg)),
                value: Box::new(Self::subst_param(*value, param, arg)),
            },
            CppExpr::CompoundAssign { target, op, value } => CppExpr::CompoundAssign {
                target: Box::new(Self::subst_param(*target, param, arg)),
                op,
                value: Box::new(Self::subst_param(*value, param, arg)),
            },
            CppExpr::Cast {
                cast_type,
                target_type,
                expr: inner,
            } => CppExpr::Cast {
                cast_type,
                target_type,
                expr: Box::new(Self::subst_param(*inner, param, arg)),
            },
            other => other,
        }
    }

    /// Substitute 'this->field' with obj_name.field in a CppExpr (for method inlining)
    fn subst_this_in_expr(
        expr: CppExpr,
        obj_name: &str,
        class_fields: &[(String, Vec<String>)],
        class_name: &str,
    ) -> CppExpr {
        match expr {
            // this->field  OR  this.field → obj_name.field (as flat Identifier)
            CppExpr::ArrowAccess {
                ref pointer,
                ref member,
            }
            | CppExpr::MemberAccess {
                object: ref pointer,
                member: ref member,
            } => {
                if let CppExpr::Identifier(ref n) = pointer.as_ref() {
                    if n == "this" {
                        return CppExpr::Identifier(format!("{}.{}", obj_name, member));
                    }
                }
                // Recurse into pointer and member normally
                match expr {
                    CppExpr::ArrowAccess {
                        pointer: p,
                        member: m,
                    } => CppExpr::ArrowAccess {
                        pointer: Box::new(Self::subst_this_in_expr(
                            *p,
                            obj_name,
                            class_fields,
                            class_name,
                        )),
                        member: m,
                    },
                    CppExpr::MemberAccess {
                        object: p,
                        member: m,
                    } => CppExpr::MemberAccess {
                        object: Box::new(Self::subst_this_in_expr(
                            *p,
                            obj_name,
                            class_fields,
                            class_name,
                        )),
                        member: m,
                    },
                    other => other,
                }
            }
            // Bare field identifier (inside method, is_class_field style) → obj_name.field
            CppExpr::Identifier(ref name) => {
                let is_field = class_fields
                    .iter()
                    .find(|(cn, _)| cn == class_name)
                    .map(|(_, fields)| fields.contains(&name.to_string()))
                    .unwrap_or(false);
                if is_field {
                    CppExpr::Identifier(format!("{}.{}", obj_name, name))
                } else {
                    expr
                }
            }
            CppExpr::BinaryOp { op, left, right } => CppExpr::BinaryOp {
                op,
                left: Box::new(Self::subst_this_in_expr(
                    *left,
                    obj_name,
                    class_fields,
                    class_name,
                )),
                right: Box::new(Self::subst_this_in_expr(
                    *right,
                    obj_name,
                    class_fields,
                    class_name,
                )),
            },
            CppExpr::UnaryOp {
                op,
                expr: inner,
                is_prefix,
            } => CppExpr::UnaryOp {
                op,
                is_prefix,
                expr: Box::new(Self::subst_this_in_expr(
                    *inner,
                    obj_name,
                    class_fields,
                    class_name,
                )),
            },
            CppExpr::Call { callee, args } => CppExpr::Call {
                callee,
                args: args
                    .into_iter()
                    .map(|a| Self::subst_this_in_expr(a, obj_name, class_fields, class_name))
                    .collect(),
            },
            CppExpr::Assign { target, value } => CppExpr::Assign {
                target: Box::new(Self::subst_this_in_expr(
                    *target,
                    obj_name,
                    class_fields,
                    class_name,
                )),
                value: Box::new(Self::subst_this_in_expr(
                    *value,
                    obj_name,
                    class_fields,
                    class_name,
                )),
            },
            CppExpr::CompoundAssign { target, op, value } => CppExpr::CompoundAssign {
                target: Box::new(Self::subst_this_in_expr(
                    *target,
                    obj_name,
                    class_fields,
                    class_name,
                )),
                op,
                value: Box::new(Self::subst_this_in_expr(
                    *value,
                    obj_name,
                    class_fields,
                    class_name,
                )),
            },
            CppExpr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => CppExpr::Ternary {
                condition: Box::new(Self::subst_this_in_expr(
                    *condition,
                    obj_name,
                    class_fields,
                    class_name,
                )),
                then_expr: Box::new(Self::subst_this_in_expr(
                    *then_expr,
                    obj_name,
                    class_fields,
                    class_name,
                )),
                else_expr: Box::new(Self::subst_this_in_expr(
                    *else_expr,
                    obj_name,
                    class_fields,
                    class_name,
                )),
            },
            CppExpr::Index { object, index } => CppExpr::Index {
                object: Box::new(Self::subst_this_in_expr(
                    *object,
                    obj_name,
                    class_fields,
                    class_name,
                )),
                index: Box::new(Self::subst_this_in_expr(
                    *index,
                    obj_name,
                    class_fields,
                    class_name,
                )),
            },
            CppExpr::Cast {
                cast_type,
                target_type,
                expr: inner,
            } => CppExpr::Cast {
                cast_type,
                target_type,
                expr: Box::new(Self::subst_this_in_expr(
                    *inner,
                    obj_name,
                    class_fields,
                    class_name,
                )),
            },
            other => other,
        }
    }

    fn subst_this_in_stmt(
        stmt: CppStmt,
        obj_name: &str,
        class_fields: &[(String, Vec<String>)],
        class_name: &str,
    ) -> CppStmt {
        let sub = |e: CppExpr| Self::subst_this_in_expr(e, obj_name, class_fields, class_name);
        match stmt {
            CppStmt::Expr(e) => CppStmt::Expr(sub(e)),
            CppStmt::Return(Some(e)) => CppStmt::Return(Some(sub(e))),
            CppStmt::Return(None) => CppStmt::Return(None),
            CppStmt::VarDecl {
                type_spec,
                declarators,
            } => {
                let ds = declarators
                    .into_iter()
                    .map(|mut d| {
                        d.initializer = d.initializer.map(sub);
                        d
                    })
                    .collect();
                CppStmt::VarDecl {
                    type_spec,
                    declarators: ds,
                }
            }
            CppStmt::If {
                init,
                condition,
                then_body,
                else_body,
                is_constexpr,
            } => CppStmt::If {
                init: init.map(|i| {
                    Box::new(Self::subst_this_in_stmt(
                        *i,
                        obj_name,
                        class_fields,
                        class_name,
                    ))
                }),
                condition: sub(condition),
                then_body: Box::new(Self::subst_this_in_stmt(
                    *then_body,
                    obj_name,
                    class_fields,
                    class_name,
                )),
                else_body: else_body.map(|eb| {
                    Box::new(Self::subst_this_in_stmt(
                        *eb,
                        obj_name,
                        class_fields,
                        class_name,
                    ))
                }),
                is_constexpr,
            },
            CppStmt::While { condition, body } => CppStmt::While {
                condition: sub(condition),
                body: Box::new(Self::subst_this_in_stmt(
                    *body,
                    obj_name,
                    class_fields,
                    class_name,
                )),
            },
            CppStmt::Block(stmts) => CppStmt::Block(
                stmts
                    .into_iter()
                    .map(|s| Self::subst_this_in_stmt(s, obj_name, class_fields, class_name))
                    .collect(),
            ),
            other => other,
        }
    }

    /// Apply subst_param to all expressions within a CppStmt
    fn subst_param_in_stmt(stmt: CppStmt, param: &str, arg: &CppExpr) -> CppStmt {
        let sub = |e: CppExpr| Self::subst_param(e, param, arg);
        match stmt {
            CppStmt::Expr(e) => CppStmt::Expr(sub(e)),
            CppStmt::Return(Some(e)) => CppStmt::Return(Some(sub(e))),
            CppStmt::Return(None) => CppStmt::Return(None),
            CppStmt::VarDecl {
                type_spec,
                declarators,
            } => {
                let ds = declarators
                    .into_iter()
                    .map(|mut d| {
                        d.initializer = d.initializer.map(|e| sub(e));
                        d
                    })
                    .collect();
                CppStmt::VarDecl {
                    type_spec,
                    declarators: ds,
                }
            }
            CppStmt::If {
                init,
                condition,
                then_body,
                else_body,
                is_constexpr,
            } => CppStmt::If {
                init: init.map(|i| Box::new(Self::subst_param_in_stmt(*i, param, arg))),
                condition: sub(condition),
                then_body: Box::new(Self::subst_param_in_stmt(*then_body, param, arg)),
                else_body: else_body.map(|eb| Box::new(Self::subst_param_in_stmt(*eb, param, arg))),
                is_constexpr,
            },
            CppStmt::While { condition, body } => CppStmt::While {
                condition: sub(condition),
                body: Box::new(Self::subst_param_in_stmt(*body, param, arg)),
            },
            CppStmt::Block(stmts) => CppStmt::Block(
                stmts
                    .into_iter()
                    .map(|s| Self::subst_param_in_stmt(s, param, arg))
                    .collect(),
            ),
            other => other,
        }
    }

    pub fn convert(&mut self, unit: &CppTranslationUnit) -> Result<Program, String> {
        let mut program = Program::new();
        program.attributes = ProgramAttributes::default();

        let flat_decls = flatten_namespaces(unit.declarations.clone(), "");

        // First pass: collect type aliases, class info, field names, and ctor inits
        for decl in &flat_decls {
            match decl {
                CppTopLevel::TypeAlias {
                    new_name, original, ..
                } => {
                    self.type_aliases.push((new_name.clone(), original.clone()));
                }
                CppTopLevel::ClassDef {
                    name,
                    members,
                    bases,
                    ..
                } => {
                    let mut field_names = Vec::new();

                    // Collect base class fields first (for inheritance flat layout)
                    for base in bases {
                        for (cn, fields) in &self.class_field_order {
                            if cn == &base.name {
                                for f in fields {
                                    if !field_names.contains(f) {
                                        field_names.push(f.clone());
                                    }
                                }
                            }
                        }
                    }

                    for member in members {
                        match member {
                            CppClassMember::Method {
                                name: method_name,
                                params,
                                return_type,
                                body,
                                ..
                            } => {
                                self.class_methods.push((
                                    name.clone(),
                                    method_name.clone(),
                                    params.clone(),
                                    return_type.clone(),
                                ));
                                // Also store body for inline expansion
                                if let Some(method_body) = body {
                                    let param_names: Vec<String> =
                                        params.iter().filter_map(|p| p.name.clone()).collect();
                                    self.class_method_bodies.push((
                                        name.clone(),
                                        method_name.clone(),
                                        param_names,
                                        method_body.clone(),
                                    ));
                                }
                            }
                            CppClassMember::Field {
                                name: field_name,
                                type_spec,
                                ..
                            } => {
                                field_names.push(field_name.clone());
                                // Track field type for accurate inline ctor expansion
                                self.class_field_type_map.push((
                                    name.clone(),
                                    field_name.clone(),
                                    type_spec.clone(),
                                ));
                                // Track if this class has array-typed fields
                                if matches!(type_spec, CppType::Array(_, _)) {
                                    if !self.classes_with_array_fields.contains(name) {
                                        self.classes_with_array_fields.push(name.clone());
                                    }
                                }
                            }
                            CppClassMember::Constructor {
                                params,
                                initializer_list,
                                body,
                                ..
                            } => {
                                // Collect ALL ctors (both default and parameterized) for inlining
                                let param_names: Vec<String> =
                                    params.iter().filter_map(|p| p.name.clone()).collect();
                                let body_stmts = body.clone().unwrap_or_default();
                                self.class_ctor_inits.push((
                                    name.clone(),
                                    param_names,
                                    initializer_list.clone(),
                                    body_stmts,
                                ));
                            }
                            _ => {}
                        }
                    }
                    // Prepend base class fields so derived class has full flat layout
                    // e.g., Circle → Shape's ['id'] + Circle's ['radius'] = ['id', 'radius']
                    let mut all_field_names = Vec::new();
                    for base in bases.iter() {
                        let base_fields: Vec<String> = self
                            .class_fields
                            .iter()
                            .find(|(cn, _)| cn == &base.name)
                            .map(|(_, f)| f.clone())
                            .unwrap_or_default();
                        for bf in base_fields {
                            if !all_field_names.contains(&bf) {
                                all_field_names.push(bf);
                            }
                        }
                    }
                    for f in &field_names {
                        if !all_field_names.contains(f) {
                            all_field_names.push(f.clone());
                        }
                    }
                    self.class_fields
                        .push((name.clone(), all_field_names.clone()));
                    self.class_field_order.push((name.clone(), all_field_names));

                    // Propagate inherited method bodies from base classes to this derived class
                    // (done after pushing current class so that base methods can be found)
                    // NOTE: this needs to be a second loop after all classes are processed;
                    // we'll do the propagation right here only for already-processed bases.
                    for base in bases.iter() {
                        let inherited: Vec<(String, String, Vec<String>, Vec<CppStmt>)> = self
                            .class_method_bodies
                            .iter()
                            .filter(|(cn, _, _, _)| cn == &base.name)
                            .map(|(_, mn, params, body)| {
                                (name.clone(), mn.clone(), params.clone(), body.clone())
                            })
                            .collect();
                        for entry in inherited {
                            // Only add if derived class doesn't already define this method
                            if !self
                                .class_method_bodies
                                .iter()
                                .any(|(cn, mn, _, _)| cn == &entry.0 && mn == &entry.1)
                            {
                                self.class_method_bodies.push(entry);
                            }
                        }
                    }
                }
                // Template specialization — collect fields like ClassDef
                CppTopLevel::TemplateSpecialization {
                    name, members, ..
                } => {
                    let mut field_names = Vec::new();
                    for member in members {
                        match member {
                            CppClassMember::Field { name: field_name, type_spec, .. } => {
                                field_names.push(field_name.clone());
                                self.class_field_type_map.push((name.clone(), field_name.clone(), type_spec.clone()));
                                if matches!(type_spec, CppType::Array(_, _)) {
                                    if !self.classes_with_array_fields.contains(name) {
                                        self.classes_with_array_fields.push(name.clone());
                                    }
                                }
                            }
                            CppClassMember::Method { name: method_name, params, return_type, body, .. } => {
                                self.class_methods.push((name.clone(), method_name.clone(), params.clone(), return_type.clone()));
                                if let Some(method_body) = body {
                                    let param_names: Vec<String> = params.iter().filter_map(|p| p.name.clone()).collect();
                                    self.class_method_bodies.push((name.clone(), method_name.clone(), param_names, method_body.clone()));
                                }
                            }
                            CppClassMember::Constructor { params, initializer_list, body, .. } => {
                                let param_names: Vec<String> = params.iter().filter_map(|p| p.name.clone()).collect();
                                let body_stmts = body.clone().unwrap_or_default();
                                self.class_ctor_inits.push((name.clone(), param_names, initializer_list.clone(), body_stmts));
                            }
                            _ => {}
                        }
                    }
                    self.class_fields.push((name.clone(), field_names.clone()));
                    self.class_field_order.push((name.clone(), field_names));
                }
                CppTopLevel::FunctionDef { name, params, .. } => {
                    let ref_flags: Vec<bool> = params
                        .iter()
                        .map(|p| match &p.param_type {
                            CppType::Reference(_) => true,
                            CppType::Const(inner) => {
                                matches!(inner.as_ref(), CppType::Reference(_))
                            }
                            _ => false,
                        })
                        .collect();
                    if ref_flags.iter().any(|&r| r) {
                        self.func_ref_params.push((name.clone(), ref_flags));
                    }
                }
                _ => {}
            }
        }

        // Second pass: convert declarations
        for decl in &flat_decls {
            match decl {
                CppTopLevel::FunctionDef {
                    return_type,
                    name,
                    params,
                    body,
                    ..
                } => {
                    let func = self.convert_function(return_type, name, params, body)?;
                    program.functions.push(func);
                }
                CppTopLevel::ClassDef {
                    name,
                    members,
                    bases,
                    ..
                } => {
                    // Convert class to struct + methods as functions
                    let ir_struct = self.convert_class_to_struct(name, members, bases)?;
                    program.structs.push(ir_struct);

                    // Convert methods to standalone functions
                    for member in members {
                        match member {
                            CppClassMember::Method {
                                name: method_name,
                                return_type,
                                params,
                                body: Some(body),
                                ..
                            } => {
                                let func_name = format!("{}::{}", name, method_name);
                                let func = self.convert_method(
                                    return_type,
                                    &func_name,
                                    name,
                                    params,
                                    body,
                                )?;
                                program.functions.push(func);
                            }
                            CppClassMember::Constructor {
                                params,
                                body: Some(body),
                                initializer_list,
                                ..
                            } => {
                                let _func_name = format!("{}::{}", name, name);
                                let func =
                                    self.convert_constructor(name, params, initializer_list, body)?;
                                program.functions.push(func);
                            }
                            CppClassMember::Destructor {
                                body: Some(body), ..
                            } => {
                                let func_name = format!("{}::~{}", name, name);
                                let func =
                                    self.convert_function(&CppType::Void, &func_name, &[], body)?;
                                program.functions.push(func);
                            }
                            _ => {}
                        }
                    }
                }
                // Namespace flattening is now handled by flatten_namespaces
                CppTopLevel::Namespace { .. } => {}
                CppTopLevel::EnumDef {
                    name: _, values, ..
                } => {
                    // Enum constants become global assignments
                    for (i, (ident, val)) in values.iter().enumerate() {
                        let value = if let Some(expr) = val {
                            self.convert_expr(expr)?
                        } else {
                            Expr::Number(i as i64)
                        };
                        program.statements.push(Stmt::VarDecl {
                            var_type: Type::I32,
                            name: ident.clone(),
                            value: Some(value),
                        });
                    }
                }
                CppTopLevel::GlobalVar {
                    type_spec,
                    declarators,
                } => {
                    for d in declarators {
                        let var_type = self.convert_type(type_spec);
                        let init = if let Some(ref e) = d.initializer {
                            Some(self.convert_expr(e)?)
                        } else {
                            None
                        };
                        program.statements.push(Stmt::VarDecl {
                            var_type,
                            name: d.name.clone(),
                            value: init,
                        });
                    }
                }
                // Template class specialization → treat like ClassDef
                CppTopLevel::TemplateSpecialization {
                    name,
                    members,
                    ..
                } => {
                    let ir_struct = self.convert_class_to_struct(name, members, &[])?;
                    program.structs.push(ir_struct);

                    for member in members {
                        match member {
                            CppClassMember::Method {
                                name: method_name,
                                return_type,
                                params,
                                body: Some(body),
                                ..
                            } => {
                                let func_name = format!("{}::{}", name, method_name);
                                let func = self.convert_method(
                                    return_type,
                                    &func_name,
                                    name,
                                    params,
                                    body,
                                )?;
                                program.functions.push(func);
                            }
                            CppClassMember::Constructor {
                                params,
                                body: Some(body),
                                initializer_list,
                                ..
                            } => {
                                let func =
                                    self.convert_constructor(name, params, initializer_list, body)?;
                                program.functions.push(func);
                            }
                            _ => {}
                        }
                    }
                }
                // Template function specialization → treat like FunctionDef
                CppTopLevel::TemplateFuncSpecialization {
                    name,
                    return_type,
                    params,
                    body,
                    ..
                } => {
                    let func = self.convert_function(return_type, name, params, body)?;
                    program.functions.push(func);
                }
                CppTopLevel::FunctionDecl { .. } => {} // Prototypes â€” skip
                CppTopLevel::UsingNamespace(_) => {}
                CppTopLevel::ExternC { declarations } => {
                    for inner in declarations {
                        if let CppTopLevel::FunctionDef {
                            return_type,
                            name,
                            params,
                            body,
                            ..
                        } = inner
                        {
                            let func = self.convert_function(return_type, name, params, body)?;
                            program.functions.push(func);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(program)
    }

    // ========== Type conversion ==========

    fn convert_type(&self, cpp_type: &CppType) -> Type {
        match cpp_type {
            CppType::Void => Type::Void,
            CppType::Bool => Type::Bool,
            CppType::Char | CppType::Char8 => Type::I8,
            CppType::WChar | CppType::Char16 => Type::I16,
            CppType::Char32 => Type::I32,
            CppType::Short => Type::I16,
            CppType::Int => Type::I32,
            CppType::Long => Type::I64,
            CppType::LongLong => Type::I64,
            CppType::Float => Type::F32,
            CppType::Double | CppType::LongDouble => Type::F64,
            CppType::Auto => Type::Auto,
            CppType::Unsigned(inner) => match inner.as_ref() {
                CppType::Char => Type::U8,
                CppType::Short => Type::U16,
                CppType::Int => Type::U32,
                CppType::Long | CppType::LongLong => Type::U64,
                _ => Type::U32,
            },
            CppType::Signed(inner) => self.convert_type(inner),
            CppType::Const(inner)
            | CppType::Volatile(inner)
            | CppType::Mutable(inner)
            | CppType::Constexpr(inner) => self.convert_type(inner),
            CppType::Pointer(inner) => Type::Pointer(Box::new(self.convert_type(inner))),
            CppType::Reference(inner) | CppType::RValueRef(inner) => {
                Type::Reference(Box::new(self.convert_type(inner)))
            }
            CppType::Array(inner, size) => Type::Array(Box::new(self.convert_type(inner)), *size),
            CppType::Named(name) | CppType::Class(name) | CppType::Struct(name) => {
                Type::Named(name.clone())
            }
            CppType::Enum(_) => Type::I32,
            CppType::StdString => Type::Str,
            CppType::StdStringView => Type::Str,
            CppType::StdVector(inner)
            | CppType::StdList(inner)
            | CppType::StdForwardList(inner)
            | CppType::StdDeque(inner)
            | CppType::StdSet(inner)
            | CppType::StdUnorderedSet(inner)
            | CppType::StdStack(inner)
            | CppType::StdQueue(inner)
            | CppType::StdPriorityQueue(inner)
            | CppType::StdSpan(inner)
            | CppType::StdInitializerList(inner) => {
                Type::Array(Box::new(self.convert_type(inner)), None)
            }
            CppType::StdArray(inner, size) => {
                Type::Array(Box::new(self.convert_type(inner)), Some(*size))
            }
            CppType::StdMap(k, v) | CppType::StdUnorderedMap(k, v) => {
                Type::Named(format!("map<{:?},{:?}>", k, v))
            }
            CppType::UniquePtr(inner) | CppType::SharedPtr(inner) | CppType::WeakPtr(inner) => {
                Type::Pointer(Box::new(self.convert_type(inner)))
            }
            CppType::StdOptional(inner) => self.convert_type(inner),
            CppType::StdTuple(args) => {
                Type::Named(format!("tuple<{}>", args.len()))
            }
            CppType::StdVariant(args) => {
                Type::Named(format!("variant<{}>", args.len()))
            }
            CppType::StdAny => Type::Named("any".to_string()),
            CppType::StdThread => Type::Named("thread".to_string()),
            CppType::StdMutex => Type::Named("mutex".to_string()),
            CppType::StdAtomic(inner) => self.convert_type(inner),
            CppType::StdFuture(inner) | CppType::StdPromise(inner) => {
                self.convert_type(inner)
            }
            CppType::StdRegex => Type::Named("regex".to_string()),
            CppType::StdFilesystemPath => Type::Named("path".to_string()),
            CppType::SizeT => Type::U64,
            CppType::Nullptr => Type::Pointer(Box::new(Type::Void)),
            CppType::TemplateType { name, args } => {
                if args.len() == 1 {
                    Type::Array(Box::new(self.convert_type(&args[0])), None)
                } else {
                    Type::Named(name.clone())
                }
            }
            _ => Type::I64,
        }
    }

    // ========== Class â†’ Struct ==========

    fn convert_class_to_struct(
        &self,
        name: &str,
        members: &[CppClassMember],
        _bases: &[CppBaseClass],
    ) -> Result<IrStruct, String> {
        let mut fields = Vec::new();
        for member in members {
            if let CppClassMember::Field {
                type_spec,
                name: field_name,
                ..
            } = member
            {
                fields.push(StructField {
                    name: field_name.clone(),
                    field_type: self.convert_type(type_spec),
                });
            }
        }
        Ok(IrStruct {
            name: name.to_string(),
            fields,
            is_packed: false,
        })
    }

    // ========== Function conversion ==========

    fn convert_function(
        &mut self,
        ret_type: &CppType,
        name: &str,
        params: &[CppParam],
        body: &[CppStmt],
    ) -> Result<Function, String> {
        let ir_params: Vec<Param> = params
            .iter()
            .map(|p| Param {
                name: p.name.clone().unwrap_or_else(|| "unnamed".to_string()),
                param_type: self.convert_type(&p.param_type),
                default_value: None,
            })
            .collect();

        let mut ir_body = Vec::new();
        for stmt in body {
            ir_body.extend(self.convert_stmt(stmt)?);
        }

        Ok(Function {
            name: name.to_string(),
            params: ir_params,
            body: ir_body,
            return_type: None,
            resolved_return_type: self.convert_type(ret_type),
            attributes: FunctionAttributes::default(),
        })
    }

    fn convert_method(
        &mut self,
        ret_type: &CppType,
        func_name: &str,
        class_name: &str,
        params: &[CppParam],
        body: &[CppStmt],
    ) -> Result<Function, String> {
        // Set current class for this-> resolution
        self.current_class = Some(class_name.to_string());

        // Add implicit 'this' pointer as first param
        let mut all_params = vec![CppParam {
            param_type: CppType::Pointer(Box::new(CppType::Named(class_name.to_string()))),
            name: Some("this".to_string()),
            default_value: None,
            is_variadic: false,
        }];
        all_params.extend_from_slice(params);
        let result = self.convert_function(ret_type, func_name, &all_params, body);

        self.current_class = None;
        result
    }

    fn convert_constructor(
        &mut self,
        class_name: &str,
        params: &[CppParam],
        init_list: &[(String, CppExpr)],
        body: &[CppStmt],
    ) -> Result<Function, String> {
        // Set current class for this-> resolution
        self.current_class = Some(class_name.to_string());

        let func_name = format!("{}::{}", class_name, class_name);

        // Add implicit 'this' pointer as first param (like methods)
        let mut all_params = vec![CppParam {
            param_type: CppType::Pointer(Box::new(CppType::Named(class_name.to_string()))),
            name: Some("this".to_string()),
            default_value: None,
            is_variadic: false,
        }];
        all_params.extend_from_slice(params);

        // Convert initializer list to field assignments at start of body
        let mut full_body = Vec::new();
        for (field, expr) in init_list {
            full_body.push(CppStmt::Expr(CppExpr::Assign {
                target: Box::new(CppExpr::MemberAccess {
                    object: Box::new(CppExpr::Identifier("this".to_string())),
                    member: field.clone(),
                }),
                value: Box::new(expr.clone()),
            }));
        }
        full_body.extend_from_slice(body);

        let result = self.convert_function(&CppType::Void, &func_name, &all_params, &full_body);
        self.current_class = None;
        result
    }

    // ========== Statement conversion ==========

    fn convert_stmt(&mut self, stmt: &CppStmt) -> Result<Vec<Stmt>, String> {
        match stmt {
            CppStmt::LineMarker(l) => Ok(vec![Stmt::LineMarker(*l)]),
            CppStmt::Expr(expr) => {
                // ── Inline method expansion ────────────────────────────────────
                // Peek at expr as a reference to see if it's a method call on a
                // known class. If so, inline-expand the body (substituting this→obj)
                // and return early. expr is NOT moved here.
                let inline_stmts: Option<Vec<Stmt>> = {
                    if let CppExpr::Call { callee, args } = expr {
                        if let CppExpr::MemberAccess { object, member } = callee.as_ref() {
                            if let CppExpr::Identifier(obj_name) = object.as_ref() {
                                if let Some(class_name) = self.class_for_var(obj_name) {
                                    let method_data = self
                                        .class_method_bodies
                                        .iter()
                                        .find(|(cn, mn, _, _)| {
                                            cn == &class_name && mn == member.as_str()
                                        })
                                        .map(|(_, _, params, body)| (params.clone(), body.clone()));

                                    if let Some((method_params, method_body)) = method_data {
                                        // Skip inlining methods that contain Return statements,
                                        // since inlining a Return would emit `ret` inside the caller,
                                        // corrupting control flow. Let these go through function call path.
                                        fn has_return(stmts: &[CppStmt]) -> bool {
                                            stmts.iter().any(|s| match s {
                                                CppStmt::Return(Some(_)) => true,
                                                CppStmt::If {
                                                    then_body,
                                                    else_body,
                                                    ..
                                                } => {
                                                    has_return(&[*then_body.clone()])
                                                        || else_body
                                                            .as_ref()
                                                            .map(|eb| has_return(&[*eb.clone()]))
                                                            .unwrap_or(false)
                                                }
                                                CppStmt::Block(inner) => has_return(inner),
                                                _ => false,
                                            })
                                        }
                                        if has_return(&method_body) {
                                            // Fall through to normal function call path
                                            None
                                        } else {
                                            let class_fields_clone = self.class_fields.clone();
                                            let obj_name_clone = obj_name.clone();
                                            let args_clone: Vec<CppExpr> = args.clone();

                                            let substituted: Vec<CppStmt> = method_body
                                                .into_iter()
                                                .map(|s| {
                                                    let mut s2 = s;
                                                    for (i, param_name) in
                                                        method_params.iter().enumerate()
                                                    {
                                                        if let Some(arg_expr) = args_clone.get(i) {
                                                            s2 = Self::subst_param_in_stmt(
                                                                s2, param_name, arg_expr,
                                                            );
                                                        }
                                                    }
                                                    Self::subst_this_in_stmt(
                                                        s2,
                                                        &obj_name_clone,
                                                        &class_fields_clone,
                                                        &class_name,
                                                    )
                                                })
                                                .collect();

                                            let mut result = Vec::new();
                                            let mut ok = true;
                                            for s in &substituted {
                                                match self.convert_stmt(s) {
                                                    Ok(ir) => result.extend(ir),
                                                    Err(_) => {
                                                        ok = false;
                                                        break;
                                                    }
                                                }
                                            }
                                            if ok {
                                                Some(result)
                                            } else {
                                                None
                                            }
                                        } // close else (no return)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some(stmts) = inline_stmts {
                    return Ok(stmts);
                }

                // ── Normal statement handling ──────────────────────────────────
                // FASM-inspired: detect assignment expressions and emit proper Stmt::Assign
                // Without this, `a = a + 1;` becomes Expr(a+1) instead of Assign{a, a+1}
                match expr {
                    CppExpr::Assign { target, value } => {
                        if let CppExpr::Identifier(name) = target.as_ref() {
                            let v = self.convert_expr(value)?;
                            return Ok(vec![Stmt::Assign {
                                name: name.clone(),
                                value: v,
                            }]);
                        }
                        // Field assignment: obj.field = value
                        if let CppExpr::MemberAccess { object, member } = target.as_ref() {
                            let obj = self.convert_expr(object)?;
                            let v = self.convert_expr(value)?;
                            return Ok(vec![Stmt::FieldAssign {
                                object: obj,
                                field: member.clone(),
                                value: v,
                            }]);
                        }
                        // Array index assignment: arr[i] = value
                        if let CppExpr::Index { object, index } = target.as_ref() {
                            let obj = self.convert_expr(object)?;
                            let idx = self.convert_expr(index)?;
                            let v = self.convert_expr(value)?;
                            return Ok(vec![Stmt::IndexAssign {
                                object: obj,
                                index: idx,
                                value: v,
                            }]);
                        }
                        let v = self.convert_expr(value)?;
                        return Ok(vec![Stmt::Expr(v)]);
                    }
                    CppExpr::CompoundAssign { target, op, value } => {
                        if let CppExpr::Identifier(name) = target.as_ref() {
                            let v = self.convert_expr(value)?;
                            let comp_op = match op {
                                CppBinOp::Add => CompoundOp::AddAssign,
                                CppBinOp::Sub => CompoundOp::SubAssign,
                                CppBinOp::Mul => CompoundOp::MulAssign,
                                CppBinOp::Div => CompoundOp::DivAssign,
                                CppBinOp::Mod => CompoundOp::ModAssign,
                                CppBinOp::BitAnd => CompoundOp::AndAssign,
                                CppBinOp::BitOr => CompoundOp::OrAssign,
                                CppBinOp::BitXor => CompoundOp::XorAssign,
                                CppBinOp::Shl => CompoundOp::ShlAssign,
                                CppBinOp::Shr => CompoundOp::ShrAssign,
                                _ => CompoundOp::AddAssign,
                            };
                            return Ok(vec![Stmt::CompoundAssign {
                                name: name.clone(),
                                op: comp_op,
                                value: v,
                            }]);
                        }
                        let v = self.convert_expr(value)?;
                        return Ok(vec![Stmt::Expr(v)]);
                    }
                    CppExpr::UnaryOp {
                        op,
                        expr: inner,
                        is_prefix,
                    } => {
                        use super::cpp_ast::CppUnaryOp;
                        match op {
                            CppUnaryOp::PreInc | CppUnaryOp::PostInc => {
                                if let CppExpr::Identifier(name) = inner.as_ref() {
                                    return Ok(vec![Stmt::Increment {
                                        name: name.clone(),
                                        is_pre: *is_prefix,
                                        is_increment: true,
                                    }]);
                                }
                            }
                            CppUnaryOp::PreDec | CppUnaryOp::PostDec => {
                                if let CppExpr::Identifier(name) = inner.as_ref() {
                                    return Ok(vec![Stmt::Increment {
                                        name: name.clone(),
                                        is_pre: *is_prefix,
                                        is_increment: false,
                                    }]);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                let ir_expr = self.convert_expr(expr)?;
                // Handle cout << "text" as Println
                if let Expr::String(ref s) = ir_expr {
                    return Ok(vec![Stmt::Print(Expr::String(s.clone()))]);
                }
                Ok(vec![Stmt::Expr(ir_expr)])
            }
            CppStmt::VarDecl {
                type_spec,
                declarators,
            } => {
                let mut stmts = Vec::new();
                for d in declarators {
                    let mut var_type = self.convert_type(type_spec);
                    // Apply derived types from declarator (e.g., int nums[3] → Array(I64, Some(3)))
                    for dt in &d.derived_type {
                        match dt {
                            crate::frontend::cpp::cpp_ast::CppDerivedType::Array(size) => {
                                var_type = Type::Array(Box::new(var_type), *size);
                            }
                            crate::frontend::cpp::cpp_ast::CppDerivedType::Pointer => {
                                var_type = Type::Pointer(Box::new(var_type));
                            }
                            crate::frontend::cpp::cpp_ast::CppDerivedType::Reference => {
                                var_type = Type::Reference(Box::new(var_type));
                            }
                            crate::frontend::cpp::cpp_ast::CppDerivedType::RValueRef => {
                                var_type = Type::Reference(Box::new(var_type));
                            }
                        }
                    }
                    // For class types, emit a zero placeholder VarDecl so the ISA
                    // can allocate the stack slot. Flat sub-fields carry the real values.
                    let is_known_class = if let CppType::Named(cn)
                    | CppType::Class(cn)
                    | CppType::Struct(cn) = type_spec
                    {
                        self.class_fields.iter().any(|(n, _)| n == cn)
                    } else {
                        false
                    };
                    if is_known_class {
                        // Emit parent as zero (a placeholder); real data is in c.field vars
                        stmts.push(Stmt::VarDecl {
                            var_type,
                            name: d.name.clone(),
                            value: Some(Expr::Number(0)),
                        });
                    } else {
                        let init = if let Some(ref e) = d.initializer {
                            Some(self.convert_expr(e)?)
                        } else {
                            None
                        };
                        // For local reference vars (int &ref = x), wrap init in AddressOf
                        let init = if matches!(&var_type, Type::Reference(_)) {
                            init.map(|v| {
                                if matches!(&v, Expr::AddressOf(_)) {
                                    v
                                } else {
                                    Expr::AddressOf(Box::new(v))
                                }
                            })
                        } else {
                            init
                        };
                        stmts.push(Stmt::VarDecl {
                            var_type,
                            name: d.name.clone(),
                            value: init,
                        });
                    }

                    // If this is a class/struct type, track it and inline constructor
                    if let CppType::Named(class_name)
                    | CppType::Class(class_name)
                    | CppType::Struct(class_name) = type_spec
                    {
                        // Register variable â†’ class mapping for MethodCall resolution
                        self.variable_types
                            .push((d.name.clone(), class_name.clone()));

                        let ctor_args: Vec<CppExpr> = if let Some(ref e) = d.initializer {
                            match e {
                                CppExpr::Call { args, .. } => args.clone(),
                                CppExpr::InitList(items) => items.clone(),
                                _ => vec![e.clone()],
                            }
                        } else {
                            vec![]
                        };

                        // Find the best matching ctor (by arity)
                        let ctor_data = self
                            .class_ctor_inits
                            .iter()
                            .find(|(cn, params, _, _)| {
                                cn == class_name && params.len() == ctor_args.len()
                            })
                            .or_else(|| {
                                self.class_ctor_inits
                                    .iter()
                                    .find(|(cn, _, _, _)| cn == class_name)
                            })
                            .map(|(_, params, inits, body)| {
                                (params.clone(), inits.clone(), body.clone())
                            });

                        if let Some((ctor_params, ctor_field_inits, ctor_body_stmts)) = ctor_data {
                            // Inline ctor: emit flat field VarDecls, substituting params with args
                            // Counter c2(5) → c2.value=0, c2.max_value=5
                            let field_names: Vec<String> = self
                                .class_fields
                                .iter()
                                .find(|(cn, _)| cn == class_name)
                                .map(|(_, f)| f.clone())
                                .unwrap_or_default();

                            // Build a param → arg substitution table (as CppExpr)
                            let substitutions: Vec<(String, CppExpr)> = ctor_params
                                .iter()
                                .zip(ctor_args.iter())
                                .map(|(p, a)| (p.clone(), a.clone()))
                                .collect();

                            for fname in &field_names {
                                let flat_name = format!("{}.{}", d.name, fname);
                                // Determine the correct IR type for this field
                                let field_ir_type = self
                                    .class_field_type_map
                                    .iter()
                                    .find(|(cn, fn2, _)| cn == class_name && fn2 == fname)
                                    .map(|(_, _, cpp_ty)| self.convert_type(cpp_ty))
                                    .unwrap_or(Type::I64);
                                // Find the init expr for this field in the ctor init list
                                // First check direct field inits (this field's own init)
                                let raw_expr = ctor_field_inits
                                    .iter()
                                    .find(|(fn2, _)| fn2 == fname)
                                    .map(|(_, e)| e.clone());

                                // If not found directly, check if this field belongs to a base class
                                // and the base class was initialized via ctor init list
                                let init_val = if let Some(mut expr) = raw_expr {
                                    // Substitute param references with actual arg exprs
                                    for (param_name, arg_expr) in &substitutions {
                                        expr = Self::subst_param(expr, param_name, arg_expr);
                                    }
                                    self.convert_expr(&expr).unwrap_or(Expr::Number(0))
                                } else {
                                    // Check if field is from a base class initialized in init list
                                    // e.g., Circle() : Shape(1), radius(0) → Shape(1) initializes id=1
                                    let mut base_val = None;
                                    for (init_name, init_expr) in &ctor_field_inits {
                                        // If init_name is a class name (base init call), expand it
                                        let is_class_name =
                                            self.class_fields.iter().any(|(cn, _)| cn == init_name);
                                        if is_class_name {
                                            // Find the base class's ctor that matches the args
                                            let base_ctor_args = match init_expr {
                                                CppExpr::Call { args, .. } => args.clone(),
                                                CppExpr::InitList(items) => items.clone(),
                                                other => vec![other.clone()],
                                            };
                                            // Find base ctor with exact arity match first
                                            let base_ctor = self
                                                .class_ctor_inits
                                                .iter()
                                                .find(|(cn, params, _, _)| {
                                                    cn == init_name
                                                        && params.len() == base_ctor_args.len()
                                                })
                                                .or_else(|| {
                                                    if base_ctor_args.is_empty() {
                                                        // fallback to default ctor only when no args
                                                        self.class_ctor_inits.iter().find(
                                                            |(cn, params, _, _)| {
                                                                cn == init_name && params.is_empty()
                                                            },
                                                        )
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .map(|(_, params, inits, _)| {
                                                    (params.clone(), inits.clone())
                                                });
                                            if let Some((base_params, base_inits)) = base_ctor {
                                                // Build substitution of base ctor params with args
                                                let base_subs: Vec<(String, CppExpr)> = base_params
                                                    .iter()
                                                    .zip(base_ctor_args.iter())
                                                    .map(|(p, a)| {
                                                        // Also substitute our own params in args
                                                        let mut a2 = a.clone();
                                                        for (pn, pe) in &substitutions {
                                                            a2 = Self::subst_param(a2, pn, pe);
                                                        }
                                                        (p.clone(), a2)
                                                    })
                                                    .collect();
                                                // Check if 'fname' is in base inits
                                                if let Some((_, field_expr)) =
                                                    base_inits.iter().find(|(fn2, _)| fn2 == fname)
                                                {
                                                    let mut expr = field_expr.clone();
                                                    for (pn, pe) in &base_subs {
                                                        expr = Self::subst_param(expr, pn, pe);
                                                    }
                                                    base_val = Some(
                                                        self.convert_expr(&expr)
                                                            .unwrap_or(Expr::Number(0)),
                                                    );
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    base_val.unwrap_or(Expr::Number(0))
                                };

                                stmts.push(Stmt::VarDecl {
                                    var_type: field_ir_type,
                                    name: flat_name,
                                    value: Some(init_val),
                                });
                            }
                            // Inline constructor body statements (e.g., data[0]=0)
                            // by substituting this->field with obj.field
                            if !ctor_body_stmts.is_empty() {
                                let class_fields_clone = self.class_fields.clone();
                                let obj_name = d.name.clone();
                                let class_name_owned = class_name.to_string();
                                for body_stmt in &ctor_body_stmts {
                                    let mut s = body_stmt.clone();
                                    // Substitute ctor params with actual args
                                    for (i, param_name) in ctor_params.iter().enumerate() {
                                        if let Some(arg_expr) = ctor_args.get(i) {
                                            s = Self::subst_param_in_stmt(s, param_name, arg_expr);
                                        }
                                    }
                                    // Substitute this->field with obj.field
                                    s = Self::subst_this_in_stmt(
                                        s,
                                        &obj_name,
                                        &class_fields_clone,
                                        &class_name_owned,
                                    );
                                    match self.convert_stmt(&s) {
                                        Ok(ir_stmts) => stmts.extend(ir_stmts),
                                        Err(_) => {} // Skip statements that can't be converted
                                    }
                                }
                            }
                        } else if !ctor_args.is_empty() {
                            // No ctor info found → fallback to function call
                            let ctor_name = format!("{}::{}", class_name, class_name);
                            let ir_ctor_args: Vec<Expr> = ctor_args
                                .iter()
                                .map(|a| self.convert_expr(a))
                                .collect::<Result<Vec<_>, _>>()?;
                            let mut call_args =
                                vec![Expr::AddressOf(Box::new(Expr::Variable(d.name.clone())))];
                            call_args.extend(ir_ctor_args);
                            stmts.push(Stmt::Expr(Expr::Call {
                                name: ctor_name,
                                args: call_args,
                            }));
                        }
                    }
                }
                Ok(stmts)
            }

            CppStmt::Return(Some(expr)) => Ok(vec![Stmt::Return(Some(self.convert_expr(expr)?))]),
            CppStmt::Return(None) => Ok(vec![Stmt::Return(None)]),
            CppStmt::Block(stmts) => {
                let vars_before = self.variable_types.len();
                let mut ir_stmts = Vec::new();
                for s in stmts {
                    ir_stmts.extend(self.convert_stmt(s)?);
                }
                // RAII: emit destructor calls in LIFO order for class-typed
                // variables declared in this block scope
                let scope_vars: Vec<(String, String)> = self
                    .variable_types[vars_before..]
                    .iter()
                    .cloned()
                    .collect();
                for (var_name, class_name) in scope_vars.iter().rev() {
                    let dtor_name = format!("{}::~{}", class_name, class_name);
                    // Check if a destructor body was actually defined
                    let has_dtor = self
                        .class_method_bodies
                        .iter()
                        .any(|(cn, mn, _, _)| cn == class_name && mn == &format!("~{}", class_name));
                    if has_dtor {
                        // Inline the destructor body with this→field substitution
                        let class_fields_clone = self.class_fields.clone();
                        let method_bodies_clone = self.class_method_bodies.clone();
                        for (cn, mn, _params, body_stmts) in &method_bodies_clone {
                            if cn == class_name && mn == &format!("~{}", class_name) {
                                for body_stmt in body_stmts {
                                    let s = Self::subst_this_in_stmt(
                                        body_stmt.clone(),
                                        var_name,
                                        &class_fields_clone,
                                        class_name,
                                    );
                                    match self.convert_stmt(&s) {
                                        Ok(dtor_stmts) => ir_stmts.extend(dtor_stmts),
                                        Err(_) => {}
                                    }
                                }
                                break;
                            }
                        }
                    } else {
                        // No body — emit a call to the destructor function
                        ir_stmts.push(Stmt::Expr(Expr::Call {
                            name: dtor_name,
                            args: vec![Expr::AddressOf(Box::new(Expr::Variable(
                                var_name.clone(),
                            )))],
                        }));
                    }
                }
                // Remove scope-local variable types
                self.variable_types.truncate(vars_before);
                Ok(ir_stmts)
            }
            CppStmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                let cond = self.convert_expr(condition)?;
                let then_stmts = self.convert_stmt(then_body)?;
                let else_stmts = if let Some(eb) = else_body {
                    Some(self.convert_stmt(eb)?)
                } else {
                    None
                };
                Ok(vec![Stmt::If {
                    condition: cond,
                    then_body: then_stmts,
                    else_body: else_stmts,
                }])
            }
            CppStmt::While { condition, body } => {
                let cond = self.convert_expr(condition)?;
                let body_stmts = self.convert_stmt(body)?;
                Ok(vec![Stmt::While {
                    condition: cond,
                    body: body_stmts,
                }])
            }
            CppStmt::DoWhile { body, condition } => {
                let body_stmts = self.convert_stmt(body)?;
                let cond = self.convert_expr(condition)?;
                Ok(vec![Stmt::DoWhile {
                    body: body_stmts,
                    condition: cond,
                }])
            }
            CppStmt::For {
                init,
                condition,
                increment,
                body,
            } => {
                // Convert C++ for to while
                let mut stmts = Vec::new();
                if let Some(init_stmt) = init {
                    stmts.extend(self.convert_stmt(init_stmt)?);
                }
                let cond = condition
                    .as_ref()
                    .map(|c| self.convert_expr(c))
                    .transpose()?
                    .unwrap_or(Expr::Bool(true));
                let mut body_stmts = self.convert_stmt(body)?;
                if let Some(inc) = increment {
                    // Convert increment expression to proper statement
                    // (e.g., i++ â†’ Stmt::Increment, i += 1 â†’ Stmt::CompoundAssign)
                    let inc_ref: &CppExpr = &inc;
                    let inc_stmt = match inc_ref {
                        CppExpr::UnaryOp {
                            op,
                            expr: inner,
                            is_prefix,
                        } => {
                            use super::cpp_ast::CppUnaryOp;
                            let is_pre = *is_prefix;
                            match op {
                                CppUnaryOp::PreInc | CppUnaryOp::PostInc => {
                                    if let CppExpr::Identifier(name) = inner.as_ref() {
                                        Some(Stmt::Increment {
                                            name: name.clone(),
                                            is_pre,
                                            is_increment: true,
                                        })
                                    } else {
                                        None
                                    }
                                }
                                CppUnaryOp::PreDec | CppUnaryOp::PostDec => {
                                    if let CppExpr::Identifier(name) = inner.as_ref() {
                                        Some(Stmt::Increment {
                                            name: name.clone(),
                                            is_pre,
                                            is_increment: false,
                                        })
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        }
                        CppExpr::CompoundAssign { target, op, value } => {
                            if let CppExpr::Identifier(name) = target.as_ref() {
                                let v = self.convert_expr(value)?;
                                let comp_op = match op {
                                    CppBinOp::Add => CompoundOp::AddAssign,
                                    CppBinOp::Sub => CompoundOp::SubAssign,
                                    CppBinOp::Mul => CompoundOp::MulAssign,
                                    CppBinOp::Div => CompoundOp::DivAssign,
                                    CppBinOp::Mod => CompoundOp::ModAssign,
                                    _ => CompoundOp::AddAssign,
                                };
                                Some(Stmt::CompoundAssign {
                                    name: name.clone(),
                                    op: comp_op,
                                    value: v,
                                })
                            } else {
                                None
                            }
                        }
                        CppExpr::Assign { target, value } => {
                            if let CppExpr::Identifier(name) = target.as_ref() {
                                let v = self.convert_expr(value)?;
                                Some(Stmt::Assign {
                                    name: name.clone(),
                                    value: v,
                                })
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    body_stmts.push(inc_stmt.unwrap_or_else(|| {
                        Stmt::Expr(self.convert_expr(inc_ref).unwrap_or(Expr::Number(0)))
                    }));
                }
                stmts.push(Stmt::While {
                    condition: cond,
                    body: body_stmts,
                });
                Ok(stmts)
            }
            CppStmt::RangeFor {
                name,
                iterable,
                body,
                ..
            } => {
                let iter_expr = self.convert_expr(iterable)?;
                let body_stmts = self.convert_stmt(body)?;
                Ok(vec![Stmt::ForEach {
                    var: name.clone(),
                    iterable: iter_expr,
                    body: body_stmts,
                }])
            }
            CppStmt::Switch {
                expr,
                cases,
                default,
            } => {
                let switch_expr = self.convert_expr(expr)?;
                let ir_cases: Vec<SwitchCase> = cases
                    .iter()
                    .map(|c| {
                        let val = self.convert_expr(&c.value).unwrap_or(Expr::Number(0));
                        let body: Vec<Stmt> = c
                            .body
                            .iter()
                            .flat_map(|s| self.convert_stmt(s).unwrap_or_default())
                            .collect();
                        SwitchCase {
                            value: val,
                            body,
                            has_break: true,
                        }
                    })
                    .collect();
                let default_body = default.as_ref().map(|d| {
                    d.iter()
                        .flat_map(|s| self.convert_stmt(s).unwrap_or_default())
                        .collect()
                });
                Ok(vec![Stmt::Switch {
                    expr: switch_expr,
                    cases: ir_cases,
                    default: default_body,
                }])
            }
            CppStmt::Break => Ok(vec![Stmt::Break]),
            CppStmt::Continue => Ok(vec![Stmt::Continue]),
            CppStmt::Goto(_) => Ok(vec![]), // Simplified
            CppStmt::Label(_, inner) => self.convert_stmt(inner),
            CppStmt::Empty => Ok(vec![]),
            CppStmt::Try { body, catches } => {
                // Exception → error codes: run try body, then check __adb_has_error()
                let mut stmts = Vec::new();
                for s in body {
                    stmts.extend(self.convert_stmt(s)?);
                }
                // For each catch: if (__adb_has_error()) { handler; __adb_clear_error(); }
                for catch_block in catches {
                    let mut catch_stmts = Vec::new();
                    // If catch has a named param, declare it with __adb_get_error()
                    if let Some(ref pname) = catch_block.param_name {
                        catch_stmts.push(Stmt::VarDecl {
                            var_type: Type::Pointer(Box::new(Type::I8)),
                            name: pname.clone(),
                            value: Some(Expr::Call {
                                name: "__adb_get_error".to_string(),
                                args: vec![],
                            }),
                        });
                    }
                    for s in &catch_block.body {
                        match self.convert_stmt(s) {
                            Ok(ir) => catch_stmts.extend(ir),
                            Err(_) => {}
                        }
                    }
                    // Clear error after handling
                    catch_stmts.push(Stmt::Expr(Expr::Call {
                        name: "__adb_clear_error".to_string(),
                        args: vec![],
                    }));
                    stmts.push(Stmt::If {
                        condition: Expr::Call {
                            name: "__adb_has_error".to_string(),
                            args: vec![],
                        },
                        then_body: catch_stmts,
                        else_body: None,
                    });
                }
                Ok(stmts)
            }
            CppStmt::Throw(expr) => {
                // throw expr → __adb_set_error(msg); return default;
                let mut stmts = Vec::new();
                if let Some(e) = expr {
                    let ir_expr = self.convert_expr(e)?;
                    stmts.push(Stmt::Expr(Expr::Call {
                        name: "__adb_set_error".to_string(),
                        args: vec![ir_expr],
                    }));
                } else {
                    stmts.push(Stmt::Expr(Expr::Call {
                        name: "__adb_set_error".to_string(),
                        args: vec![Expr::String("exception".to_string())],
                    }));
                }
                Ok(stmts)
            }
            CppStmt::CoReturn(expr) => Ok(vec![Stmt::Return(
                expr.as_ref().map(|e| self.convert_expr(e)).transpose()?,
            )]),
        }
    }

    // ========== Expression conversion ==========

    fn convert_expr(&mut self, expr: &CppExpr) -> Result<Expr, String> {
        match expr {
            CppExpr::IntLiteral(n) => Ok(Expr::Number(*n)),
            CppExpr::UIntLiteral(n) => Ok(Expr::Number(*n as i64)),
            CppExpr::FloatLiteral(f) => Ok(Expr::Float(*f)),
            CppExpr::StringLiteral(s) => Ok(Expr::String(s.clone())),
            CppExpr::CharLiteral(c) => Ok(Expr::Number(*c as i64)),
            CppExpr::BoolLiteral(b) => Ok(Expr::Bool(*b)),
            CppExpr::NullptrLiteral => Ok(Expr::Nullptr),
            CppExpr::Identifier(name) => {
                // If inside a class method and this is a class field, convert to this->field
                if self.is_class_field(name) {
                    Ok(Expr::ArrowAccess {
                        pointer: Box::new(Expr::Variable("this".to_string())),
                        field: name.clone(),
                    })
                } else {
                    Ok(Expr::Variable(name.clone()))
                }
            }
            CppExpr::ScopedIdentifier { scope, name } => {
                let full = format!("{}::{}", scope.join("::"), name);
                // Handle std::cout, std::endl, etc
                match full.as_str() {
                    "std::cout" => Ok(Expr::Variable("stdout".to_string())),
                    "std::cerr" => Ok(Expr::Variable("stderr".to_string())),
                    "std::endl" => Ok(Expr::String("\n".to_string())),
                    _ => Ok(Expr::Variable(full)),
                }
            }
            CppExpr::This => Ok(Expr::Variable("this".to_string())),

            CppExpr::BinaryOp { op, left, right } => {
                let l = self.convert_expr(left)?;
                let r = self.convert_expr(right)?;

                // Handle cout << x as Print
                if *op == CppBinOp::Shl {
                    if let Expr::Variable(ref name) = l {
                        if name == "stdout" || name == "cout" {
                            return Ok(r); // Will be wrapped in Print by stmt handler
                        }
                    }
                    // Chained: (cout << "hello") << endl
                    if let Expr::String(_) = l {
                        return Ok(r);
                    }
                }

                match op {
                    CppBinOp::Add => Ok(Expr::BinaryOp {
                        op: BinOp::Add,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Sub => Ok(Expr::BinaryOp {
                        op: BinOp::Sub,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Mul => Ok(Expr::BinaryOp {
                        op: BinOp::Mul,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Div => Ok(Expr::BinaryOp {
                        op: BinOp::Div,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Mod => Ok(Expr::BinaryOp {
                        op: BinOp::Mod,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Eq => Ok(Expr::Comparison {
                        op: CmpOp::Eq,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Ne => Ok(Expr::Comparison {
                        op: CmpOp::Ne,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Lt => Ok(Expr::Comparison {
                        op: CmpOp::Lt,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Le => Ok(Expr::Comparison {
                        op: CmpOp::Le,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Gt => Ok(Expr::Comparison {
                        op: CmpOp::Gt,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Ge => Ok(Expr::Comparison {
                        op: CmpOp::Ge,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::And => Ok(Expr::BinaryOp {
                        op: BinOp::And,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Or => Ok(Expr::BinaryOp {
                        op: BinOp::Or,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::BitAnd => Ok(Expr::BitwiseOp {
                        op: IrBitwiseOp::And,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::BitOr => Ok(Expr::BitwiseOp {
                        op: IrBitwiseOp::Or,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::BitXor => Ok(Expr::BitwiseOp {
                        op: IrBitwiseOp::Xor,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Shl => Ok(Expr::BitwiseOp {
                        op: IrBitwiseOp::LeftShift,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Shr => Ok(Expr::BitwiseOp {
                        op: IrBitwiseOp::RightShift,
                        left: Box::new(l),
                        right: Box::new(r),
                    }),
                    CppBinOp::Spaceship => {
                        // <=> returns -1, 0, 1 â€” approximate with subtraction
                        Ok(Expr::BinaryOp {
                            op: BinOp::Sub,
                            left: Box::new(l),
                            right: Box::new(r),
                        })
                    }
                }
            }

            CppExpr::UnaryOp { op, expr, .. } => {
                let e = self.convert_expr(expr)?;
                match op {
                    CppUnaryOp::Neg => Ok(Expr::UnaryOp {
                        op: IrUnaryOp::Neg,
                        expr: Box::new(e),
                    }),
                    CppUnaryOp::Not => Ok(Expr::UnaryOp {
                        op: IrUnaryOp::Not,
                        expr: Box::new(e),
                    }),
                    CppUnaryOp::BitNot => Ok(Expr::BitwiseNot(Box::new(e))),
                    CppUnaryOp::PreInc => Ok(Expr::PreIncrement(Box::new(e))),
                    CppUnaryOp::PreDec => Ok(Expr::PreDecrement(Box::new(e))),
                    CppUnaryOp::PostInc => Ok(Expr::PostIncrement(Box::new(e))),
                    CppUnaryOp::PostDec => Ok(Expr::PostDecrement(Box::new(e))),
                }
            }

            CppExpr::Assign { target, value } => {
                let _t = self.convert_expr(target)?;
                let v = self.convert_expr(value)?;
                // Return the value (C++ assignment is an expression)
                Ok(v)
            }

            CppExpr::CompoundAssign { value, .. } => {
                let v = self.convert_expr(value)?;
                Ok(v)
            }

            CppExpr::Call { callee, args } => {
                let ir_args: Vec<Expr> = args
                    .iter()
                    .map(|a| self.convert_expr(a))
                    .collect::<Result<Vec<_>, _>>()?;

                // Check if this is a method call: obj.method() or obj->method()
                match callee.as_ref() {
                    CppExpr::MemberAccess { object, member } => {
                        // Try expression-level inline expansion first:
                        // If method body is `return expr;`, inline as substituted expr
                        if let CppExpr::Identifier(obj_name) = object.as_ref() {
                            if let Some(class_name) = self.class_for_var(obj_name) {
                                let method_data = self
                                    .class_method_bodies
                                    .iter()
                                    .find(|(cn, mn, _, _)| {
                                        cn == &class_name && mn == member.as_str()
                                    })
                                    .map(|(_, _, params, body)| (params.clone(), body.clone()));

                                if let Some((method_params, method_body)) = method_data {
                                    // Check if body is a single return statement
                                    let return_expr = if method_body.len() == 1 {
                                        match &method_body[0] {
                                            CppStmt::Return(Some(e)) => Some(e.clone()),
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    };

                                    if let Some(mut ret_expr) = return_expr {
                                        // Substitute method params with args
                                        for (i, param_name) in method_params.iter().enumerate() {
                                            if let Some(arg_expr) = args.get(i) {
                                                ret_expr = Self::subst_param(
                                                    ret_expr, param_name, arg_expr,
                                                );
                                            }
                                        }
                                        // Substitute this->field with obj.field
                                        let class_fields_clone = self.class_fields.clone();
                                        ret_expr = Self::subst_this_in_expr(
                                            ret_expr,
                                            obj_name,
                                            &class_fields_clone,
                                            &class_name,
                                        );
                                        // Convert the substituted expression
                                        return self.convert_expr(&ret_expr);
                                    }
                                }
                            }
                        }

                        // obj.method(args) → ClassName::method(&obj, args...)
                        let class_opt = if let CppExpr::Identifier(obj_name) = object.as_ref() {
                            self.class_for_var(obj_name)
                        } else {
                            None
                        };

                        if let Some(class_name) = class_opt {
                            let obj_ir = self.convert_expr(object)?;
                            let mut call_args = vec![Expr::AddressOf(Box::new(obj_ir))];
                            call_args.extend(ir_args);
                            let func_name = format!("{}::{}", class_name, member);
                            return Ok(Expr::Call {
                                name: func_name,
                                args: call_args,
                            });
                        }

                        // Fallback: generate MethodCall (ISA will try to handle it)
                        let obj = self.convert_expr(object)?;
                        return Ok(Expr::MethodCall {
                            object: Box::new(obj),
                            method: member.clone(),
                            args: ir_args,
                        });
                    }
                    CppExpr::ArrowAccess { pointer, member } => {
                        // ptr->method(args) â€” resolve through pointer
                        let class_opt = if let CppExpr::Identifier(obj_name) = pointer.as_ref() {
                            self.class_for_var(obj_name)
                        } else {
                            None
                        };

                        if let Some(class_name) = class_opt {
                            let obj_ir = self.convert_expr(pointer)?;
                            let mut call_args = vec![Expr::AddressOf(Box::new(obj_ir))];
                            call_args.extend(ir_args);
                            let func_name = format!("{}::{}", class_name, member);
                            return Ok(Expr::Call {
                                name: func_name,
                                args: call_args,
                            });
                        }

                        let ptr = self.convert_expr(pointer)?;
                        return Ok(Expr::MethodCall {
                            object: Box::new(Expr::Deref(Box::new(ptr))),
                            method: member.clone(),
                            args: ir_args,
                        });
                    }
                    _ => {}
                }

                let name = match callee.as_ref() {
                    CppExpr::Identifier(n) => {
                        // If inside a namespace and this is an unqualified call to a sibling
                        // function, qualify it with the namespace prefix (FASM-style label resolution)
                        if let Some(ref ns) = self.current_namespace {
                            if self.namespace_functions.contains(n) {
                                format!("{}::{}", ns, n)
                            } else {
                                n.clone()
                            }
                        } else {
                            n.clone()
                        }
                    }
                    CppExpr::ScopedIdentifier { scope, name } => {
                        format!("{}::{}", scope.join("::"), name)
                    }
                    _ => "unknown".to_string(),
                };

                // Handle special functions
                match name.as_str() {
                    "printf" | "std::printf" => {
                        if let Some(Expr::String(ref _s)) = ir_args.first() {
                            return Ok(Expr::Call {
                                name: "printf".to_string(),
                                args: ir_args,
                            });
                        }
                        Ok(Expr::Call {
                            name,
                            args: ir_args,
                        })
                    }
                    "std::cout" => Ok(Expr::Call {
                        name: "print".to_string(),
                        args: ir_args,
                    }),
                    "malloc" | "std::malloc" => {
                        if let Some(size) = ir_args.first() {
                            Ok(Expr::Malloc(Box::new(size.clone())))
                        } else {
                            Ok(Expr::Malloc(Box::new(Expr::Number(0))))
                        }
                    }
                    _ => {
                        // Wrap arguments in AddressOf for reference parameters
                        let mut final_args = ir_args;
                        if let Some((_, ref_flags)) =
                            self.func_ref_params.iter().find(|(n, _)| n == &name)
                        {
                            for (i, is_ref) in ref_flags.iter().enumerate() {
                                if *is_ref && i < final_args.len() {
                                    // Don't double-wrap if already AddressOf
                                    if !matches!(&final_args[i], Expr::AddressOf(_)) {
                                        let arg = final_args[i].clone();
                                        final_args[i] = Expr::AddressOf(Box::new(arg));
                                    }
                                }
                            }
                        }
                        Ok(Expr::Call {
                            name,
                            args: final_args,
                        })
                    }
                }
            }

            CppExpr::MemberAccess { object, member } => {
                let obj = self.convert_expr(object)?;
                Ok(Expr::FieldAccess {
                    object: Box::new(obj),
                    field: member.clone(),
                })
            }

            CppExpr::ArrowAccess { pointer, member } => {
                let ptr = self.convert_expr(pointer)?;
                Ok(Expr::ArrowAccess {
                    pointer: Box::new(ptr),
                    field: member.clone(),
                })
            }

            CppExpr::Index { object, index } => {
                let obj = self.convert_expr(object)?;
                let idx = self.convert_expr(index)?;
                Ok(Expr::Index {
                    object: Box::new(obj),
                    index: Box::new(idx),
                })
            }

            CppExpr::Deref(inner) => Ok(Expr::Deref(Box::new(self.convert_expr(inner)?))),

            CppExpr::AddressOf(inner) => Ok(Expr::AddressOf(Box::new(self.convert_expr(inner)?))),

            CppExpr::Cast {
                target_type, expr, ..
            } => {
                let t = self.convert_type(target_type);
                let e = self.convert_expr(expr)?;
                Ok(Expr::Cast {
                    target_type: t,
                    expr: Box::new(e),
                })
            }

            CppExpr::SizeOf(arg) => match arg {
                CppSizeOfArg::Type(t) => {
                    let ir_type = self.convert_type(t);
                    Ok(Expr::SizeOf(Box::new(SizeOfArg::Type(ir_type))))
                }
                CppSizeOfArg::Expr(e) => {
                    let ir_expr = self.convert_expr(e)?;
                    Ok(Expr::SizeOf(Box::new(SizeOfArg::Expr(ir_expr))))
                }
            },

            CppExpr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => Ok(Expr::Ternary {
                condition: Box::new(self.convert_expr(condition)?),
                then_expr: Box::new(self.convert_expr(then_expr)?),
                else_expr: Box::new(self.convert_expr(else_expr)?),
            }),

            CppExpr::New {
                type_name,
                args,
                is_array,
                array_size,
            } => {
                let t = self.convert_type(type_name);
                let type_size = Expr::SizeOf(Box::new(SizeOfArg::Type(t.clone())));

                if *is_array {
                    // new T[n] → malloc(sizeof(T) * n)
                    let n = if let Some(sz) = array_size {
                        self.convert_expr(sz)?
                    } else {
                        Expr::Number(1)
                    };
                    let total = Expr::BinaryOp {
                        op: BinOp::Mul,
                        left: Box::new(type_size),
                        right: Box::new(n),
                    };
                    Ok(Expr::Malloc(Box::new(total)))
                } else {
                    // new T(args) → malloc(sizeof(T)) then call ctor
                    // Check if T is a class type with a constructor
                    let class_name = match type_name {
                        CppType::Named(n) => Some(n.clone()),
                        _ => None,
                    };

                    if let Some(ref cn) = class_name {
                        if !args.is_empty() || self.class_ctor_inits.iter().any(|(c, _, _, _)| c == cn) {
                            // Class with constructor: __new_ptr = malloc(sizeof(T)); T::T(__new_ptr, args); result = __new_ptr
                            // Since we can only return one Expr, we emit the malloc and ctor as a sequence
                            // For now: emit as Expr::New which the ISA handles
                            let ir_args: Vec<Expr> = args
                                .iter()
                                .map(|a| self.convert_expr(a))
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(Expr::New {
                                class_name: cn.clone(),
                                args: ir_args,
                            })
                        } else {
                            // Simple class, no ctor args → just malloc
                            Ok(Expr::Malloc(Box::new(type_size)))
                        }
                    } else if !args.is_empty() {
                        // new int(42) → malloc + deref assign
                        // Simplified: malloc returns ptr, value init handled by caller
                        Ok(Expr::Malloc(Box::new(type_size)))
                    } else {
                        Ok(Expr::Malloc(Box::new(type_size)))
                    }
                }
            }

            CppExpr::Delete { expr, is_array } => {
                let e = self.convert_expr(expr)?;
                if *is_array {
                    // delete[] arr → free(arr)
                    // (element destructors not called for primitive arrays)
                    Ok(Expr::Call {
                        name: "free".to_string(),
                        args: vec![e],
                    })
                } else {
                    // delete ptr → destructor + free
                    // Try to detect class type for destructor call
                    // For now: emit free (destructor inlined by RAII at scope exit)
                    Ok(Expr::Call {
                        name: "free".to_string(),
                        args: vec![e],
                    })
                }
            }

            CppExpr::Lambda { .. } => {
                // Simplified: convert lambda body to a call expression
                Ok(Expr::Number(0)) // Placeholder
            }

            CppExpr::InitList(items) => {
                let ir_items: Vec<Expr> = items
                    .iter()
                    .map(|e| self.convert_expr(e))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expr::Array(ir_items))
            }

            CppExpr::Throw(_) => Ok(Expr::Number(-1)), // Exception â†’ error code
            CppExpr::CoAwait(inner) => self.convert_expr(inner),
            CppExpr::CoYield(inner) => self.convert_expr(inner),

            _ => Ok(Expr::Number(0)),
        }
    }
}

// ========== Public API ==========

/// Convenience: parse C++ source â†’ ADead-BIB Program in one call
/// Full pipeline: Preprocessor â†’ Lexer â†’ Parser â†’ IR
pub fn compile_cpp_to_program(source: &str) -> Result<Program, String> {
    use super::cpp_lexer::CppLexer;
    use super::cpp_parser::CppParser;
    use super::cpp_preprocessor::CppPreprocessor;

    // Phase 0: Preprocess â€” resolve #include, strip #define/#ifdef/etc.
    let mut preprocessor = CppPreprocessor::new();
    let preprocessed = preprocessor.process(source);

    // Phase 1: Lex â€” tokenize preprocessed source
    let (tokens, lines) = CppLexer::new(&preprocessed).tokenize();

    // Phase 2: Parse â€” tokens â†’ C++ AST
    let unit = CppParser::new(tokens, lines).parse_translation_unit()?;

    // Phase 3: Lower â€” C++ AST â†’ ADead-BIB IR
    let mut converter = CppToIR::new();
    converter.convert(&unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world_cpp() {
        let program = compile_cpp_to_program(
            r#"
            int main() {
                printf("Hello from C++!\n");
                return 0;
            }
        "#,
        )
        .unwrap();
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn test_class_compilation() {
        let program = compile_cpp_to_program(
            r#"
            class Point {
            public:
                int x;
                int y;
                int getX() { return x; }
            };

            int main() {
                return 0;
            }
        "#,
        )
        .unwrap();
        assert!(program.structs.len() >= 1);
        assert_eq!(program.structs[0].name, "Point");
        assert!(program.functions.len() >= 1); // main + getX
    }

    #[test]
    fn test_template_function() {
        let program = compile_cpp_to_program(
            r#"
            template<typename T>
            T add(T a, T b) {
                return a + b;
            }

            int main() {
                int result = add(3, 4);
                return 0;
            }
        "#,
        )
        .unwrap();
        assert!(program.functions.len() >= 2);
    }

    #[test]
    fn test_namespace() {
        let program = compile_cpp_to_program(
            r#"
            namespace math {
                int square(int x) {
                    return x * x;
                }
            }

            int main() {
                return 0;
            }
        "#,
        )
        .unwrap();
        assert!(program.functions.len() >= 2);
    }

    #[test]
    fn test_enum_class() {
        let program = compile_cpp_to_program(
            r#"
            enum class Color : int {
                Red = 0,
                Green = 1,
                Blue = 2
            };

            int main() {
                return 0;
            }
        "#,
        )
        .unwrap();
        assert_eq!(program.statements.len(), 3); // 3 enum constants
    }

    #[test]
    fn test_modern_cpp() {
        let program = compile_cpp_to_program(
            r#"
            int main() {
                auto x = 42;
                const int y = 100;
                int arr[] = {1, 2, 3};
                return x + y;
            }
        "#,
        )
        .unwrap();
        assert_eq!(program.functions.len(), 1);
    }

    // ========== Example file tests ==========

    #[test]
    fn test_example_hello_cpp() {
        let source = std::fs::read_to_string("examples/cpp/hello.cpp").unwrap();
        let result = compile_cpp_to_program(&source);
        assert!(result.is_ok(), "hello.cpp failed: {}", result.unwrap_err());
    }

    #[test]
    fn test_example_cpp_oop() {
        let source = std::fs::read_to_string("examples/cpp/cpp_oop.cpp").unwrap();
        let result = compile_cpp_to_program(&source);
        assert!(
            result.is_ok(),
            "cpp_oop.cpp failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_cpp_templates() {
        let source = std::fs::read_to_string("examples/cpp/cpp_templates.cpp").unwrap();
        let result = compile_cpp_to_program(&source);
        assert!(
            result.is_ok(),
            "cpp_templates.cpp failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_cpp_modern() {
        let source = std::fs::read_to_string("examples/cpp/cpp_modern.cpp").unwrap();
        let result = compile_cpp_to_program(&source);
        assert!(
            result.is_ok(),
            "cpp_modern.cpp failed: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn test_example_cpp_stdlib_long() {
        let source = std::fs::read_to_string("examples/cpp/cpp_stdlib_long.cpp").unwrap();
        let result = compile_cpp_to_program(&source);
        assert!(
            result.is_ok(),
            "cpp_stdlib_long.cpp failed: {}",
            result.unwrap_err()
        );
        let prog = result.unwrap();
        assert!(
            prog.functions.len() > 10,
            "cpp_stdlib_long.cpp should have many functions, got {}",
            prog.functions.len()
        );
    }

    // ================================================================
    // GCC/LLVM/MSVC-INSPIRED C++ COMPREHENSIVE TESTS
    // ================================================================
    // Covers: classes, inheritance, templates, namespaces, enum class,
    //         constexpr, auto, nullptr, type aliases, control flow
    // Each test verifies parsing + IR generation for C++ features.
    // ================================================================

    // --- Example file tests (new test files) ---

    #[test]
    fn test_example_class_basic() {
        let source = std::fs::read_to_string("examples/cpp/test_class_basic.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_class_basic.cpp failed");
        assert!(prog.structs.len() >= 1, "should have Counter struct");
        assert!(prog.functions.len() >= 1, "should have main + methods");
    }

    #[test]
    fn test_example_inheritance() {
        let source = std::fs::read_to_string("examples/cpp/test_inheritance.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_inheritance.cpp failed");
        assert!(
            prog.structs.len() >= 3,
            "should have Shape + Circle + Rectangle, got {}",
            prog.structs.len()
        );
    }

    #[test]
    fn test_example_template_basic() {
        let source = std::fs::read_to_string("examples/cpp/test_template_basic.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_template_basic.cpp failed");
        assert!(
            prog.functions.len() >= 4,
            "should have max + min + abs + main"
        );
    }

    #[test]
    fn test_example_enum_class() {
        let source = std::fs::read_to_string("examples/cpp/test_enum_class.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_enum_class.cpp failed");
        assert_eq!(prog.functions.len(), 1, "should have main");
    }

    #[test]
    fn test_example_constexpr() {
        let source = std::fs::read_to_string("examples/cpp/test_constexpr.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_constexpr.cpp failed");
        assert!(
            prog.functions.len() >= 4,
            "should have factorial + fib + square + cube + main"
        );
    }

    #[test]
    fn test_example_auto_nullptr() {
        let source = std::fs::read_to_string("examples/cpp/test_auto_nullptr.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_auto_nullptr.cpp failed");
        assert!(
            prog.functions.len() >= 3,
            "should have add + multiply + main"
        );
    }

    #[test]
    fn test_example_nested_namespace() {
        let source = std::fs::read_to_string("examples/cpp/test_nested_namespace.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_nested_namespace.cpp failed");
        assert!(
            prog.functions.len() >= 4,
            "should have multiple namespace functions + main"
        );
    }

    #[test]
    fn test_example_using_alias() {
        let source = std::fs::read_to_string("examples/cpp/test_using_alias.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_using_alias.cpp failed");
        assert!(
            prog.functions.len() >= 3,
            "should have double_val + triple_val + main"
        );
    }

    #[test]
    fn test_example_class_methods() {
        let source = std::fs::read_to_string("examples/cpp/test_class_methods.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_class_methods.cpp failed");
        assert!(prog.structs.len() >= 1, "should have Calculator struct");
    }

    #[test]
    fn test_example_cpp_control_flow() {
        let source = std::fs::read_to_string("examples/cpp/test_cpp_control_flow.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_cpp_control_flow.cpp failed");
        assert!(
            prog.functions.len() >= 3,
            "should have fibonacci + is_prime + main"
        );
    }

    // --- Existing .cpp test file validations ---

    #[test]
    fn test_example_test_minimal() {
        let source = std::fs::read_to_string("examples/cpp/test_minimal.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_minimal.cpp failed");
        assert!(prog.functions.len() >= 2);
    }

    #[test]
    fn test_example_test_5func() {
        let source = std::fs::read_to_string("examples/cpp/test_5func.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_5func.cpp failed");
        assert!(prog.functions.len() >= 5);
    }

    #[test]
    fn test_example_test_namespace() {
        let source = std::fs::read_to_string("examples/cpp/test_namespace.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_namespace.cpp failed");
        assert!(prog.functions.len() >= 4);
    }

    #[test]
    fn test_example_test_recursion_cpp() {
        let source = std::fs::read_to_string("examples/cpp/test_recursion.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_recursion.cpp failed");
        assert!(prog.functions.len() >= 2);
    }

    #[test]
    fn test_example_test_forloop_cpp() {
        let source = std::fs::read_to_string("examples/cpp/test_forloop.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_forloop.cpp failed");
        assert!(prog.functions.len() >= 1);
    }

    #[test]
    fn test_example_test_gcd() {
        let source = std::fs::read_to_string("examples/cpp/test_gcd.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_gcd.cpp failed");
        assert!(prog.functions.len() >= 2);
    }

    #[test]
    fn test_example_test_prime() {
        let source = std::fs::read_to_string("examples/cpp/test_prime.cpp").unwrap();
        let prog = compile_cpp_to_program(&source).expect("test_prime.cpp failed");
        assert!(prog.functions.len() >= 2);
    }

    // --- Inline C++ feature tests (no external files) ---

    #[test]
    fn test_cpp_class_with_constructor() {
        let prog = compile_cpp_to_program(
            r#"
            class Vector2D {
            public:
                int x;
                int y;
                Vector2D(int px, int py) : x(px), y(py) {}
                int dot(int ox, int oy) { return x * ox + y * oy; }
                int magnitude_sq() { return x * x + y * y; }
            };
            int main() { return 0; }
        "#,
        )
        .unwrap();
        assert!(prog.structs.len() >= 1);
    }

    #[test]
    fn test_cpp_multiple_classes() {
        let prog = compile_cpp_to_program(
            r#"
            class A {
            public:
                int val;
                A(int v) : val(v) {}
                int get() { return val; }
            };
            class B {
            public:
                int data;
                B(int d) : data(d) {}
                int get() { return data; }
            };
            int main() { return 0; }
        "#,
        )
        .unwrap();
        assert!(prog.structs.len() >= 2);
    }

    #[test]
    fn test_cpp_namespace_with_classes() {
        let prog = compile_cpp_to_program(
            r#"
            namespace game {
                class Entity {
                public:
                    int id;
                    Entity(int i) : id(i) {}
                };
                int next_id() { return 42; }
            }
            int main() { return 0; }
        "#,
        )
        .unwrap();
        assert!(prog.functions.len() >= 1);
    }

    #[test]
    fn test_cpp_virtual_method() {
        let prog = compile_cpp_to_program(
            r#"
            class Base {
            public:
                virtual int value() { return 0; }
            };
            class Derived : public Base {
            public:
                int value() override { return 42; }
            };
            int main() { return 0; }
        "#,
        )
        .unwrap();
        assert!(prog.structs.len() >= 2);
    }

    #[test]
    fn test_cpp_explicit_constructor() {
        let prog = compile_cpp_to_program(
            r#"
            class Wrapper {
            public:
                int val;
                explicit Wrapper(int v) : val(v) {}
                int get() { return val; }
            };
            int main() {
                Wrapper w(42);
                return 0;
            }
        "#,
        )
        .unwrap();
        assert!(prog.structs.len() >= 1);
    }

    #[test]
    fn test_cpp_const_method() {
        let prog = compile_cpp_to_program(
            r#"
            class Buffer {
            public:
                int size;
                Buffer(int s) : size(s) {}
                int get_size() const { return size; }
                bool empty() const { return size <= 0; }
            };
            int main() { return 0; }
        "#,
        )
        .unwrap();
        assert!(prog.structs.len() >= 1);
    }

    #[test]
    fn test_cpp_noexcept() {
        let prog = compile_cpp_to_program(
            r#"
            class Safe {
            public:
                int val;
                Safe() : val(0) {}
                void reset() noexcept { val = 0; }
                int get() noexcept { return val; }
            };
            int main() { return 0; }
        "#,
        )
        .unwrap();
        assert!(prog.structs.len() >= 1);
    }

    #[test]
    fn test_cpp_for_loop_with_namespace() {
        let prog = compile_cpp_to_program(
            r#"
            namespace util {
                int sum(int n) {
                    int total = 0;
                    for (int i = 1; i <= n; i++) {
                        total = total + i;
                    }
                    return total;
                }
            }
            int main() {
                int s = util::sum(100);
                return s;
            }
        "#,
        )
        .unwrap();
        assert!(prog.functions.len() >= 2);
    }

    #[test]
    fn test_cpp_while_loop() {
        let prog = compile_cpp_to_program(
            r#"
            int power(int base, int exp) {
                int result = 1;
                while (exp > 0) {
                    result = result * base;
                    exp = exp - 1;
                }
                return result;
            }
            int main() {
                int r = power(2, 10);
                return r;
            }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 2);
    }

    #[test]
    fn test_cpp_ternary_operator() {
        let prog = compile_cpp_to_program(
            r#"
            int abs_val(int x) {
                return (x < 0) ? (0 - x) : x;
            }
            int max2(int a, int b) {
                return (a > b) ? a : b;
            }
            int main() { return abs_val(-5) + max2(3, 7); }
        "#,
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 3);
    }

    // --- End-to-end C++ â†’ machine code tests ---

    #[test]
    fn test_cpp_e2e_hello() {
        let prog = compile_cpp_to_program(
            r#"
            int main() {
                printf("Hello C++!\n");
                return 0;
            }
        "#,
        )
        .unwrap();
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, data, _, _) = compiler.compile(&prog);
        assert!(!code.is_empty(), "should generate code");
        assert!(!data.is_empty(), "should have string data");
    }

    #[test]
    fn test_cpp_e2e_namespace() {
        let prog = compile_cpp_to_program(
            r#"
            namespace math {
                int add(int a, int b) { return a + b; }
                int mul(int a, int b) { return a * b; }
            }
            int main() {
                int r = math::add(3, 4);
                printf("result=%d\n", r);
                return 0;
            }
        "#,
        )
        .unwrap();
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, _, _, _) = compiler.compile(&prog);
        assert!(!code.is_empty());
    }

    #[test]
    fn test_cpp_e2e_class() {
        let prog = compile_cpp_to_program(
            r#"
            class Point {
            public:
                int x;
                int y;
                Point(int px, int py) : x(px), y(py) {}
                int sum() { return x + y; }
            };
            int main() {
                printf("done\n");
                return 0;
            }
        "#,
        )
        .unwrap();
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, _, _, _) = compiler.compile(&prog);
        assert!(!code.is_empty());
    }

    #[test]
    fn test_cpp_e2e_template() {
        let prog = compile_cpp_to_program(
            r#"
            template<typename T>
            T add(T a, T b) { return a + b; }
            int main() {
                int r = add(10, 20);
                printf("add=%d\n", r);
                return 0;
            }
        "#,
        )
        .unwrap();
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, _, _, _) = compiler.compile(&prog);
        assert!(!code.is_empty());
    }

    #[test]
    fn test_cpp_e2e_full_oop_example() {
        let source =
            std::fs::read_to_string("examples/cpp/cpp_oop.cpp").expect("cpp_oop.cpp should exist");
        let prog = compile_cpp_to_program(&source).expect("cpp_oop.cpp should parse");
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, data, _, _) = compiler.compile(&prog);
        assert!(!code.is_empty(), "cpp_oop.cpp should generate code");
        assert!(!data.is_empty(), "cpp_oop.cpp should have string data");
    }

    #[test]
    fn test_cpp_e2e_full_templates_example() {
        let source = std::fs::read_to_string("examples/cpp/cpp_templates.cpp")
            .expect("cpp_templates.cpp should exist");
        let prog = compile_cpp_to_program(&source).expect("cpp_templates.cpp should parse");
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, _, _, _) = compiler.compile(&prog);
        assert!(!code.is_empty(), "cpp_templates.cpp should generate code");
    }

    #[test]
    fn test_cpp_e2e_full_modern_example() {
        let source = std::fs::read_to_string("examples/cpp/cpp_modern.cpp")
            .expect("cpp_modern.cpp should exist");
        let prog = compile_cpp_to_program(&source).expect("cpp_modern.cpp should parse");
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, _, _, _) = compiler.compile(&prog);
        assert!(!code.is_empty(), "cpp_modern.cpp should generate code");
    }
}
