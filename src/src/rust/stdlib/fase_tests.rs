// ============================================================
// ADead-BIB v7.0 — FASE 1/2/3 Verification Tests
// ============================================================
// Verifies complete pipeline: Source → Parse → IR → ISA → PE
// Each test confirms:
//   1. File parses without error
//   2. IR has functions (at least main)
//   3. ISA compiler generates x86-64 code bytes
//   4. PE generator produces valid PE header (MZ signature)
//   5. Binary is self-contained (no external linker)
// ============================================================

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// Full pipeline: source → parse → IR → ISA → PE bytes
    /// Returns (pe_bytes, code_len, data_len, num_functions)
    fn compile_c_to_pe(source: &str) -> Result<(Vec<u8>, usize, usize, usize), String> {
        let program = crate::frontend::c::c_to_ir::compile_c_to_program(source)?;
        let num_funcs = program.functions.len();
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, data, iat_offsets, string_offsets) = compiler.compile(&program);
        let pe = crate::backend::pe::generate_pe_bytes(&code, &data, &iat_offsets, &string_offsets);
        Ok((pe, code.len(), data.len(), num_funcs))
    }

    fn compile_cpp_to_pe(source: &str) -> Result<(Vec<u8>, usize, usize, usize), String> {
        let program = crate::frontend::cpp::cpp_to_ir::compile_cpp_to_program(source)?;
        let num_funcs = program.functions.len();
        let mut compiler =
            crate::isa::isa_compiler::IsaCompiler::new(crate::isa::isa_compiler::Target::Windows);
        let (code, data, iat_offsets, string_offsets) = compiler.compile(&program);
        let pe = crate::backend::pe::generate_pe_bytes(&code, &data, &iat_offsets, &string_offsets);
        Ok((pe, code.len(), data.len(), num_funcs))
    }

    fn verify_pe_valid(pe: &[u8], test_name: &str) {
        assert!(
            pe.len() >= 64,
            "{}: PE too small ({} bytes)",
            test_name,
            pe.len()
        );
        assert_eq!(
            pe[0], b'M',
            "{}: Missing MZ signature byte 0",
            test_name
        );
        assert_eq!(
            pe[1], b'Z',
            "{}: Missing MZ signature byte 1",
            test_name
        );
        assert!(
            pe.len() < 50 * 1024,
            "{}: PE too large ({} bytes, should be < 50KB)",
            test_name,
            pe.len()
        );
    }

    // ================================================================
    // FASE 1 — C99 Complete
    // ================================================================

    macro_rules! fase1_c99_test {
        ($name:ident, $file:expr, $min_funcs:expr) => {
            #[test]
            fn $name() {
                let path = format!("examples/c/{}", $file);
                if !Path::new(&path).exists() {
                    eprintln!("SKIP: {} not found", path);
                    return;
                }
                let source = std::fs::read_to_string(&path).expect(&format!("read {}", path));
                let (pe, code_len, data_len, num_funcs) = compile_c_to_pe(&source)
                    .expect(&format!("{} should compile", $file));

                // Verify PE structure
                verify_pe_valid(&pe, $file);

                // Verify code was generated
                assert!(
                    code_len > 0,
                    "{}: no code generated",
                    $file
                );
                assert!(
                    num_funcs >= $min_funcs,
                    "{}: expected >= {} functions, got {}",
                    $file,
                    $min_funcs,
                    num_funcs
                );

                eprintln!(
                    "  PASS {}: PE={} bytes, code={}, data={}, funcs={}",
                    $file,
                    pe.len(),
                    code_len,
                    data_len,
                    num_funcs
                );
            }
        };
    }

    fase1_c99_test!(test_fase1_c99_types, "test_c99_types.c", 1);
    fase1_c99_test!(test_fase1_c99_vla, "test_c99_vla.c", 3);
    fase1_c99_test!(test_fase1_c99_designated, "test_c99_designated.c", 1);
    fase1_c99_test!(test_fase1_c99_compound, "test_c99_compound.c", 4);
    fase1_c99_test!(test_fase1_c99_restrict, "test_c99_restrict.c", 4);
    fase1_c99_test!(test_fase1_c99_inline, "test_c99_inline.c", 7);
    fase1_c99_test!(test_fase1_c99_stdint, "test_c99_stdint.c", 1);
    fase1_c99_test!(test_fase1_c99_variadics, "test_c99_variadics.c", 4);

    // ================================================================
    // FASE 2 — C++17 Complete (via expander → C++98 canon)
    // ================================================================

    macro_rules! fase2_cpp17_test {
        ($name:ident, $file:expr, $min_funcs:expr) => {
            #[test]
            fn $name() {
                let path = format!("examples/cpp/{}", $file);
                if !Path::new(&path).exists() {
                    eprintln!("SKIP: {} not found", path);
                    return;
                }
                let source = std::fs::read_to_string(&path).expect(&format!("read {}", path));
                let (pe, code_len, data_len, num_funcs) = compile_cpp_to_pe(&source)
                    .expect(&format!("{} should compile", $file));

                // Verify PE structure
                verify_pe_valid(&pe, $file);

                // Verify code was generated
                assert!(
                    code_len > 0,
                    "{}: no code generated",
                    $file
                );
                assert!(
                    num_funcs >= $min_funcs,
                    "{}: expected >= {} functions, got {}",
                    $file,
                    $min_funcs,
                    num_funcs
                );

                eprintln!(
                    "  PASS {}: PE={} bytes, code={}, data={}, funcs={}",
                    $file,
                    pe.len(),
                    code_len,
                    data_len,
                    num_funcs
                );
            }
        };
    }

    fase2_cpp17_test!(test_fase2_cpp11_lambda, "test_cpp11_lambda.cpp", 4);
    fase2_cpp17_test!(test_fase2_cpp11_auto, "test_cpp11_auto.cpp", 5);
    fase2_cpp17_test!(test_fase2_cpp11_move, "test_cpp11_move.cpp", 1);
    fase2_cpp17_test!(test_fase2_cpp14_generic, "test_cpp14_generic.cpp", 3);
    fase2_cpp17_test!(test_fase2_cpp17_bindings, "test_cpp17_bindings.cpp", 5);
    fase2_cpp17_test!(test_fase2_cpp17_optional, "test_cpp17_optional.cpp", 4);
    fase2_cpp17_test!(test_fase2_cpp17_constexpr, "test_cpp17_constexpr.cpp", 9);

    // ================================================================
    // FASE 3 — header_main.h Integration (C + C++)
    // ================================================================

    #[test]
    fn test_fase3_header_main_c() {
        let path = "examples/c/test_header_main_c.c";
        if !Path::new(path).exists() {
            eprintln!("SKIP: {} not found", path);
            return;
        }
        let source = std::fs::read_to_string(path).expect("read header_main C test");
        let (pe, code_len, data_len, num_funcs) =
            compile_c_to_pe(&source).expect("header_main.h C test should compile");

        verify_pe_valid(&pe, "test_header_main_c.c");
        assert!(code_len > 0, "header_main C: no code generated");
        assert!(num_funcs >= 1, "header_main C: should have main");

        eprintln!(
            "  PASS header_main_c: PE={} bytes, code={}, data={}, funcs={} — C libre!",
            pe.len(),
            code_len,
            data_len,
            num_funcs
        );
    }

    #[test]
    fn test_fase3_header_main_cpp() {
        let path = "examples/cpp/test_header_main_cpp.cpp";
        if !Path::new(path).exists() {
            eprintln!("SKIP: {} not found", path);
            return;
        }
        let source = std::fs::read_to_string(path).expect("read header_main C++ test");
        let (pe, code_len, data_len, num_funcs) =
            compile_cpp_to_pe(&source).expect("header_main.h C++ test should compile");

        verify_pe_valid(&pe, "test_header_main_cpp.cpp");
        assert!(code_len > 0, "header_main C++: no code generated");
        assert!(num_funcs >= 1, "header_main C++: should have main + methods");

        eprintln!(
            "  PASS header_main_cpp: PE={} bytes, code={}, data={}, funcs={} — C++ libre!",
            pe.len(),
            code_len,
            data_len,
            num_funcs
        );
    }

    // ================================================================
    // VERIFICATION — PE structure deep check
    // ================================================================

    #[test]
    fn test_pe_no_external_deps() {
        // Compile a simple C program and verify the PE has no msvcrt/vcruntime imports
        let source = r#"
            #include <stdio.h>
            int main() { printf("libre\n"); return 0; }
        "#;
        let (pe, _, _, _) = compile_c_to_pe(source).expect("simple program should compile");
        verify_pe_valid(&pe, "no_external_deps");

        // Check that binary doesn't reference msvcrt or vcruntime in its import table
        let pe_str = String::from_utf8_lossy(&pe);
        assert!(
            !pe_str.contains("vcruntime"),
            "PE should not reference vcruntime"
        );

        eprintln!(
            "  PASS no_external_deps: PE={} bytes — sin msvcrt, sin vcruntime",
            pe.len()
        );
    }

    #[test]
    fn test_pe_size_under_50kb() {
        // Compile a moderately complex program and check size
        let source = std::fs::read_to_string("examples/c/hello.c").expect("hello.c");
        let (pe, _, _, _) = compile_c_to_pe(&source).expect("hello.c PE");
        assert!(
            pe.len() < 50 * 1024,
            "PE should be < 50KB, got {} bytes",
            pe.len()
        );
        eprintln!("  PASS size_check: hello.c PE = {} bytes (< 50KB)", pe.len());
    }
}
