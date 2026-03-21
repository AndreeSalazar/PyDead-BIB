use crate::frontend::ast::*;
use crate::frontend::types::Type;
use std::collections::HashMap;

/// Registro de una función: (tipos de parámetros, tipo de retorno)
type FuncSig = (Vec<Type>, Type);

/// Registro de un struct: nombre → lista de (campo, tipo)
type StructFields = Vec<(String, Type)>;

pub struct TypeChecker {
    /// Variables locales en scope actual: nombre → tipo
    symbol_table: HashMap<String, Type>,
    /// Funciones registradas: nombre → (params, retorno)
    function_registry: HashMap<String, FuncSig>,
    /// Structs registrados: nombre → campos
    struct_registry: HashMap<String, StructFields>,
    /// Tipo de retorno de la función actual
    current_return_type: Type,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            symbol_table: HashMap::new(),
            function_registry: HashMap::new(),
            struct_registry: HashMap::new(),
            current_return_type: Type::Void,
        }
    }

    // =========================================================
    // Entry point
    // =========================================================

    pub fn check_program(&mut self, program: &Program) -> HashMap<String, Type> {
        // 1. Registrar structs con sus campos
        for s in &program.structs {
            let fields: StructFields = s
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.field_type.clone()))
                .collect();
            self.struct_registry.insert(s.name.clone(), fields);
        }

        // 2. Registrar firmas de funciones libres
        for func in &program.functions {
            let param_types: Vec<Type> = func.params.iter().map(|p| p.param_type.clone()).collect();
            let ret = Type::from_c_name(func.return_type.as_deref().unwrap_or("void"));
            self.function_registry
                .insert(func.name.clone(), (param_types, ret));
        }

        // 3. Registrar métodos de impl como "Struct::method"
        for imp in &program.impls {
            let struct_name = &imp.struct_name;
            for method in &imp.methods {
                let param_types: Vec<Type> = method
                    .params
                    .iter()
                    // skip &self / &mut self
                    .filter(|p| p.name != "self")
                    .map(|p| p.param_type.clone())
                    .collect();
                let ret = Type::from_c_name(method.return_type.as_deref().unwrap_or("void"));
                let key = format!("{}::{}", struct_name, method.name);
                self.function_registry.insert(key, (param_types, ret));
            }
        }

        // 4. Verificar cuerpos de funciones libres
        for func in &program.functions {
            self.check_function(func);
        }

        // 5. Verificar impl blocks (methods son Function en Impl)
        for imp in &program.impls {
            for func in &imp.methods {
                let struct_type = Type::Named(imp.struct_name.clone());
                self.check_impl_function(func, struct_type);
            }
        }

        // 6. Verificar clases Python-style (herencia, methods son Method)
        for class in &program.classes {
            for method in &class.methods {
                let self_type = Type::Class(class.name.clone());
                self.check_class_method(method, self_type);
            }
        }

        // 7. Top-level statements
        for stmt in &program.statements {
            self.check_stmt(stmt);
        }

        self.symbol_table.clone()
    }

    // =========================================================
    // Funciones y métodos
    // =========================================================

    fn check_function(&mut self, func: &Function) {
        self.symbol_table.clear();
        for param in &func.params {
            self.symbol_table
                .insert(param.name.clone(), param.param_type.clone());
        }
        self.current_return_type = Type::from_c_name(func.return_type.as_deref().unwrap_or("void"));
        for stmt in &func.body {
            self.check_stmt(stmt);
        }
    }

    /// Verificar un método de `impl` (usa ast::Function con posible &self como primer param)
    fn check_impl_function(&mut self, func: &Function, self_type: Type) {
        self.symbol_table.clear();
        // Inyectar `self`
        self.symbol_table.insert("self".to_string(), self_type);
        for param in &func.params {
            if param.name != "self" {
                self.symbol_table
                    .insert(param.name.clone(), param.param_type.clone());
            }
        }
        self.current_return_type = Type::from_c_name(func.return_type.as_deref().unwrap_or("void"));
        for stmt in &func.body {
            self.check_stmt(stmt);
        }
    }

    /// Verificar un método de clase Python-style (usa ast::Method)
    fn check_class_method(&mut self, method: &Method, self_type: Type) {
        self.symbol_table.clear();
        self.symbol_table.insert("self".to_string(), self_type);
        for param in &method.params {
            if param.name != "self" {
                self.symbol_table
                    .insert(param.name.clone(), param.param_type.clone());
            }
        }
        self.current_return_type =
            Type::from_c_name(method.return_type.as_deref().unwrap_or("void"));
        for stmt in &method.body {
            self.check_stmt(stmt);
        }
    }

    // =========================================================
    // Statements
    // =========================================================

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // Asignación sin tipo: let x = ... o x = ...
            Stmt::Assign { name, value } => {
                let val_type = self.infer_expr(value);
                self.symbol_table.insert(name.clone(), val_type);
            }

            // Declaración tipada: int x = 5  /  int* ptr = &x
            Stmt::VarDecl {
                var_type,
                name,
                value,
            } => {
                if let Some(init) = value {
                    let inferred = self.infer_expr(init);
                    if !self.types_compatible(var_type, &inferred) {
                        eprintln!(
                            "⚠️  Warning: '{}' declared as {} but initialized with {}",
                            name, var_type, inferred
                        );
                    }
                }
                self.symbol_table.insert(name.clone(), var_type.clone());
            }

            // Compound: x += 5
            Stmt::CompoundAssign { name, op: _, value } => {
                let _val_type = self.infer_expr(value);
                if !self.symbol_table.contains_key(name.as_str()) {
                    eprintln!(
                        "⚠️  Warning: CompoundAssign to undeclared variable '{}'",
                        name
                    );
                }
            }

            // Asignación a campo: obj.field = value
            Stmt::FieldAssign {
                object,
                field,
                value,
            } => {
                let obj_type = self.infer_expr(object);
                let _val_type = self.infer_expr(value);
                self.check_field_access(&obj_type, field, "assign");
            }

            // Asignación a puntero: *ptr = value
            Stmt::DerefAssign { pointer, value } => {
                let ptr_type = self.infer_expr(pointer);
                let _val_type = self.infer_expr(value);
                if !matches!(ptr_type, Type::Pointer(_)) && ptr_type != Type::Unknown {
                    eprintln!("⚠️  Warning: Dereferencing non-pointer type: {}", ptr_type);
                }
            }

            // Index assign: arr[i] = val
            Stmt::IndexAssign {
                object,
                index: _,
                value: _,
            } => {
                let arr_type = self.infer_expr(object);
                if !matches!(arr_type, Type::Array(_, _) | Type::Pointer(_))
                    && arr_type != Type::Unknown
                {
                    eprintln!("⚠️  Warning: Index assign on non-array type: {}", arr_type);
                }
            }

            // Incremento/Decremento
            Stmt::Increment { name, .. } => {
                if !self.symbol_table.contains_key(name.as_str()) {
                    eprintln!("⚠️  Warning: Increment on undeclared variable '{}'", name);
                }
            }

            // If / else
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.infer_expr(condition);
                for s in then_body {
                    self.check_stmt(s);
                }
                if let Some(eb) = else_body {
                    for s in eb {
                        self.check_stmt(s);
                    }
                }
            }

            // While
            Stmt::While { condition, body } => {
                self.infer_expr(condition);
                for s in body {
                    self.check_stmt(s);
                }
            }

            // Do-While
            Stmt::DoWhile { body, condition } => {
                for s in body {
                    self.check_stmt(s);
                }
                self.infer_expr(condition);
            }

            // For (C-style index)
            Stmt::For {
                var,
                start,
                end,
                body,
            } => {
                let start_type = self.infer_expr(start);
                self.symbol_table.insert(var.clone(), start_type);
                self.infer_expr(end);
                for s in body {
                    self.check_stmt(s);
                }
            }

            // ForEach
            Stmt::ForEach {
                var,
                iterable,
                body,
            } => {
                let iter_type = self.infer_expr(iterable);
                let elem_type = match &iter_type {
                    Type::Array(inner, _) => *inner.clone(),
                    _ => Type::Unknown,
                };
                self.symbol_table.insert(var.clone(), elem_type);
                for s in body {
                    self.check_stmt(s);
                }
            }

            // Switch
            Stmt::Switch {
                expr,
                cases,
                default,
            } => {
                self.infer_expr(expr);
                for case in cases {
                    self.infer_expr(&case.value);
                    for s in &case.body {
                        self.check_stmt(s);
                    }
                }
                if let Some(def) = default {
                    for s in def {
                        self.check_stmt(s);
                    }
                }
            }

            // Return
            Stmt::Return(Some(expr)) => {
                let ret_type = self.infer_expr(expr);
                if self.current_return_type != Type::Void
                    && ret_type != Type::Unknown
                    && self.current_return_type != Type::Unknown
                    && !self.types_compatible(&self.current_return_type.clone(), &ret_type)
                {
                    eprintln!(
                        "⚠️  Warning: Return type mismatch. Expected {}, found {}",
                        self.current_return_type, ret_type
                    );
                }
            }
            Stmt::Return(None) => {
                if self.current_return_type != Type::Void
                    && self.current_return_type != Type::Unknown
                {
                    eprintln!(
                        "⚠️  Warning: Missing return value (expected {})",
                        self.current_return_type
                    );
                }
            }

            Stmt::LineMarker(_) => {}

            // Expresión como statement (llamada a función, etc.)
            Stmt::Expr(expr) => {
                self.infer_expr(expr);
            }

            // Free / Delete — checar que sea puntero
            Stmt::Free(expr) => {
                let t = self.infer_expr(expr);
                if !matches!(t, Type::Pointer(_)) && t != Type::Unknown {
                    eprintln!("⚠️  Warning: free() called on non-pointer type: {}", t);
                }
            }

            // Printf / Print
            Stmt::Print(e) | Stmt::Println(e) | Stmt::PrintNum(e) => {
                self.infer_expr(e);
            }

            // OS-level: ignorar semánticamente (correcto por definición en Modo 1)
            Stmt::Cli
            | Stmt::Sti
            | Stmt::Hlt
            | Stmt::Iret
            | Stmt::Cpuid
            | Stmt::RawBlock { .. }
            | Stmt::OrgDirective { .. }
            | Stmt::AlignDirective { .. }
            | Stmt::FarJump { .. }
            | Stmt::LabelDef { .. }
            | Stmt::JumpTo { .. }
            | Stmt::JumpIfZero { .. }
            | Stmt::JumpIfNotZero { .. }
            | Stmt::JumpIfCarry { .. }
            | Stmt::JumpIfNotCarry { .. }
            | Stmt::DataBytes { .. }
            | Stmt::DataWords { .. }
            | Stmt::DataDwords { .. }
            | Stmt::TimesDirective { .. }
            | Stmt::IntCall { .. }
            | Stmt::PortOut { .. }
            | Stmt::MemWrite { .. }
            | Stmt::RegAssign { .. } => {}

            // Break / Continue / Pass / Assert
            Stmt::Break | Stmt::Continue | Stmt::Pass => {}
            Stmt::Assert { condition, .. } => {
                self.infer_expr(condition);
            }

            // Delete
            Stmt::Delete { expr, .. } => {
                self.infer_expr(expr);
            }

            // ArrowAssign: ptr->field = val
            Stmt::ArrowAssign {
                pointer,
                field: _,
                value,
            } => {
                let ptr_t = self.infer_expr(pointer);
                let _val_t = self.infer_expr(value);
                if !matches!(ptr_t, Type::Pointer(_)) && ptr_t != Type::Unknown {
                    eprintln!("⚠️  Warning: Arrow assign on non-pointer: {}", ptr_t);
                }
            }
        }
    }

    // =========================================================
    // Inferencia de tipos de expresiones
    // =========================================================

    fn infer_expr(&self, expr: &Expr) -> Type {
        match expr {
            // Literales
            Expr::Number(_) => Type::I64,
            Expr::Float(_) => Type::F64,
            Expr::String(_) => Type::Str,
            Expr::Bool(_) => Type::Bool,
            Expr::Null => Type::Unknown,
            Expr::Nullptr => Type::Pointer(Box::new(Type::Void)),

            // Variable
            Expr::Variable(name) => {
                self.symbol_table.get(name).cloned().unwrap_or_else(|| {
                    // No warning: podría ser función global o built-in
                    Type::Unknown
                })
            }

            // Operaciones binarias
            Expr::BinaryOp { left, right, op } => {
                let l = self.infer_expr(left);
                let r = self.infer_expr(right);
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        if l.is_float() || r.is_float() {
                            Type::F64
                        } else if l.is_numeric() && r.is_numeric() {
                            Type::I64
                        } else {
                            Type::Unknown
                        }
                    }
                    BinOp::And | BinOp::Or => Type::Bool,
                }
            }

            // Comparaciones → bool
            Expr::Comparison { .. } => Type::Bool,

            // Operaciones bitwise
            Expr::BitwiseOp {
                left,
                right: _,
                op: _,
            } => self.infer_expr(left),

            // NOT bitwise
            Expr::BitwiseNot(inner) => self.infer_expr(inner),

            // NOT lógico → bool
            Expr::UnaryOp {
                op: UnaryOp::Not, ..
            } => Type::Bool,
            Expr::UnaryOp {
                op: UnaryOp::Neg,
                expr,
            } => self.infer_expr(expr),

            // Deref: *ptr → tipo apuntado
            Expr::Deref(inner) => match self.infer_expr(inner) {
                Type::Pointer(inner_t) => *inner_t,
                _ => Type::Unknown,
            },

            // Address-of: &x → Pointer(T)
            Expr::AddressOf(inner) => {
                let t = self.infer_expr(inner);
                Type::Pointer(Box::new(t))
            }

            // Array literal
            Expr::Array(elements) => {
                let elem_type = elements
                    .first()
                    .map(|e| self.infer_expr(e))
                    .unwrap_or(Type::Unknown);
                Type::Array(Box::new(elem_type), Some(elements.len()))
            }

            // Index: arr[i] → elem type
            Expr::Index { object, index: _ } => match self.infer_expr(object) {
                Type::Array(inner, _) => *inner,
                Type::Pointer(inner) => *inner,
                _ => Type::Unknown,
            },

            // Field access: obj.field
            Expr::FieldAccess { object, field } => {
                let obj_type = self.infer_expr(object);
                self.resolve_field_type(&obj_type, field)
            }

            // Arrow access: ptr->field
            Expr::ArrowAccess { pointer, field } => {
                let ptr_type = self.infer_expr(pointer);
                match ptr_type {
                    Type::Pointer(inner) => self.resolve_field_type(&inner, field),
                    _ => Type::Unknown,
                }
            }

            // Method call: obj.method(args)
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                let obj_type = self.infer_expr(object);
                let struct_name = match &obj_type {
                    Type::Named(n) | Type::Struct(n) | Type::Class(n) => n.clone(),
                    _ => return Type::Unknown,
                };
                let key = format!("{}::{}", struct_name, method);
                if let Some((param_types, ret)) = self.function_registry.get(&key) {
                    // Verificar aridad (sin contar self)
                    if args.len() != param_types.len() {
                        eprintln!(
                            "⚠️  Warning: {}.{}() expects {} args, got {}",
                            struct_name,
                            method,
                            param_types.len(),
                            args.len()
                        );
                    }
                    ret.clone()
                } else {
                    Type::Unknown
                }
            }

            // Struct literal: Punto { x: 1, y: 2 }
            Expr::New { class_name, .. } => Type::Named(class_name.clone()),

            // Function call: add(a, b)
            Expr::Call { name, args } => {
                if let Some((param_types, ret)) = self.function_registry.get(name) {
                    if args.len() != param_types.len() {
                        eprintln!(
                            "⚠️  Warning: {}() expects {} args, got {}",
                            name,
                            param_types.len(),
                            args.len()
                        );
                    }
                    ret.clone()
                } else {
                    // Built-ins: printf → void, etc.
                    Type::Unknown
                }
            }

            // Sizeof → int (u64)
            Expr::SizeOf(_) => Type::U64,

            // Malloc → pointer genérico
            Expr::Malloc(_) => Type::Pointer(Box::new(Type::Void)),

            // Cast explícito
            Expr::Cast { target_type, .. } => target_type.clone(),

            // Type casts
            Expr::IntCast(_) => Type::I64,
            Expr::FloatCast(_) => Type::F64,
            Expr::StrCast(_) => Type::Str,
            Expr::BoolCast(_) => Type::Bool,

            // Pre/post inc/dec
            Expr::PreIncrement(e)
            | Expr::PostIncrement(e)
            | Expr::PreDecrement(e)
            | Expr::PostDecrement(e) => self.infer_expr(e),

            // Ternario
            Expr::Ternary { then_expr, .. } => self.infer_expr(then_expr),

            // Len → int
            Expr::Len(_) => Type::I64,

            // Push / Pop
            Expr::Push { array, .. } => self.infer_expr(array),
            Expr::Pop(arr) => match self.infer_expr(arr) {
                Type::Array(inner, _) => *inner,
                _ => Type::Unknown,
            },

            // Lambda
            Expr::Lambda { body, .. } => self.infer_expr(body),

            // Concat de strings
            Expr::StringConcat { .. } => Type::Str,

            // OS-level
            Expr::RegRead { .. } => Type::U64,
            Expr::MemRead { .. } => Type::U64,
            Expr::PortIn { .. } => Type::U8,
            Expr::CpuidExpr => Type::U32,
            Expr::LabelAddr { .. } => Type::U64,

            // This / Super
            Expr::This => self
                .symbol_table
                .get("self")
                .cloned()
                .unwrap_or(Type::Unknown),
            Expr::Super => Type::Unknown,

            // Input
            Expr::Input => Type::I64,

            // Slice
            Expr::Slice { object, .. } => self.infer_expr(object),

            // Realloc
            Expr::Realloc { ptr, .. } => self.infer_expr(ptr),
        }
    }

    // =========================================================
    // Helpers
    // =========================================================

    /// Resolver el tipo de un campo dado el tipo del objeto
    fn resolve_field_type(&self, obj_type: &Type, field: &str) -> Type {
        let struct_name = match obj_type {
            Type::Named(n) | Type::Struct(n) | Type::Class(n) => n.clone(),
            _ => return Type::Unknown,
        };
        if let Some(fields) = self.struct_registry.get(&struct_name) {
            for (fname, ftype) in fields {
                if fname == field {
                    return ftype.clone();
                }
            }
            eprintln!(
                "⚠️  Warning: struct '{}' has no field '{}'",
                struct_name, field
            );
        }
        Type::Unknown
    }

    /// Verificar acceso a campo (para FieldAssign)
    fn check_field_access(&self, obj_type: &Type, field: &str, context: &str) {
        let struct_name = match obj_type {
            Type::Named(n) | Type::Struct(n) | Type::Class(n) => n.clone(),
            Type::Unknown => return,
            _ => {
                eprintln!(
                    "⚠️  Warning: Field {} '{}' on non-struct type: {}",
                    context, field, obj_type
                );
                return;
            }
        };
        if let Some(fields) = self.struct_registry.get(&struct_name) {
            if !fields.iter().any(|(n, _)| n == field) {
                eprintln!(
                    "⚠️  Warning: struct '{}' has no field '{}' ({})",
                    struct_name, field, context
                );
            }
        }
    }

    /// Compatibilidad de tipos (permisiva para types numéricos y Unknown)
    fn types_compatible(&self, declared: &Type, inferred: &Type) -> bool {
        if declared == inferred {
            return true;
        }
        if *inferred == Type::Unknown || *declared == Type::Unknown {
            return true;
        }
        // Numérico ↔ numérico es compatible (conversión implícita)
        if declared.is_numeric() && inferred.is_numeric() {
            return true;
        }
        // Puntero ↔ nullptr
        if declared.is_pointer() && *inferred == Type::Pointer(Box::new(Type::Void)) {
            return true;
        }
        false
    }
}
