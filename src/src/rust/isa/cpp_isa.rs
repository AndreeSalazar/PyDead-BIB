// ============================================================
// ADead-BIB — C++98 ISA Compiler
// ============================================================
// Specialized compiler for C++98 with vtable, this pointer,
// and MSVC x64 class layout semantics.
//
// Key differences from adead_isa (isa_compiler.rs):
//   - Classes with vtable pointer at offset 0 (when virtual methods exist)
//   - this pointer in RCX (MSVC x64 thiscall)
//   - Inheritance: base class fields first, then derived fields
//   - sizeof follows C++ rules (includes vtable ptr if virtual)
//   - Name mangling: Class::method
//
// Key differences from c_isa:
//   - vtable support for virtual dispatch
//   - this pointer ABI (implicit first parameter)
//   - Constructor/destructor semantics
//   - Inheritance layout (base subobject at offset 0)
//
// Shares: ADeadOp, Reg, Operand, Encoder (from mod.rs / encoder.rs)
//
// Inspired by: MSVC x64 ABI, Itanium C++ ABI (GCC/Clang), FASM
//
// Autor: Eddi Andreé Salazar Matos
// ============================================================

use super::isa_compiler::{ClassLayout, IsaCompiler, Target};
use crate::frontend::ast::{Class, Program, Type};

// ============================================================
// C++ Size Policy — sizeof with vtable awareness
// ============================================================

/// Returns the C++ sizeof for a given Type.
/// Same as C99 for primitives, but classes may include vtable pointer.
pub fn cpp_sizeof(ty: &Type) -> i32 {
    match ty {
        Type::I8 | Type::U8 | Type::Bool => 1,
        Type::I16 | Type::U16 => 2,
        Type::I32 | Type::U32 => 4,
        Type::I64 | Type::U64 => 8,
        Type::F32 => 4,
        Type::F64 => 8,
        Type::Pointer(_) => 8,
        Type::Named(name) => match name.as_str() {
            "char" | "signed char" | "unsigned char" | "bool" => 1,
            "short" | "unsigned short" => 2,
            "int" | "unsigned int" | "unsigned" => 4,
            "long" | "unsigned long" => 8,
            "long long" | "unsigned long long" => 8,
            "float" => 4,
            "double" => 8,
            "size_t" | "ptrdiff_t" => 8,
            "void" => 0,
            _ => 8,
        },
        Type::Array(inner, Some(count)) => cpp_sizeof(inner) * (*count as i32),
        Type::Array(_, None) => 8,
        Type::Struct(_) | Type::Class(_) => 0, // Resolved via class_layouts
        Type::Void => 0,
        _ => 8,
    }
}

// ============================================================
// C++98 ISA Compiler — Wraps IsaCompiler with C++ policy
// ============================================================

/// C++98-specialized ISA compiler.
///
/// Adds vtable layout, this-pointer ABI, and inheritance
/// layout on top of the base IsaCompiler.
pub struct CppIsaCompiler {
    inner: IsaCompiler,
}

impl CppIsaCompiler {
    /// Create a new C++98 ISA compiler for the given target.
    pub fn new(target: Target) -> Self {
        Self {
            inner: IsaCompiler::new(target),
        }
    }

    /// Compile a C++98 program with vtable and this-pointer semantics.
    pub fn compile(&mut self, program: &Program) -> (Vec<u8>, Vec<u8>, Vec<usize>, Vec<usize>) {
        // Register C++ class layouts with inheritance and vtable awareness
        self.register_cpp_layouts(program);

        // Delegate to the base compiler
        self.inner.compile(program)
    }

    /// Register class layouts with C++ semantics:
    /// - Base class fields first (inheritance)
    /// - vtable pointer at offset 0 if any virtual methods
    /// - this pointer is implicit first argument (MSVC: RCX)
    fn register_cpp_layouts(&mut self, program: &Program) {
        // First pass: register structs (same as C99 but with 8-byte slots)
        for st in &program.structs {
            let mut fields = Vec::new();
            let mut offset = 0i32;
            for field in &st.fields {
                fields.push((field.name.clone(), offset));
                offset += 8;
            }
            self.inner.insert_class_layout(
                st.name.clone(),
                ClassLayout {
                    name: st.name.clone(),
                    fields,
                    field_types: vec![],
                    size: offset,
                    real_size: offset,
                },
            );
        }

        // Second pass: register classes with inheritance
        for class in &program.classes {
            let layout = self.compute_class_layout(class, &program.classes);
            self.inner.insert_class_layout(class.name.clone(), layout);
        }
    }

    /// Compute a class layout considering inheritance.
    ///
    /// Layout order (MSVC x64 / Itanium ABI):
    /// 1. vtable pointer (if class has virtual methods) — 8 bytes at offset 0
    /// 2. Base class fields (if inheriting) — copied from base layout
    /// 3. Own fields — appended after base
    fn compute_class_layout(&self, class: &Class, all_classes: &[Class]) -> ClassLayout {
        let mut fields = Vec::new();
        let mut offset = 0i32;

        // Check if class has virtual methods (needs vtable pointer)
        let has_virtual = class.methods.iter().any(|m| m.is_virtual);
        if has_virtual {
            fields.push(("__vtable".to_string(), 0));
            offset = 8; // vtable pointer takes 8 bytes
        }

        // Inherit base class fields
        if let Some(ref base_name) = class.parent {
            // Look up base class layout (already registered or compute recursively)
            if let Some(base_layout) = self.inner.class_layouts().get(base_name) {
                for (field_name, field_offset) in &base_layout.fields {
                    if field_name == "__vtable" {
                        continue;
                    } // Don't duplicate vtable
                    fields.push((field_name.clone(), offset + field_offset));
                }
                offset += base_layout.size;
            } else {
                // Try to find base class in the program and compute its layout
                if let Some(base_class) = all_classes.iter().find(|c| c.name == *base_name) {
                    let base_layout = self.compute_class_layout(base_class, all_classes);
                    for (field_name, field_offset) in &base_layout.fields {
                        if field_name == "__vtable" {
                            continue;
                        }
                        fields.push((field_name.clone(), offset + field_offset));
                    }
                    offset += base_layout.size;
                }
            }
        }

        // Own fields (8-byte aligned, MSVC x64)
        for field in &class.fields {
            fields.push((field.name.clone(), offset));
            offset += 8;
        }

        // Minimum size is 8 (empty class optimization not applied for simplicity)
        if offset == 0 {
            offset = 8;
        }

        ClassLayout {
            name: class.name.clone(),
            fields,
            field_types: vec![],
            size: offset,
            real_size: offset,
        }
    }

    /// Access the underlying IR for optimization passes.
    pub fn ir(&self) -> &super::ADeadIR {
        self.inner.ir()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_sizeof_basic() {
        assert_eq!(cpp_sizeof(&Type::I8), 1);
        assert_eq!(cpp_sizeof(&Type::I32), 4);
        assert_eq!(cpp_sizeof(&Type::I64), 8);
        assert_eq!(cpp_sizeof(&Type::Pointer(Box::new(Type::I32))), 8);
    }

    #[test]
    fn test_cpp_sizeof_named() {
        assert_eq!(cpp_sizeof(&Type::Named("int".to_string())), 4);
        assert_eq!(cpp_sizeof(&Type::Named("bool".to_string())), 1);
        assert_eq!(cpp_sizeof(&Type::Named("double".to_string())), 8);
    }
}
