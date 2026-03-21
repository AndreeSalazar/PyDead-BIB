// ============================================================
// ADead-BIB v7.0 — Test-Canon Integration Tests
// ============================================================
// Verifica que TODOS los archivos en Test-Canon/ compilan
// a traves del pipeline completo de ADead-BIB.
//
// C99 Canon: 18 archivos (tipos, punteros, structs, etc.)
// C++98 Canon: 15 archivos (clases, herencia, templates, etc.)
// ============================================================

#[cfg(test)]
mod tests {
    use std::path::Path;

    fn canon_c_path(name: &str) -> String {
        format!("Test-Canon/C99/{}", name)
    }

    fn canon_cpp_path(name: &str) -> String {
        format!("Test-Canon/Cpp98/{}", name)
    }

    // ================================================================
    // C99 Canon Tests — All 18 files
    // ================================================================

    macro_rules! canon_c_test {
        ($name:ident, $file:expr) => {
            #[test]
            fn $name() {
                let path = canon_c_path($file);
                if !Path::new(&path).exists() {
                    eprintln!("SKIP: {} not found", path);
                    return;
                }
                let source = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Cannot read {}: {}", path, e));
                let result = crate::frontend::c::c_to_ir::compile_c_to_program(&source);
                assert!(
                    result.is_ok(),
                    "Canon C99 {} failed: {}",
                    $file,
                    result.unwrap_err()
                );
                let prog = result.unwrap();
                assert!(
                    prog.functions.len() >= 1,
                    "Canon {} must have at least main()",
                    $file
                );
            }
        };
    }

    canon_c_test!(test_canon_c99_01_tipos, "01_tipos_fundamentales.c");
    canon_c_test!(test_canon_c99_02_punteros, "02_punteros_autenticos.c");
    canon_c_test!(test_canon_c99_03_arrays, "03_arrays_memoria.c");
    canon_c_test!(test_canon_c99_04_structs, "04_structs_layout.c");
    canon_c_test!(test_canon_c99_05_unions, "05_unions_memoria.c");
    canon_c_test!(test_canon_c99_06_enums, "06_enums_constantes.c");
    canon_c_test!(test_canon_c99_07_typedef, "07_typedef_alias.c");
    canon_c_test!(test_canon_c99_08_control, "08_control_flujo.c");
    canon_c_test!(test_canon_c99_09_funciones, "09_funciones_calling.c");
    canon_c_test!(test_canon_c99_10_punteros_fn, "10_punteros_funcion.c");
    canon_c_test!(test_canon_c99_11_preprocesador, "11_preprocesador.c");
    canon_c_test!(test_canon_c99_12_bitwise, "12_bitwise_operadores.c");
    canon_c_test!(test_canon_c99_13_casting, "13_casting_tipos.c");
    canon_c_test!(test_canon_c99_14_scope, "14_scope_lifetime.c");
    canon_c_test!(test_canon_c99_15_strings, "15_string_operaciones.c");
    canon_c_test!(test_canon_c99_16_malloc, "16_malloc_free.c");
    canon_c_test!(test_canon_c99_17_sizeof, "17_sizeof_alineacion.c");
    canon_c_test!(test_canon_c99_18_expresiones, "18_expresiones_complejas.c");

    // ================================================================
    // C++98 Canon Tests — All 15 files
    // ================================================================

    macro_rules! canon_cpp_test {
        ($name:ident, $file:expr) => {
            #[test]
            fn $name() {
                let path = canon_cpp_path($file);
                if !Path::new(&path).exists() {
                    eprintln!("SKIP: {} not found", path);
                    return;
                }
                let source = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Cannot read {}: {}", path, e));
                let result = crate::frontend::cpp::cpp_to_ir::compile_cpp_to_program(&source);
                assert!(
                    result.is_ok(),
                    "Canon C++98 {} failed: {}",
                    $file,
                    result.unwrap_err()
                );
                let prog = result.unwrap();
                assert!(
                    prog.functions.len() >= 1,
                    "Canon {} must have at least main()",
                    $file
                );
            }
        };
    }

    canon_cpp_test!(test_canon_cpp98_01_clases, "01_clases_basicas.cpp");
    canon_cpp_test!(test_canon_cpp98_02_herencia, "02_herencia.cpp");
    canon_cpp_test!(test_canon_cpp98_03_virtual, "03_virtual_polimorfismo.cpp");
    canon_cpp_test!(test_canon_cpp98_04_templates_fn, "04_templates_funcion.cpp");
    canon_cpp_test!(test_canon_cpp98_05_templates_cls, "05_templates_clase.cpp");
    canon_cpp_test!(test_canon_cpp98_06_namespaces, "06_namespaces.cpp");
    canon_cpp_test!(test_canon_cpp98_07_operator, "07_operator_overload.cpp");
    canon_cpp_test!(test_canon_cpp98_08_referencias, "08_referencias.cpp");
    canon_cpp_test!(test_canon_cpp98_09_const, "09_const_correctness.cpp");
    canon_cpp_test!(test_canon_cpp98_10_encapsulamiento, "10_encapsulamiento.cpp");
    canon_cpp_test!(test_canon_cpp98_11_constructores, "11_constructores_avanzados.cpp");
    canon_cpp_test!(test_canon_cpp98_12_static, "12_static_members.cpp");
    canon_cpp_test!(test_canon_cpp98_13_punteros_obj, "13_punteros_objetos.cpp");
    canon_cpp_test!(test_canon_cpp98_14_enum_class, "14_enum_class.cpp");
    canon_cpp_test!(test_canon_cpp98_15_stl, "15_stl_basico.cpp");
}
