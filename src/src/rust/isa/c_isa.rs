// ============================================================
// ADead-BIB — C99 ISA Compiler
// ============================================================
// Specialized compiler for C99 with real sizeof semantics.
//
// Key differences from adead_isa (isa_compiler.rs):
//   - sizeof(char)=1, sizeof(short)=2, sizeof(int)=4, sizeof(long)=8
//   - Struct field offsets use real C99 alignment rules
//   - No vtable, no this pointer, no inheritance
//   - MSVC x64 ABI calling convention
//
// Shares: ADeadOp, Reg, Operand, Encoder (from mod.rs / encoder.rs)
//
// Inspired by: FASM determinism, GCC layout, MSVC x64 ABI
//
// Autor: Eddi Andreé Salazar Matos
// ============================================================

use super::isa_compiler::{ClassLayout, IsaCompiler, Target};
use crate::frontend::ast::Type;

// ============================================================
// C99 Size Policy — Real sizeof semantics
// ============================================================

/// Returns the real C99 sizeof for a given Type.
/// MSVC x64: char=1, short=2, int=4, long=4(MSVC)/8(LP64), long long=8,
///           float=4, double=8, pointer=8
pub fn c99_sizeof(ty: &Type) -> i32 {
    match ty {
        // Exact-width integer types
        Type::I8 | Type::U8 | Type::Bool => 1, // char, unsigned char, _Bool
        Type::I16 | Type::U16 => 2,            // short, unsigned short
        Type::I32 | Type::U32 => 4,            // int, unsigned int
        Type::I64 | Type::U64 => 8,            // long long, unsigned long long
        Type::F32 => 4,                        // float
        Type::F64 => 8,                        // double

        // C named types
        Type::Named(name) => match name.as_str() {
            "char" | "signed char" | "unsigned char" | "_Bool" => 1,
            "short" | "unsigned short" => 2,
            "int" | "unsigned int" | "unsigned" => 4,
            "long" | "unsigned long" => 8, // LP64 model (Linux/macOS)
            "long long" | "unsigned long long" => 8,
            "float" => 4,
            "double" => 8,
            "long double" => 8, // Simplified: treat as double
            "size_t" | "ptrdiff_t" | "intptr_t" | "uintptr_t" => 8,
            // Windows typedefs — 1 byte
            "BYTE" | "UINT8" => 1,
            // Windows typedefs — 2 bytes
            "WORD" | "UINT16" | "WCHAR" => 2,
            // Windows typedefs — 4 bytes
            "UINT" | "UINT32" | "DWORD" | "INT" | "LONG" | "HRESULT" | "BOOL" | "FLOAT" => 4,
            // Windows typedefs — 8 bytes (pointers and 64-bit integers)
            "UINT64" | "ULONG_PTR" | "SIZE_T" | "ULONGLONG" | "LONGLONG" | "INT64"
            | "WPARAM" | "LPARAM" | "LRESULT"
            | "HANDLE" | "HWND" | "HINSTANCE" | "HMODULE" | "HDC" | "HICON" | "HCURSOR"
            | "HBRUSH" | "HMENU" | "HMONITOR" | "LPVOID" | "LPCVOID"
            | "LPSTR" | "LPCSTR" | "LPWSTR" | "LPCWSTR" | "WNDPROC" => 8,
            "void" => 0,
            _ => 8, // Unknown named type → pointer-sized default
        },

        // Pointers are always 8 bytes on x64
        Type::Pointer(_) => 8,

        // Arrays: element_size * count
        Type::Array(inner, Some(count)) => c99_sizeof(inner) * (*count as i32),
        Type::Array(_, None) => 8, // Flexible array → pointer

        // Struct/Class types — resolved dynamically via class_layouts
        Type::Struct(_) | Type::Class(_) => 0, // Sentinel: look up in layouts

        // Void
        Type::Void => 0,

        // Default: 8 bytes (pointer-sized)
        _ => 8,
    }
}

/// Compute C99-compliant field alignment for a given size.
/// MSVC x64: alignment = min(sizeof(field), 8)
pub fn c99_align(size: i32) -> i32 {
    match size {
        0 => 1,
        1 => 1,
        2 => 2,
        3..=4 => 4,
        _ => 8,
    }
}

/// Align an offset to the given alignment boundary.
pub fn align_to(offset: i32, align: i32) -> i32 {
    if align <= 1 {
        return offset;
    }
    (offset + align - 1) & !(align - 1)
}

// ============================================================
// C99 ISA Compiler — Wraps IsaCompiler with C99 policy
// ============================================================

/// C99-specialized ISA compiler.
///
/// Uses real sizeof/alignment for struct layouts instead of
/// the default 8-byte-everything policy of adead_isa.
pub struct CIsaCompiler {
    inner: IsaCompiler,
}

impl CIsaCompiler {
    /// Create a new C99 ISA compiler for the given target.
    pub fn new(target: Target) -> Self {
        Self {
            inner: IsaCompiler::new(target),
        }
    }

    /// Compile a C99 program with real sizeof semantics.
    ///
    /// The key difference: struct layouts use C99 alignment rules
    /// instead of 8-byte-everything.
    pub fn compile(
        &mut self,
        program: &crate::frontend::ast::Program,
    ) -> (Vec<u8>, Vec<u8>, Vec<usize>, Vec<usize>) {
        // Override struct layouts with C99-compliant offsets BEFORE compilation
        self.register_c99_layouts(program);

        // Delegate to the base compiler (which already registered layouts in Fase 0,
        // but we override them here with correct C99 layouts)
        self.inner.compile(program)
    }

    /// Register struct layouts with real C99 sizeof/alignment.
    ///
    /// For each struct in the program, computes field offsets using
    /// real type sizes and natural alignment, matching what GCC/Clang
    /// would produce on x86-64 with LP64 model.
    fn register_c99_layouts(&mut self, program: &crate::frontend::ast::Program) {
        for st in &program.structs {
            let mut fields = Vec::new();
            let mut offset = 0i32;
            let mut max_align = 1i32;

            for field in &st.fields {
                let field_size = c99_sizeof(&field.field_type);

                // For struct fields, look up the struct size from already-registered layouts
                let actual_size = if field_size == 0 {
                    // It's a struct type — look up its layout
                    match &field.field_type {
                        Type::Struct(name) | Type::Named(name) | Type::Class(name) => self
                            .inner
                            .class_layouts()
                            .get(name)
                            .map(|l| l.size)
                            .unwrap_or(8),
                        _ => 8,
                    }
                } else {
                    field_size
                };

                let field_align = c99_align(actual_size);
                if field_align > max_align {
                    max_align = field_align;
                }

                // Align the current offset
                offset = align_to(offset, field_align);
                fields.push((field.name.clone(), offset));

                // Advance by field size (minimum 8 for stack slot compatibility)
                // NOTE: We use max(actual_size, 8) for stack layout because
                // the ISA compiler stores all values as 64-bit qwords on stack.
                // But for sizeof reporting, we use the real size.
                offset += 8.max(actual_size);
            }

            // Align total struct size to its largest member alignment
            let total_size = align_to(offset, max_align.max(8));

            // Compute real C99 sizeof (sum of actual field sizes with alignment)
            let mut real_offset = 0i32;
            for field in &st.fields {
                let fs = c99_sizeof(&field.field_type);
                let actual_fs = if fs == 0 {
                    match &field.field_type {
                        Type::Struct(n) | Type::Named(n) | Type::Class(n) => self
                            .inner
                            .class_layouts()
                            .get(n)
                            .map(|l| l.real_size)
                            .unwrap_or(8),
                        _ => 8,
                    }
                } else {
                    fs
                };
                let fa = c99_align(actual_fs);
                real_offset = align_to(real_offset, fa);
                real_offset += actual_fs;
            }
            let real_size = align_to(real_offset, max_align);

            self.inner.insert_class_layout(
                st.name.clone(),
                ClassLayout {
                    name: st.name.clone(),
                    fields,
                    field_types: vec![],
                    size: total_size,
                    real_size,
                },
            );
        }
    }

    /// Access the underlying IR for optimization passes.
    pub fn ir(&self) -> &super::ADeadIR {
        self.inner.ir()
    }
}

// ============================================================
// C99 sizeof lookup for the SizeOf expression
// ============================================================

/// Resolve sizeof for C99 types at compile time.
/// Used by the ISA compiler when encountering Expr::SizeOf.
pub fn c99_sizeof_for_expr(ty: &Type) -> i64 {
    match ty {
        Type::I8 | Type::U8 | Type::Bool => 1,
        Type::I16 | Type::U16 => 2,
        Type::I32 | Type::U32 => 4,
        Type::I64 | Type::U64 => 8,
        Type::F32 => 4,
        Type::F64 => 8,
        Type::Pointer(_) => 8,
        Type::Named(name) => match name.as_str() {
            "char" | "signed char" | "unsigned char" | "_Bool" => 1,
            "short" | "unsigned short" => 2,
            "int" | "unsigned int" | "unsigned" => 4,
            "long" | "unsigned long" => 8,
            "long long" | "unsigned long long" => 8,
            "float" => 4,
            "double" => 8,
            "size_t" | "ptrdiff_t" => 8,
            _ => 8,
        },
        _ => 8,
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c99_sizeof_basic() {
        assert_eq!(c99_sizeof(&Type::I8), 1);
        assert_eq!(c99_sizeof(&Type::I16), 2);
        assert_eq!(c99_sizeof(&Type::I32), 4);
        assert_eq!(c99_sizeof(&Type::I64), 8);
        assert_eq!(c99_sizeof(&Type::F32), 4);
        assert_eq!(c99_sizeof(&Type::F64), 8);
        assert_eq!(c99_sizeof(&Type::Bool), 1);
    }

    #[test]
    fn test_c99_sizeof_named() {
        assert_eq!(c99_sizeof(&Type::Named("char".to_string())), 1);
        assert_eq!(c99_sizeof(&Type::Named("short".to_string())), 2);
        assert_eq!(c99_sizeof(&Type::Named("int".to_string())), 4);
        assert_eq!(c99_sizeof(&Type::Named("long".to_string())), 8);
        assert_eq!(c99_sizeof(&Type::Named("double".to_string())), 8);
    }

    #[test]
    fn test_c99_sizeof_pointer() {
        assert_eq!(c99_sizeof(&Type::Pointer(Box::new(Type::I32))), 8);
        assert_eq!(c99_sizeof(&Type::Pointer(Box::new(Type::I8))), 8);
    }

    #[test]
    fn test_c99_align() {
        assert_eq!(c99_align(1), 1);
        assert_eq!(c99_align(2), 2);
        assert_eq!(c99_align(4), 4);
        assert_eq!(c99_align(8), 8);
    }

    #[test]
    fn test_align_to() {
        assert_eq!(align_to(0, 4), 0);
        assert_eq!(align_to(1, 4), 4);
        assert_eq!(align_to(3, 4), 4);
        assert_eq!(align_to(4, 4), 4);
        assert_eq!(align_to(5, 8), 8);
    }
}
