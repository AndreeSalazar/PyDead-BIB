// ============================================================
// ADead-BIB v7.0 — Integration Tests
// ============================================================
// Verifica que el pipeline completo funciona con header_main.h
// SIN LINKER EXTERNO — NUNCA
//
// Tests:
//   1. C: #include <header_main.h> + printf → compila a IR
//   2. C++: #include <header_main.h> + std::cout → compila a IR
//   3. C: fastos_*.h individuales → compilan
//   4. Verify symbol registries connected
//   5. Verify no linker needed (all symbols resolved internally)
//   6. header_main.h resolver → real content (not placeholders)
// ============================================================

#[cfg(test)]
mod tests {
    // ================================================================
    // Test 1: C with #include <header_main.h> — printf
    // ================================================================
    // Filosofía v7.0: Un solo include. Todo disponible.
    // Hello World con header_main.h → debe compilar sin linker.
    // ================================================================

    #[test]
    fn test_c_header_main_printf() {
        use crate::frontend::c::c_to_ir::compile_c_to_program;

        let source = r#"
            #include <header_main.h>

            int main() {
                printf("Hello from ADead-BIB v7.0!\n");
                printf("Sin linker externo — NUNCA\n");
                int x = 42;
                printf("x = %d\n", x);
                return 0;
            }
        "#;

        let program = compile_c_to_program(source)
            .expect("C header_main.h + printf debe compilar sin error");

        assert_eq!(program.functions.len(), 1, "Debe tener 1 función: main");
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn test_c_header_main_stdlib_functions() {
        use crate::frontend::c::c_to_ir::compile_c_to_program;

        let source = r#"
            #include <header_main.h>

            int main() {
                void *p = malloc(1024);
                if (p) {
                    memset(p, 0, 1024);
                    size_t len = strlen("ADead-BIB");
                    printf("len = %lu\n", len);
                    free(p);
                }
                double s = sin(3.14159);
                double c = cos(0.0);
                return 0;
            }
        "#;

        let program = compile_c_to_program(source)
            .expect("C header_main.h + malloc/memset/strlen/sin/cos debe compilar");

        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn test_c_header_main_multi_function() {
        use crate::frontend::c::c_to_ir::compile_c_to_program;

        let source = r#"
            #include <header_main.h>

            int add(int a, int b) {
                return a + b;
            }

            int main() {
                int result = add(10, 20);
                printf("result = %d\n", result);
                return 0;
            }
        "#;

        let program = compile_c_to_program(source)
            .expect("C header_main.h + multi-function debe compilar");

        assert!(program.functions.len() >= 2, "Debe tener al menos 2 funciones");
    }

    // ================================================================
    // Test 2: C++ with #include <header_main.h> — std::cout
    // ================================================================
    // C++ mode: header_main.h inyecta declaraciones C + reconoce STL.
    // std::cout << "..." es reconocido por parser prescan.
    // ================================================================

    #[test]
    fn test_cpp_header_main_printf() {
        use crate::frontend::cpp::cpp_to_ir::compile_cpp_to_program;

        let source = r#"
            #include <header_main.h>

            int main() {
                printf("Hello from C++ with header_main.h!\n");
                return 0;
            }
        "#;

        let program = compile_cpp_to_program(source)
            .expect("C++ header_main.h + printf debe compilar sin error");

        assert!(program.functions.len() >= 1, "should have at least main");
        assert!(program.functions.iter().any(|f| f.name == "main"), "should have main function");
    }

    #[test]
    fn test_cpp_header_main_cpp_features() {
        use crate::frontend::cpp::cpp_to_ir::compile_cpp_to_program;

        let source = r#"
            #include <header_main.h>

            int factorial(int n) {
                if (n <= 1) return 1;
                return n * factorial(n - 1);
            }

            int main() {
                int result = factorial(10);
                printf("10! = %d\n", result);
                return 0;
            }
        "#;

        let program = compile_cpp_to_program(source)
            .expect("C++ header_main.h + recursion debe compilar");

        assert!(program.functions.len() >= 2);
    }

    // ================================================================
    // Test 3: fastos_*.h individuales
    // ================================================================

    #[test]
    fn test_c_fastos_stdio_individual() {
        use crate::frontend::c::c_to_ir::compile_c_to_program;

        let source = r#"
            #include <fastos_stdio.h>

            int main() {
                printf("Using fastos_stdio.h directly\n");
                return 0;
            }
        "#;

        let program = compile_c_to_program(source)
            .expect("fastos_stdio.h debe compilar");

        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn test_c_fastos_math_individual() {
        use crate::frontend::c::c_to_ir::compile_c_to_program;

        let source = r#"
            #include <fastos_math.h>

            int main() {
                double result = sqrt(144.0);
                double s = sin(3.14);
                return 0;
            }
        "#;

        let program = compile_c_to_program(source)
            .expect("fastos_math.h debe compilar");

        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn test_c_fastos_string_individual() {
        use crate::frontend::c::c_to_ir::compile_c_to_program;

        let source = r#"
            #include <fastos_string.h>

            int main() {
                char buf[64];
                strcpy(buf, "ADead-BIB");
                size_t len = strlen(buf);
                return 0;
            }
        "#;

        let program = compile_c_to_program(source)
            .expect("fastos_string.h debe compilar");

        assert_eq!(program.functions[0].name, "main");
    }

    // ================================================================
    // Test 4: Symbol registries connected
    // ================================================================
    // Verify that c_stdlib → stdlib/c/ and cpp_stdlib → stdlib/cpp/
    // bridges work correctly.
    // ================================================================

    #[test]
    fn test_c_symbol_registry_connected() {
        use crate::frontend::c::c_stdlib::is_known_c_symbol;

        // stdio symbols
        assert!(is_known_c_symbol("printf"), "printf must be a known C symbol");
        assert!(is_known_c_symbol("fprintf"), "fprintf must be known");
        assert!(is_known_c_symbol("scanf"), "scanf must be known");
        assert!(is_known_c_symbol("puts"), "puts must be known");
        assert!(is_known_c_symbol("fopen"), "fopen must be known");

        // stdlib symbols
        assert!(is_known_c_symbol("malloc"), "malloc must be known");
        assert!(is_known_c_symbol("free"), "free must be known");
        assert!(is_known_c_symbol("exit"), "exit must be known");
        assert!(is_known_c_symbol("atoi"), "atoi must be known");

        // string symbols
        assert!(is_known_c_symbol("strlen"), "strlen must be known");
        assert!(is_known_c_symbol("strcmp"), "strcmp must be known");
        assert!(is_known_c_symbol("strcpy"), "strcpy must be known");
        assert!(is_known_c_symbol("memcpy"), "memcpy must be known");

        // math symbols
        assert!(is_known_c_symbol("sin"), "sin must be known");
        assert!(is_known_c_symbol("cos"), "cos must be known");
        assert!(is_known_c_symbol("sqrt"), "sqrt must be known");
        assert!(is_known_c_symbol("pow"), "pow must be known");

        // time symbols
        assert!(is_known_c_symbol("time"), "time must be known");
        assert!(is_known_c_symbol("clock"), "clock must be known");

        // errno
        assert!(is_known_c_symbol("errno"), "errno must be known");

        // Unknown → false
        assert!(!is_known_c_symbol("this_doesnt_exist_xyz123"));
    }

    #[test]
    fn test_cpp_symbol_registry_connected() {
        use crate::frontend::cpp::cpp_stdlib::is_known_cpp_symbol;

        // iostream symbols
        assert!(is_known_cpp_symbol("cout"), "cout must be a known C++ symbol");
        assert!(is_known_cpp_symbol("cin"), "cin must be known");
        assert!(is_known_cpp_symbol("cerr"), "cerr must be known");
        assert!(is_known_cpp_symbol("endl"), "endl must be known");

        // vector symbols
        assert!(is_known_cpp_symbol("vector"), "vector must be known");
        assert!(is_known_cpp_symbol("push_back"), "push_back must be known");

        // string symbols
        assert!(is_known_cpp_symbol("string"), "string must be known");

        // map symbols
        assert!(is_known_cpp_symbol("map"), "map must be known");

        // memory symbols
        assert!(is_known_cpp_symbol("unique_ptr"), "unique_ptr must be known");
        assert!(is_known_cpp_symbol("shared_ptr"), "shared_ptr must be known");
        assert!(is_known_cpp_symbol("make_unique"), "make_unique must be known");

        // algorithm symbols
        assert!(is_known_cpp_symbol("sort"), "sort must be known");
        assert!(is_known_cpp_symbol("find"), "find must be known");

        // exception symbols
        assert!(is_known_cpp_symbol("exception"), "exception must be known");
        assert!(is_known_cpp_symbol("runtime_error"), "runtime_error must be known");

        // Unknown → false
        assert!(!is_known_cpp_symbol("nonexistent_symbol_xyz"));
    }

    // ================================================================
    // Test 5: No linker needed — all headers resolved internally
    // ================================================================
    // ADead-BIB filosofía: NUNCA linker externo.
    // Verify that ALL standard headers are resolved internally,
    // not from filesystem, not from GCC, not from MSVC.
    // ================================================================

    #[test]
    fn test_no_linker_all_c_headers_internal() {
        use crate::frontend::c::c_stdlib::get_header;

        // Every standard header must resolve without filesystem access
        let standard_c_headers = [
            "header_main.h",
            "stdio.h", "stdlib.h", "string.h", "math.h", "time.h",
            "errno.h", "assert.h", "limits.h", "stdint.h", "stdbool.h",
            "stddef.h", "stdarg.h", "ctype.h", "signal.h", "float.h",
            "setjmp.h", "locale.h",
        ];

        for header in &standard_c_headers {
            assert!(
                get_header(header).is_some(),
                "Header <{}> must be resolved internally — NO linker, NO filesystem",
                header
            );
        }

        // fastos_*.h must also resolve
        let fastos_headers = [
            "fastos_stdio.h", "fastos_stdlib.h", "fastos_string.h",
            "fastos_math.h", "fastos_time.h", "fastos_assert.h",
            "fastos_errno.h", "fastos_limits.h", "fastos_types.h",
        ];

        for header in &fastos_headers {
            assert!(
                get_header(header).is_some(),
                "Header <{}> must be resolved internally — fastos v7.0",
                header
            );
        }
    }

    #[test]
    fn test_no_linker_all_cpp_headers_internal() {
        use crate::frontend::cpp::cpp_stdlib::get_cpp_header;

        // Every standard C++ header must resolve
        let standard_cpp_headers = [
            "header_main.h",
            "iostream", "string", "vector", "map", "memory",
            "algorithm", "functional", "utility", "chrono",
            "thread", "mutex", "atomic",
            "cstdio", "cstdlib", "cstring", "cmath",
        ];

        for header in &standard_cpp_headers {
            assert!(
                get_cpp_header(header).is_some(),
                "C++ header <{}> must be resolved internally — NO linker, NO libstdc++",
                header
            );
        }
    }

    // ================================================================
    // Test 6: header_main.h resolver → real content
    // ================================================================

    #[test]
    fn test_resolver_header_main_has_real_content() {
        use crate::preprocessor::HeaderResolver;

        let mut resolver = HeaderResolver::new();
        let content = resolver.resolve("header_main.h").unwrap();

        // Must contain actual declarations, not just comments
        assert!(content.contains("printf"), "header_main.h must declare printf");
        assert!(content.contains("malloc"), "header_main.h must declare malloc");
        assert!(content.contains("strlen"), "header_main.h must declare strlen");
        assert!(content.contains("sin"), "header_main.h must declare sin");
        assert!(content.contains("FILE"), "header_main.h must declare FILE type");
        assert!(content.contains("size_t"), "header_main.h must declare size_t");
        assert!(content.contains("uint8_t"), "header_main.h must declare uint8_t");
    }

    #[test]
    fn test_resolver_stdlib_symbol_detection() {
        let resolver = crate::preprocessor::HeaderResolver::new();

        // Verify symbol detection works through the resolver
        assert!(resolver.is_stdlib_symbol("printf"), "printf → stdlib symbol");
        assert!(resolver.is_stdlib_symbol("malloc"), "malloc → stdlib symbol");
        assert!(resolver.is_stdlib_symbol("strlen"), "strlen → stdlib symbol");
        assert!(!resolver.is_stdlib_symbol("nonexistent_xyz"), "unknown → not stdlib");
    }

    // ================================================================
    // Test 7: C header_main.h → fastos module resolution
    // ================================================================

    #[test]
    fn test_fastos_header_resolution() {
        use crate::frontend::c::c_stdlib::resolve_fastos_header;

        assert_eq!(resolve_fastos_header("stdio.h"), Some("fastos_stdio"));
        assert_eq!(resolve_fastos_header("fastos_stdio.h"), Some("fastos_stdio"));
        assert_eq!(resolve_fastos_header("stdlib.h"), Some("fastos_stdlib"));
        assert_eq!(resolve_fastos_header("string.h"), Some("fastos_string"));
        assert_eq!(resolve_fastos_header("math.h"), Some("fastos_math"));
        assert_eq!(resolve_fastos_header("time.h"), Some("fastos_time"));
        assert_eq!(resolve_fastos_header("header_main.h"), Some("header_main"));
        assert_eq!(resolve_fastos_header("unknown_header.h"), None);
    }

    // ================================================================
    // Test 8: header_main.h C produces actual declarations
    // ================================================================

    #[test]
    fn test_header_main_c_content_complete() {
        use crate::frontend::c::c_stdlib::get_header;

        let header = get_header("header_main.h")
            .expect("header_main.h must exist");

        // Verify it contains complete declarations from each fastos_* section
        assert!(header.contains("fastos_stdio.h"), "Must reference stdio section");
        assert!(header.contains("fastos_stdlib.h"), "Must reference stdlib section");
        assert!(header.contains("fastos_string.h"), "Must reference string section");
        assert!(header.contains("fastos_math.h"), "Must reference math section");
        assert!(header.contains("fastos_time.h"), "Must reference time section");
        assert!(header.contains("fastos_types.h"), "Must reference types section");
        assert!(header.contains("TREE SHAKING"), "Must mention tree shaking");
    }

    #[test]
    fn test_header_main_cpp_content_complete() {
        use crate::frontend::cpp::cpp_stdlib::get_cpp_header;

        let header = get_cpp_header("header_main.h")
            .expect("C++ header_main.h must exist");

        // Verify C declarations available in C++ mode
        assert!(header.contains("printf"), "C++ header_main.h must have printf");
        assert!(header.contains("malloc"), "C++ header_main.h must have malloc");
        assert!(header.contains("strlen"), "C++ header_main.h must have strlen");
        assert!(header.contains("sin"), "C++ header_main.h must have sin");

        // Verify STL recognition comment
        assert!(header.contains("STL"), "Must mention STL type recognition");
        assert!(header.contains("TREE SHAKING"), "Must mention tree shaking");
    }

    // ================================================================
    // Test 9: End-to-end — header_main.h + multiple C99 features
    // ================================================================

    #[test]
    fn test_c_header_main_e2e_full_program() {
        use crate::frontend::c::c_to_ir::compile_c_to_program;

        let source = r#"
            #include <header_main.h>

            typedef struct {
                int x;
                int y;
            } Point;

            int distance_sq(Point a, Point b) {
                int dx = a.x - b.x;
                int dy = a.y - b.y;
                return dx * dx + dy * dy;
            }

            void print_point(Point p) {
                printf("(%d, %d)\n", p.x, p.y);
            }

            int main() {
                printf("=== ADead-BIB v7.0 Integration Test ===\n");

                int arr[5];
                int i = 0;
                while (i < 5) {
                    arr[i] = i * i;
                    i = i + 1;
                }

                double pi = 3.14159265;
                double area = pi * 10.0 * 10.0;
                printf("Circle area: %f\n", area);

                return 0;
            }
        "#;

        let program = compile_c_to_program(source)
            .expect("Full C program with header_main.h must compile — NO linker needed");

        // Verify structure
        assert!(program.functions.len() >= 3, "Expected 3+ functions (distance_sq, print_point, main)");

        // Verify main exists
        let has_main = program.functions.iter().any(|f| f.name == "main");
        assert!(has_main, "Must have main function");
    }
}
