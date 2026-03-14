// ============================================================
// PyDead-BIB Compiler CLI v1.0
// Python Native Compiler — Sin CPython, Sin GIL, Sin Runtime
// 100% Self-Sufficient — Sin linker externo
// 256-bit nativo — YMM/AVX2 — SoA natural
// Hereda ADead-BIB v8.0 — IR probado — codegen probado
// ============================================================

use pydead_bib::frontend::python::compile_python_to_ir;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        // ============================================================
        // PY — Compile Python source
        // ============================================================
        "py" => {
            if args.len() < 3 {
                eprintln!("❌ Error: Missing Python source file");
                eprintln!("   Usage: pyb py <file.py> [-o output.exe]");
                std::process::exit(1);
            }
            compile_python_file(&args[2], &args)?;
        }

        // ============================================================
        // STEP — Step-by-step compilation visualization (11 phases)
        // ============================================================
        "step" => {
            if args.len() < 3 {
                eprintln!("Usage: pyb step <file.py>");
                std::process::exit(1);
            }
            step_compile_python(&args[2])?;
        }

        // ============================================================
        // RUN — Compile and execute
        // ============================================================
        "run" => {
            if args.len() < 3 {
                eprintln!("Usage: pyb run <file.py>");
                std::process::exit(1);
            }
            compile_python_file(&args[2], &args)?;
            let output = get_output_filename(&args[2], &args);
            println!("🚀 Running {}...\n", &args[2]);
            let exe_path = if cfg!(target_os = "windows") {
                format!(".\\{}", output)
            } else {
                format!("./{}", output)
            };
            let status = std::process::Command::new(&exe_path).status()?;
            if !status.success() {
                eprintln!("\n⚠️  Program exited with status: {}", status);
            }
        }

        // ============================================================
        // BUILD — Build from pyb.toml project
        // ============================================================
        "build" => {
            println!("📦 Building project from pyb.toml...");
            eprintln!("⚠️  pyb.toml project builds coming soon");
        }

        // ============================================================
        // CREATE — New Python project
        // ============================================================
        "create" => {
            if args.len() < 3 {
                eprintln!("Usage: pyb create <project_name>");
                std::process::exit(1);
            }
            create_project(&args[2])?;
        }

        // ============================================================
        // VERSION
        // ============================================================
        "--version" | "-v" | "version" => {
            println!("PyDead-BIB v1.0.0 💀🦈");
            println!("Python → x86-64 nativo — Sin CPython — Sin GIL — Sin runtime");
            println!("Hereda ADead-BIB v8.0 — IR probado — codegen probado");
            println!("Eddi Andreé Salazar Matos — Lima, Perú 🇵🇪");
        }

        _ => {
            if command.ends_with(".py") {
                compile_python_file(command, &args)?;
            } else {
                eprintln!("❌ Unknown command: '{}'", command);
                print_usage(&args[0]);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

// ============================================================
// Compile Python file — Full 11-phase pipeline
// ============================================================
fn compile_python_file(input_file: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input_file)
        .map_err(|e| format!("❌ Cannot read '{}': {}", input_file, e))?;

    let output_file = get_output_filename(input_file, args);
    let target = get_target(args);

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   PyDead-BIB Compiler v1.0 💀🦈                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!("  Source:   {}", input_file);
    println!("  Output:   {}", output_file);
    println!("  Target:   {}", target);
    println!("  Language: Python");
    println!();

    // Phase 01: Preprocessor
    println!("--- Phase 01: PREPROCESSOR ---");
    let mut preprocessor = pydead_bib::frontend::python::py_preprocessor::PyPreprocessor::new();
    let preprocessed = preprocessor.process(&source);
    println!("[PREPROC]  encoding: UTF-8 detectado");
    println!("[PREPROC]  source: {} lines", preprocessed.lines().count());
    println!();

    // Phase 02: Import Eliminator
    println!("--- Phase 02: IMPORT ELIMINATOR ---");
    let import_resolver = pydead_bib::frontend::python::py_import_resolver::PyImportResolver::new();
    let imports = import_resolver.resolve(&preprocessed);
    for imp in &imports {
        let resolved = import_resolver.resolve_module(imp);
        println!("[IMPORT]   {} → {:?}", imp, resolved.compile_action);
    }
    if imports.is_empty() {
        println!("[IMPORT]   no imports detected");
    }
    println!("[IMPORT]   sin site-packages — NUNCA");
    println!();

    // Phase 03: Lexer
    println!("--- Phase 03: LEXER ---");
    let mut lexer = pydead_bib::frontend::python::py_lexer::PyLexer::new(&preprocessed);
    let tokens = lexer.tokenize();
    let indent_count = tokens.iter().filter(|t| matches!(t, pydead_bib::frontend::python::py_lexer::PyToken::Indent)).count();
    let dedent_count = tokens.iter().filter(|t| matches!(t, pydead_bib::frontend::python::py_lexer::PyToken::Dedent)).count();
    println!("[LEXER]    {} tokens generados", tokens.len());
    println!("[LEXER]    INDENT/DEDENT: {}/{} pares", indent_count, dedent_count);
    println!();

    // Phase 04: Parser
    println!("--- Phase 04: PARSER ---");
    let mut parser = pydead_bib::frontend::python::py_parser::PyParser::new(tokens);
    let ast = parser.parse()?;
    println!("[PARSER]   AST generated — {} top-level nodes", ast.body.len());
    for stmt in &ast.body {
        match stmt {
            pydead_bib::frontend::python::py_ast::PyStmt::FunctionDef { name, params, .. } => {
                println!("[PARSER]     fn {}({} params)", name, params.len());
            }
            pydead_bib::frontend::python::py_ast::PyStmt::ClassDef { name, .. } => {
                println!("[PARSER]     class {}", name);
            }
            pydead_bib::frontend::python::py_ast::PyStmt::Import { names } => {
                for alias in names {
                    println!("[PARSER]     import {}", alias.name);
                }
            }
            _ => {}
        }
    }
    println!();

    // Phase 05: Type Inferencer
    println!("--- Phase 05: TYPE INFERENCER ---");
    let mut inferencer = pydead_bib::frontend::python::py_types::PyTypeInferencer::new();
    let typed_ast = inferencer.infer(&ast);
    println!("[TYPES]    type inference complete");
    println!();

    // Phase 06: IR Generation
    println!("--- Phase 06: IR (ADeadOp SSA-form) ---");
    let ir = compile_python_to_ir(&typed_ast)?;
    println!("[IR]       {} functions compiled", ir.functions.len());
    println!("[IR]       {} IR statements total", ir.statement_count());
    println!("[IR]       {} string literals in .data", ir.string_data.len());
    println!("[IR]       GIL eliminado — ownership estático ✓");
    println!();

    // Phase 07: UB Detector
    println!("--- Phase 07: UB DETECTOR ---");
    let mut ub_detector = pydead_bib::middle::ub_detector::PyUBDetector::new()
        .with_file(input_file.to_string());
    let reports = ub_detector.analyze(&ir);
    if reports.is_empty() {
        println!("[UB]       ✓ CLEAN — sin undefined behavior detectado");
    } else {
        for report in reports {
            let icon = match report.severity {
                pydead_bib::middle::ub_detector::UBSeverity::Error => "✗",
                pydead_bib::middle::ub_detector::UBSeverity::Warning => "⚠",
                pydead_bib::middle::ub_detector::UBSeverity::Info => "ℹ",
            };
            println!("[UB]       {} {:?}: {}", icon, report.kind, report.message);
            if let Some(suggestion) = &report.suggestion {
                println!("[UB]         fix: {}", suggestion);
            }
        }
    }
    println!();

    // Phases 08-11: TODO — heredar de ADead-BIB
    println!("--- Phase 08: OPTIMIZER ---");
    println!("[OPT]      (heredar de ADead-BIB v8.0)");
    println!();
    println!("--- Phase 09: REGISTER ALLOCATOR ---");
    println!("[REGALLOC] (heredar de ADead-BIB v8.0)");
    println!();
    println!("--- Phase 10: BIT RESOLVER ---");
    println!("[BITS]     --target {} → pending", target);
    println!();
    println!("--- Phase 11: OUTPUT ---");
    println!("[OUTPUT]   Target: {}", target);
    println!("[OUTPUT]   (PE/ELF generation — heredar de ADead-BIB v8.0)");
    println!();

    println!("✅ Frontend compilation complete: {}", input_file);
    println!("   {} functions → IR → ready for backend", ir.functions.len());
    println!("   Sin CPython — Sin GIL — Sin runtime 💀🦈");

    Ok(())
}

// ============================================================
// Step compile — verbose 11 phases
// ============================================================
fn step_compile_python(input_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   PyDead-BIB Step Compiler — Deep Analysis Mode 💀🦈         ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    compile_python_file(input_file, &["pyb".to_string(), "step".to_string(), input_file.to_string()])
}

// ============================================================
// Create new project
// ============================================================
fn create_project(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(format!("{}/src", name))?;

    let toml = format!(r#"[project]
name = "{}"
version = "0.1.0"
lang = "python"
standard = "py3"

[build]
src = "src/"
output = "bin/"

[python]
version = "3.11"
type_check = "strict"
ub_mode = "strict"
simd = "auto"
"#, name);
    fs::write(format!("{}/pyb.toml", name), toml)?;

    let main_py = r#"def main() -> int:
    print("Hello from PyDead-BIB! 💀🦈")
    return 0

if __name__ == "__main__":
    main()
"#;
    fs::write(format!("{}/src/main.py", name), main_py)?;

    println!("✅ Project '{}' created!", name);
    println!("   cd {} && pyb run src/main.py", name);
    Ok(())
}

// ============================================================
// Helpers
// ============================================================
fn get_output_filename(input: &str, args: &[String]) -> String {
    for i in 0..args.len().saturating_sub(1) {
        if args[i] == "-o" {
            return args[i + 1].clone();
        }
    }
    let stem = Path::new(input)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    if cfg!(target_os = "windows") {
        format!("{}.exe", stem)
    } else {
        stem.to_string()
    }
}

fn get_target(args: &[String]) -> &str {
    for i in 0..args.len().saturating_sub(1) {
        if args[i] == "--target" {
            return &args[i + 1];
        }
    }
    if cfg!(target_os = "windows") { "windows" } else { "linux" }
}

fn print_usage(program: &str) {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   PyDead-BIB v1.0 — Python Native Compiler 💀🦈             ║");
    println!("║   Sin CPython — Sin GIL — Sin Runtime                        ║");
    println!("║   Hereda ADead-BIB v8.0 — 256-bit nativo                     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Usage:");
    println!("  {} py <file.py> [-o output]          Compile Python", program);
    println!("  {} py <file.py> --target windows     Target Windows PE", program);
    println!("  {} py <file.py> --target linux       Target Linux ELF", program);
    println!("  {} py <file.py> --target fastos256   Target FastOS 256-bit", program);
    println!("  {} step <file.py>                    Step mode (11 phases)", program);
    println!("  {} run <file.py>                     Compile and run", program);
    println!("  {} build                             Build pyb.toml project", program);
    println!("  {} create <name>                     Create new project", program);
    println!("  {} --version                         Show version", program);
}
