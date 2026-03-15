// ============================================================
// PyDead-BIB Compiler CLI v2.0
// Python Native Compiler — Sin CPython, Sin GIL, Sin Runtime
// 100% Self-Sufficient — Sin linker externo
// 256-bit nativo — YMM/AVX2 — SoA natural
// Hereda ADead-BIB v8.0 — IR probado — codegen probado
// 13/13 fases completas — Real Runtime Output ✓
// ============================================================

use pydead_bib::frontend::python::compile_python_to_ir;
use pydead_bib::backend::isa::Target;
use std::env;
use std::fs;
use std::path::Path;

// ── ANSI Color Module ────────────────────────────────────────
mod colors {
    pub const RESET:   &str = "\x1b[0m";
    pub const BOLD:    &str = "\x1b[1m";
    pub const DIM:     &str = "\x1b[2m";
    pub const RED:     &str = "\x1b[31m";
    pub const GREEN:   &str = "\x1b[32m";
    pub const YELLOW:  &str = "\x1b[33m";
    pub const BLUE:    &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN:    &str = "\x1b[36m";
    pub const WHITE:   &str = "\x1b[37m";
    pub const BG_RED:     &str = "\x1b[41m";
    pub const BG_GREEN:   &str = "\x1b[42m";
    pub const BG_BLUE:    &str = "\x1b[44m";
    pub const BG_MAGENTA: &str = "\x1b[45m";
    pub const BG_CYAN:    &str = "\x1b[46m";
}
use colors::*;

fn enable_ansi() {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::io::AsRawHandle;
        let handle = std::io::stdout().as_raw_handle();
        unsafe {
            extern "system" {
                fn SetConsoleMode(h: *mut std::ffi::c_void, mode: u32) -> i32;
                fn GetConsoleMode(h: *mut std::ffi::c_void, mode: *mut u32) -> i32;
            }
            let mut mode: u32 = 0;
            GetConsoleMode(handle as *mut _, &mut mode);
            SetConsoleMode(handle as *mut _, mode | 0x0004); // ENABLE_VIRTUAL_TERMINAL_PROCESSING
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_ansi();
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        // ============================================================
        // PY — Compile Python source → native binary
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
        // STEP — Step-by-step compilation visualization (13 phases)
        // ============================================================
        "step" => {
            if args.len() < 3 {
                eprintln!("Usage: pyb step <file.py>");
                std::process::exit(1);
            }
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║   PyDead-BIB Step Compiler — Deep Analysis Mode 💀🦈        ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            compile_python_file(
                &args[2],
                &["pyb".to_string(), "step".to_string(), args[2].clone()],
            )?;
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
        // TEST — Run PyDead-BIB test suite
        // ============================================================
        "test" => {
            run_test_suite()?;
        }

        // ============================================================
        // INSTALL — pyb install <package> (pip nativo)
        // ============================================================
        "install" => {
            if args.len() < 3 {
                eprintln!("Usage: pyb install <package>");
                std::process::exit(1);
            }
            cmd_install(&args[2]);
        }

        // ============================================================
        // LIST — pyb list (installed packages)
        // ============================================================
        "list" => {
            cmd_list();
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
            println!("{}{}PyDead-BIB v3.0.0 💀🛸🦈{}", BOLD, CYAN, RESET);
            println!("Python → x86-64 nativo — Sin CPython — Sin GIL — Sin runtime");
            println!("13/13 fases — async/await — generators — SIMD AVX2 — ctypes");
            println!("Hereda ADead-BIB v8.0 — Production Ready — Techne License v1.0");
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
// Compile Python file — Full 13-phase pipeline
// ============================================================
fn compile_python_file(input_file: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input_file)
        .map_err(|e| format!("{}{}❌ Cannot read '{}': {}{}", BOLD, RED, input_file, e, RESET))?;

    let output_file = get_output_filename(input_file, args);
    let target_str = get_target(args);
    let target = Target::from_str(target_str);

    println!();
    println!("{}{}╔══════════════════════════════════════════════════════════════╗{}", BOLD, CYAN, RESET);
    println!("{}{}║   🛸 PyDead-BIB Compiler v3.0 💀🦈                          ║{}", BOLD, CYAN, RESET);
    println!("{}{}║   Python → x86-64 Nativo — Sin CPython — Sin GIL             ║{}", DIM, CYAN, RESET);
    println!("{}{}╚══════════════════════════════════════════════════════════════╝{}", BOLD, CYAN, RESET);
    println!("  {}Source:{}   {}{}{}", DIM, RESET, BOLD, input_file, RESET);
    println!("  {}Output:{}  {}{}{}", DIM, RESET, GREEN, output_file, RESET);
    println!("  {}Target:{}  {}{}{}", DIM, RESET, YELLOW, target_str, RESET);
    println!("  {}Lang:{}    {}Python{}", DIM, RESET, MAGENTA, RESET);
    println!();

    // ── Phase 01: Preprocessor ────────────────────────────────
    println!("{}{}▸ Phase 01:{} {}PREPROCESSOR{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut preprocessor = pydead_bib::frontend::python::py_preprocessor::PyPreprocessor::new();
    let preprocessed = preprocessor.process(&source);
    let line_count = preprocessed.lines().count();
    println!("  {}encoding:{} UTF-8", DIM, RESET);
    println!("  {}source:{}  {}{}{} lines", DIM, RESET, BOLD, line_count, RESET);

    // ── Phase 02: Import Eliminator ───────────────────────────
    println!("{}{}▸ Phase 02:{} {}IMPORT ELIMINATOR{}", BOLD, BLUE, RESET, CYAN, RESET);
    let import_resolver = pydead_bib::frontend::python::py_import_resolver::PyImportResolver::new();
    let imports = import_resolver.resolve(&preprocessed);
    for imp in &imports {
        let resolved = import_resolver.resolve_module(imp);
        println!("  {}→{} {} {}({:?}){}", GREEN, RESET, imp, DIM, resolved.compile_action, RESET);
    }
    if imports.is_empty() {
        println!("  {}no imports{}", DIM, RESET);
    }

    // ── Phase 03: Lexer ───────────────────────────────────────
    println!("{}{}▸ Phase 03:{} {}LEXER{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut lexer = pydead_bib::frontend::python::py_lexer::PyLexer::new(&preprocessed);
    let tokens = lexer.tokenize();
    let indent_count = tokens.iter().filter(|t| matches!(t, pydead_bib::frontend::python::py_lexer::PyToken::Indent)).count();
    let dedent_count = tokens.iter().filter(|t| matches!(t, pydead_bib::frontend::python::py_lexer::PyToken::Dedent)).count();
    println!("  {}tokens:{}  {}{}{}", DIM, RESET, BOLD, tokens.len(), RESET);
    println!("  {}indent:{}  {}/{} pares", DIM, RESET, indent_count, dedent_count);

    // ── Phase 04: Parser ──────────────────────────────────────
    println!("{}{}▸ Phase 04:{} {}PARSER{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut parser = pydead_bib::frontend::python::py_parser::PyParser::new(tokens);
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(e) => {
            println!("  {}{}✗ PARSE ERROR:{} {}", BOLD, RED, RESET, e);
            println!("  {}{}  → Check syntax at the indicated line{}", DIM, RED, RESET);
            println!("  {}{}  → Common: missing colon, unmatched parens, bad indent{}", DIM, RED, RESET);
            return Err(e.into());
        }
    };
    println!("  {}AST:{}     {}{}{} top-level nodes", DIM, RESET, BOLD, ast.body.len(), RESET);
    for stmt in &ast.body {
        match stmt {
            pydead_bib::frontend::python::py_ast::PyStmt::FunctionDef { name, params, .. } => {
                println!("  {}├─{} {}fn{} {}{}{}({}{}{})", DIM, RESET, MAGENTA, RESET, BOLD, name, RESET, DIM, params.len(), RESET);
            }
            pydead_bib::frontend::python::py_ast::PyStmt::ClassDef { name, .. } => {
                println!("  {}├─{} {}class{} {}{}{}", DIM, RESET, YELLOW, RESET, BOLD, name, RESET);
            }
            pydead_bib::frontend::python::py_ast::PyStmt::Import { names } => {
                for alias in names {
                    println!("  {}├─{} {}import{} {}", DIM, RESET, BLUE, RESET, alias.name);
                }
            }
            _ => {}
        }
    }

    // ── Phase 05: Type Inferencer ─────────────────────────────
    println!("{}{}▸ Phase 05:{} {}TYPE INFERENCER{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut inferencer = pydead_bib::frontend::python::py_types::PyTypeInferencer::new();
    let typed_ast = inferencer.infer(&ast);
    println!("  {}{}✓{} inference complete", GREEN, BOLD, RESET);

    // ── Phase 06: IR Generation ───────────────────────────────
    println!("{}{}▸ Phase 06:{} {}IR GEN (ADeadOp SSA){}", BOLD, BLUE, RESET, CYAN, RESET);
    let ir = match compile_python_to_ir(&typed_ast) {
        Ok(i) => i,
        Err(e) => {
            println!("  {}{}✗ IR ERROR:{} {}", BOLD, RED, RESET, e);
            println!("  {}{}  → Unsupported construct or internal error{}", DIM, RED, RESET);
            return Err(e.into());
        }
    };
    println!("  {}funcs:{}   {}{}{}", DIM, RESET, BOLD, ir.functions.len(), RESET);
    println!("  {}stmts:{}   {}", DIM, RESET, ir.statement_count());
    println!("  {}strings:{} {} in .data", DIM, RESET, ir.string_data.len());
    println!("  {}GIL:{}     {}eliminado{} — ownership estático", DIM, RESET, GREEN, RESET);

    // ── Phase 06b: Optimizer (v3.0) ─────────────────────────────
    println!("{}{}▸ Phase 06b:{} {}OPTIMIZER (v3.0){}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut total_folded = 0usize;
    let mut total_eliminated = 0usize;
    let mut ir = ir; // make mutable
    for func in ir.functions.iter_mut() {
        let (f, e) = pydead_bib::middle::ir::optimize_function(func);
        total_folded += f;
        total_eliminated += e;
    }
    println!("  {}const fold:{} {}{}{} expressions", DIM, RESET, BOLD, total_folded, RESET);
    println!("  {}dead code:{}  {}{}{} removed", DIM, RESET, BOLD, total_eliminated, RESET);

    // ── Phase 07: UB Detector ─────────────────────────────────
    println!("{}{}▸ Phase 07:{} {}UB DETECTOR{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut ub_detector = pydead_bib::middle::ub_detector::PyUBDetector::new()
        .with_file(input_file.to_string());
    let reports = ub_detector.analyze(&ir);
    let ub_errors = reports.iter().filter(|r| matches!(r.severity, pydead_bib::middle::ub_detector::UBSeverity::Error)).count();
    let ub_warnings = reports.iter().filter(|r| matches!(r.severity, pydead_bib::middle::ub_detector::UBSeverity::Warning)).count();
    let ub_infos = reports.iter().filter(|r| matches!(r.severity, pydead_bib::middle::ub_detector::UBSeverity::Info)).count();
    if reports.is_empty() {
        println!("  {}{}✓ CLEAN{} — 0 errors, 0 warnings, 0 infos", GREEN, BOLD, RESET);
    } else {
        println!("  {}scan:{} {}{} errors{}, {}{} warnings{}, {}{} infos{}",
            DIM, RESET,
            if ub_errors > 0 { RED } else { GREEN }, ub_errors, RESET,
            if ub_warnings > 0 { YELLOW } else { GREEN }, ub_warnings, RESET,
            DIM, ub_infos, RESET);
        for report in reports.iter() {
            let (icon, color) = match report.severity {
                pydead_bib::middle::ub_detector::UBSeverity::Error => ("✗", RED),
                pydead_bib::middle::ub_detector::UBSeverity::Warning => ("⚠", YELLOW),
                pydead_bib::middle::ub_detector::UBSeverity::Info => ("ℹ", BLUE),
            };
            println!("  {}{}{}{} {:?}: {}{}", color, BOLD, icon, RESET, report.kind, report.message, RESET);
            if let Some(suggestion) = &report.suggestion {
                println!("    {}{}fix:{} {}", DIM, GREEN, RESET, suggestion);
            }
        }
        if ub_errors > 0 {
            println!();
            println!("  {}{}╔═══════════════════════════════════════════════════╗{}", BG_RED, WHITE, RESET);
            println!("  {}{}║  COMPILATION BLOCKED — {} UB error(s) detected    ║{}", BG_RED, WHITE, ub_errors, RESET);
            println!("  {}{}║  Fix all errors above to compile successfully     ║{}", BG_RED, WHITE, RESET);
            println!("  {}{}╚═══════════════════════════════════════════════════╝{}", BG_RED, WHITE, RESET);
            println!();
            println!("  {}{}Why?{} PyDead-BIB detects undefined behavior at compile time.", BOLD, RED, RESET);
            println!("  {}CPython would crash at runtime. We catch it before.{}", DIM, RESET);
            return Err(format!("{} UB error(s) — compilation aborted", ub_errors).into());
        }
    }

    // ── Phase 08: Optimizer ───────────────────────────────────
    println!("{}{}▸ Phase 08:{} {}OPTIMIZER{}", BOLD, BLUE, RESET, CYAN, RESET);
    let optimized = pydead_bib::backend::optimizer::optimize(&ir);
    println!("  {}folded:{}  {} constants", DIM, RESET, optimized.stats.constants_folded);
    println!("  {}dead:{}    {} removed", DIM, RESET, optimized.stats.dead_code_removed);
    println!("  {}SIMD:{}    {} vectorized", DIM, RESET, optimized.stats.simd_vectorized);

    // ── Phase 09: Register Allocator ──────────────────────────
    println!("{}{}▸ Phase 09:{} {}REGISTER ALLOCATOR{}", BOLD, BLUE, RESET, CYAN, RESET);
    let allocated = pydead_bib::backend::reg_alloc::allocate(&optimized);
    println!("  {}vars:{}    {} → {}{}{} regs, {} spills",
        DIM, RESET, allocated.stats.total_vars, GREEN, allocated.stats.registers_used, RESET, allocated.stats.spills);
    for func in &allocated.functions {
        println!("  {}├─{} {} {}(stack: {}B, regs: {}){}", DIM, RESET, func.name, DIM, func.stack_size, func.reg_map.len(), RESET);
    }

    // ── Phase 10: ISA Compiler ────────────────────────────────
    println!("{}{}▸ Phase 10:{} {}ISA COMPILER (x86-64){}", BOLD, BLUE, RESET, CYAN, RESET);
    let compiled = pydead_bib::backend::isa::compile(&allocated, target);
    println!("  {}.text:{}   {}{}{} bytes", DIM, RESET, BOLD, compiled.stats.total_bytes, RESET);
    println!("  {}funcs:{}   {}", DIM, RESET, compiled.stats.functions_compiled);
    println!("  {}instrs:{}  {}", DIM, RESET, compiled.stats.instructions_emitted);

    // ── Phase 11: BG Stamp ────────────────────────────────────
    println!("{}{}▸ Phase 11:{} {}BINARY GUARDIAN{}", BOLD, BLUE, RESET, CYAN, RESET);
    let stamped = pydead_bib::backend::bg::stamp(&compiled);
    println!("  {}magic:{}   {}0x{:08X}{}", DIM, RESET, MAGENTA, stamped.stamp.magic, RESET);
    println!("  {}ver:{}     0x{:04X}", DIM, RESET, stamped.stamp.version);
    println!("  {}chksum:{}  {}0x{:08X}{}", DIM, RESET, YELLOW, stamped.stamp.checksum, RESET);

    // ── Phase 12: Output (PE/ELF/Po) ─────────────────────────
    println!("{}{}▸ Phase 12:{} {}OUTPUT{}", BOLD, BLUE, RESET, CYAN, RESET);
    let binary = pydead_bib::backend::output::emit(&stamped);
    let stats = pydead_bib::backend::output::binary_stats(&binary, &stamped);
    println!("  {}format:{}  {}", DIM, RESET, stats.target);
    println!("  {}.text:{}   {} bytes", DIM, RESET, stats.text_bytes);
    println!("  {}.data:{}   {} bytes", DIM, RESET, stats.data_bytes);
    println!("  {}total:{}   {}{}{} bytes", DIM, RESET, BOLD, stats.total_bytes, RESET);

    // ── Phase 13: Write binary ────────────────────────────────
    println!("{}{}▸ Phase 13:{} {}WRITE{}", BOLD, BLUE, RESET, CYAN, RESET);
    fs::write(&output_file, &binary)?;
    println!("  {}→{} {}{}{} ({} bytes)", GREEN, RESET, BOLD, output_file, RESET, binary.len());
    println!();

    // ── Success summary ───────────────────────────────────────
    let size_kb = binary.len() as f64 / 1024.0;
    println!("{}{}╔══════════════════════════════════════════════════════════════╗{}", BOLD, GREEN, RESET);
    println!("{}{}║   ✅ Compilation complete — 13/13 phases                     ║{}", BOLD, GREEN, RESET);
    println!("{}{}╚══════════════════════════════════════════════════════════════╝{}", BOLD, GREEN, RESET);
    println!("  {}Output:{}    {}{}{} ({:.1}KB)", DIM, RESET, BOLD, output_file, RESET, size_kb);
    println!("  {}Target:{}    {:?}", DIM, RESET, target);
    println!("  {}Functions:{} {}", DIM, RESET, ir.functions.len());
    println!("  {}UB Found:{}  {}{}{}", DIM, RESET,
        if ub_errors + ub_warnings == 0 { GREEN } else { YELLOW },
        if ub_errors + ub_warnings == 0 { "0 (clean)" } else { "see above" }, RESET);
    println!();
    println!("  {}Sin CPython — Sin GIL — Sin runtime 💀🦈{}", DIM, RESET);
    println!();

    // ── Architecture diagram ──────────────────────────────────
    println!("  {}┌──────────────────────────────────────────┐{}", DIM, RESET);
    println!("  {}│{} .py {}→{} Lexer {}→{} Parser {}→{} IR {}→{} ISA {}→{} .exe {}│{}", DIM, RESET,
        BLUE, RESET, CYAN, RESET, GREEN, RESET, YELLOW, RESET, MAGENTA, RESET, DIM, RESET);
    println!("  {}│  Python   {:?}   {:.1}KB   nativo  │{}", DIM, target, size_kb, RESET);
    println!("  {}└──────────────────────────────────────────┘{}", DIM, RESET);

    Ok(())
}

// ============================================================
// Test suite — pyb test
// ============================================================
fn run_test_suite() -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}{}╔══════════════════════════════════════════════════════════════╗{}", BOLD, CYAN, RESET);
    println!("{}{}║   🧪 PyDead-BIB Test Suite v2.0 💀🦈                        ║{}", BOLD, CYAN, RESET);
    println!("{}{}╚══════════════════════════════════════════════════════════════╝{}", BOLD, CYAN, RESET);
    println!();

    let tests = [
        ("tests/test_hello.py", "Hello World"),
        ("tests/test_types.py", "Types"),
        ("tests/test_functions.py", "Functions"),
        ("tests/test_classes.py", "Classes"),
        ("tests/test_builtins.py", "Builtins"),
        ("tests/test_hello_real.py", "Real Print"),
        ("tests/test_print_types.py", "Print Types"),
        ("tests/test_float.py", "Float Print"),
        ("tests/test_arithmetic.py", "Arithmetic"),
        ("tests/test_if.py", "If/Else"),
        ("tests/test_loops.py", "Loops"),
        ("tests/test_fstring.py", "F-Strings"),
        ("tests/test_tuple.py", "Tuple Unpack"),
        ("tests/test_import.py", "Import Math"),
        ("tests/test_builtins2.py", "Builtins v2"),
        ("tests/test_list.py", "List Heap"),
        ("tests/test_dict.py", "Dict Heap"),
        ("tests/test_class_v2.py", "Class v2"),
        ("tests/test_os.py", "OS Module"),
        ("tests/test_sys.py", "Sys Module"),
        ("tests/test_random.py", "Random"),
        ("tests/test_strings.py", "Strings"),
        ("tests/test_file.py", "File I/O"),
        ("tests/test_try.py", "Try/Except"),
        ("tests/test_json.py", "JSON"),
        // v2.0 tests
        ("tests/test_exceptions.py", "Exceptions v2"),
        ("tests/test_generators.py", "Generators"),
        ("tests/test_decorators.py", "Decorators"),
        ("tests/test_numpy.py", "Numpy Native"),
        ("tests/test_inheritance.py", "Inheritance"),
        ("tests/test_comprehensions.py", "Comprehensions"),
        ("tests/test_string_format.py", "String Format"),
        ("tests/test_modules.py", "Modules"),
        ("tests/test_typing.py", "Typing"),
        ("tests/test_with.py", "With/Context"),
        ("tests/test_dataclass.py", "Dataclass"),
        // v3.0 tests
        ("tests/test_async.py", "Async/Await"),
        ("tests/test_generators_v2.py", "Generators v2"),
        ("tests/test_property.py", "Property"),
        ("tests/test_lru_cache.py", "LRU Cache"),
        ("tests/test_numpy_v2.py", "Numpy v2"),
        ("tests/test_ctypes.py", "CTypes"),
        ("tests/test_simd.py", "SIMD AVX2"),
        ("tests/test_optimizations.py", "Optimizations"),
        // pyb_ai — Metal-Dead + IA-Personal rewritten for PyDead-BIB
        ("pyb_ai/tokenizer.py", "AI Tokenizer"),
        ("pyb_ai/memory.py", "AI Memory"),
        ("pyb_ai/context.py", "AI Context"),
        ("pyb_ai/model.py", "AI Model"),
        ("pyb_ai/intelligence.py", "AI Intelligence"),
        ("pyb_ai/metal_dead.py", "Metal-Dead"),
        ("pyb_ai/ollama_bridge.py", "Ollama Bridge"),
        ("pyb_ai/ia_personal.py", "IA-Personal"),
        ("pyb_ai/main.py", "AI Suite"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (file, name) in &tests {
        print!("  {} ", name);
        for _ in name.len()..20 {
            print!(" ");
        }

        if !Path::new(file).exists() {
            println!("{}[SKIP]{} file not found", YELLOW, RESET);
            continue;
        }

        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => {
                println!("{}{}[FAIL]{} cannot read", BOLD, RED, RESET);
                failed += 1;
                continue;
            }
        };

        // Run frontend pipeline silently
        let mut preprocessor = pydead_bib::frontend::python::py_preprocessor::PyPreprocessor::new();
        let preprocessed = preprocessor.process(&source);
        let mut lexer = pydead_bib::frontend::python::py_lexer::PyLexer::new(&preprocessed);
        let tokens = lexer.tokenize();
        let mut parser = pydead_bib::frontend::python::py_parser::PyParser::new(tokens);
        let ast = match parser.parse() {
            Ok(a) => a,
            Err(e) => {
                println!("{}{}[FAIL]{} parse: {}", BOLD, RED, RESET, e);
                failed += 1;
                continue;
            }
        };
        let mut inferencer = pydead_bib::frontend::python::py_types::PyTypeInferencer::new();
        let typed_ast = inferencer.infer(&ast);
        let ir = match compile_python_to_ir(&typed_ast) {
            Ok(i) => i,
            Err(e) => {
                println!("{}{}[FAIL]{} IR: {}", BOLD, RED, RESET, e);
                failed += 1;
                continue;
            }
        };

        // Backend pipeline
        let optimized = pydead_bib::backend::optimizer::optimize(&ir);
        let allocated = pydead_bib::backend::reg_alloc::allocate(&optimized);
        let compiled = pydead_bib::backend::isa::compile(&allocated, Target::Windows);
        let stamped = pydead_bib::backend::bg::stamp(&compiled);
        let binary = pydead_bib::backend::output::emit(&stamped);

        // Write output
        let out_name = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("test");
        let out_file = format!("{}.exe", out_name);
        fs::write(&out_file, &binary)?;

        let size_kb = binary.len() as f64 / 1024.0;
        println!("{}{}[PASS]{} {}({:.1}KB, {} funcs){}", BOLD, GREEN, RESET, DIM, size_kb, ir.functions.len(), RESET);
        passed += 1;

        // Cleanup
        let _ = fs::remove_file(&out_file);
    }

    println!();
    let total = passed + failed;
    if failed == 0 {
        println!("{}{}╔══════════════════════════════════════════════════════════════╗{}", BOLD, GREEN, RESET);
        println!("{}{}║   ✅ TOTAL: {}/{} PASS                                      ║{}", BOLD, GREEN, passed, total, RESET);
        println!("{}{}╚══════════════════════════════════════════════════════════════╝{}", BOLD, GREEN, RESET);
        println!("  {}Binary Is Binary 💀🦈🇵🇪{}", DIM, RESET);
    } else {
        println!("{}{}╔══════════════════════════════════════════════════════════════╗{}", BOLD, RED, RESET);
        println!("{}{}║   ❌ TOTAL: {}/{} PASS — {} FAILED                           ║{}", BOLD, RED, passed, total, failed, RESET);
        println!("{}{}╚══════════════════════════════════════════════════════════════╝{}", BOLD, RED, RESET);
    }

    Ok(())
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

// ============================================================
// pyb install — native package manager
// ============================================================
fn cmd_install(package: &str) {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   pyb install — Native Package Manager 💀🦈                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // stdlib modules — already included
    let stdlib = ["math", "os", "sys", "json", "random", "re", "time"];
    if stdlib.contains(&package) {
        println!("  📦 '{}' is part of PyDead-BIB stdlib — already included!", package);
        println!("  Just use: import {}", package);
        println!();
        println!("  stdlib modules: {}", stdlib.join(", "));
        return;
    }

    // Check native registry
    let pyb_dir = get_pyb_dir();
    let lib_dir = format!("{}/lib/{}", pyb_dir, package);

    if Path::new(&lib_dir).exists() {
        println!("  ✅ '{}' already installed at {}", package, lib_dir);
        return;
    }

    // Known packages (future AOT compile targets)
    match package {
        "requests" => {
            println!("  � Package 'requests' — HTTP client");
            println!("  ⚠️  Native HTTP not yet available in v1.5");
            println!("  Planned: pyb install requests → AOT compiled HTTP via WinHTTP/libcurl");
        }
        "numpy-native" | "numpy" => {
            println!("  🔢 Package 'numpy-native' — Native SIMD arrays");
            println!("  ⚠️  Coming in v1.6 with AVX2/YMM native arrays");
        }
        "pygame-native" | "pygame" => {
            println!("  🎮 Package 'pygame-native' — Native game framework");
            println!("  ⚠️  Coming in v2.0 with DirectX/Vulkan backend");
        }
        _ => {
            println!("  ❌ Package '{}' not found in PyDead-BIB registry", package);
            println!();
            println!("  Available:");
            println!("    stdlib (included): math, os, sys, json, random, re, time");
            println!("    planned:          requests, numpy-native, pygame-native");
            println!();
            println!("  Note: PyDead-BIB uses native packages (.pyblib), not CPython wheels");
        }
    }
    println!();
    println!("  Registry: {}/registry.toml", pyb_dir);
    println!("  Lib dir:  {}/lib/", pyb_dir);
}

fn cmd_list() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   pyb list — Installed Packages 💀🦈                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  stdlib (built-in):");
    println!("    math     ✅  sqrt, floor, ceil, sin, cos, log, pi, e");
    println!("    os       ✅  getcwd, path.exists/join, getpid, listdir, makedirs, remove");
    println!("    sys      ✅  argv, platform, version, maxsize, exit");
    println!("    json     ✅  loads, dumps");
    println!("    random   ✅  randint, random, choice, seed");
    println!();

    // Check for installed packages
    let pyb_dir = get_pyb_dir();
    let lib_dir = format!("{}/lib", pyb_dir);
    if Path::new(&lib_dir).exists() {
        if let Ok(entries) = fs::read_dir(&lib_dir) {
            let pkgs: Vec<_> = entries.filter_map(|e| {
                e.ok().and_then(|e| e.file_name().into_string().ok())
            }).collect();
            if !pkgs.is_empty() {
                println!("  installed:");
                for pkg in &pkgs {
                    println!("    {}  📦", pkg);
                }
            } else {
                println!("  installed: (none)");
            }
        }
    } else {
        println!("  installed: (none)");
    }
    println!();
    println!("  Use 'pyb install <package>' to install packages");
}

fn get_pyb_dir() -> String {
    if cfg!(target_os = "windows") {
        let home = env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\default".to_string());
        format!("{}/.pyb", home)
    } else {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/.pyb", home)
    }
}

fn print_usage(program: &str) {
    println!();
    println!("{}{}╔══════════════════════════════════════════════════════════════╗{}", BOLD, CYAN, RESET);
    println!("{}{}║   🛸 PyDead-BIB v2.0 — Python Native Compiler 💀🦈          ║{}", BOLD, CYAN, RESET);
    println!("{}{}║   Sin CPython — Sin GIL — Sin Runtime                        ║{}", DIM, CYAN, RESET);
    println!("{}{}╚══════════════════════════════════════════════════════════════╝{}", BOLD, CYAN, RESET);
    println!();
    println!("{}Usage:{}", BOLD, RESET);
    println!("  {}{}py{} <file.py> [-o output]          {}Compile Python → native{}", BOLD, GREEN, RESET, DIM, RESET);
    println!("  {}{}run{} <file.py>                     {}Compile and run{}", BOLD, GREEN, RESET, DIM, RESET);
    println!("  {}{}test{}                              {}Run test suite{}", BOLD, YELLOW, RESET, DIM, RESET);
    println!("  {}{}install{} <package>                 {}Install native package{}", BOLD, BLUE, RESET, DIM, RESET);
    println!("  {}{}list{}                              {}List installed packages{}", BOLD, BLUE, RESET, DIM, RESET);
    println!("  {}{}build{}                             {}Build pyb.toml project{}", BOLD, MAGENTA, RESET, DIM, RESET);
    println!("  {}{}create{} <name>                     {}Create new project{}", BOLD, MAGENTA, RESET, DIM, RESET);
    println!("  {}--version{}                         {}Show version{}", DIM, RESET, DIM, RESET);
    println!();
    println!("  {}Eddi Andreé Salazar Matos — Lima, Perú 🇵🇪{}", DIM, RESET);
}
