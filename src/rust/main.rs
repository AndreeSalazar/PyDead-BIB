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
            // FASE 2: JIT execution via VirtualAlloc — no .exe written to disk
            jit_execute(&args[2])?;
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

    let pipeline_start = std::time::Instant::now();

    // ── Phase 01: Preprocessor ────────────────────────────────
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 01:{} {}PREPROCESSOR{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut preprocessor = pydead_bib::frontend::python::py_preprocessor::PyPreprocessor::new();
    let preprocessed = preprocessor.process(&source);
    let line_count = preprocessed.lines().count();
    let t01 = t0.elapsed();
    println!("  {}encoding:{} UTF-8", DIM, RESET);
    println!("  {}source:{}  {}{}{} lines  {}[{:.3}ms]{}", DIM, RESET, BOLD, line_count, RESET, DIM, t01.as_secs_f64()*1000.0, RESET);

    // ── Phase 02: Import Eliminator ───────────────────────────
    let t0 = std::time::Instant::now();
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
    println!("  {}[{:.3}ms]{}", DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 03: Lexer ───────────────────────────────────────
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 03:{} {}LEXER{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut lexer = pydead_bib::frontend::python::py_lexer::PyLexer::new(&preprocessed);
    let tokens = lexer.tokenize();
    let indent_count = tokens.iter().filter(|t| matches!(t, pydead_bib::frontend::python::py_lexer::PyToken::Indent)).count();
    let dedent_count = tokens.iter().filter(|t| matches!(t, pydead_bib::frontend::python::py_lexer::PyToken::Dedent)).count();
    println!("  {}tokens:{}  {}{}{}", DIM, RESET, BOLD, tokens.len(), RESET);
    println!("  {}indent:{}  {}/{} pares  {}[{:.3}ms]{}", DIM, RESET, indent_count, dedent_count, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 04: Parser ──────────────────────────────────────
    let t0 = std::time::Instant::now();
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
    println!("  {}AST:{}     {}{}{} top-level nodes  {}[{:.3}ms]{}", DIM, RESET, BOLD, ast.body.len(), RESET, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);
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
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 05:{} {}TYPE INFERENCER{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut inferencer = pydead_bib::frontend::python::py_types::PyTypeInferencer::new();
    let typed_ast = inferencer.infer(&ast);
    println!("  {}{}✓{} inference complete  {}[{:.3}ms]{}", GREEN, BOLD, RESET, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);
    // v4.0 FASE 3: Show struct layouts
    if !inferencer.class_layouts.is_empty() {
        for (cls_name, layout) in &inferencer.class_layouts {
            let parent = layout.parent.as_deref().unwrap_or("none");
            println!("  {}├─{} {}class{} {}{}{} ({} fields, {}B, parent: {})",
                DIM, RESET, YELLOW, RESET, BOLD, cls_name, RESET,
                layout.fields.len(), layout.total_size, parent);
        }
    }
    if !inferencer.dynamic_fallbacks.is_empty() {
        for warn in &inferencer.dynamic_fallbacks {
            println!("  {}⚠{} {}{}{}", YELLOW, RESET, DIM, warn, RESET);
        }
    }

    // ── Phase 06: IR Generation ───────────────────────────────
    let t0 = std::time::Instant::now();
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
    println!("  {}GIL:{}     {}eliminado{} — ownership estático  {}[{:.3}ms]{}", DIM, RESET, GREEN, RESET, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 06b: Optimizer (v3.0) ─────────────────────────────
    let t0 = std::time::Instant::now();
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
    println!("  {}dead code:{}  {}{}{} removed  {}[{:.3}ms]{}", DIM, RESET, BOLD, total_eliminated, RESET, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 07: UB Detector ─────────────────────────────────
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 07:{} {}UB DETECTOR{}", BOLD, BLUE, RESET, CYAN, RESET);
    let mut ub_detector = pydead_bib::middle::ub_detector::PyUBDetector::new()
        .with_file(input_file.to_string());
    let reports = ub_detector.analyze(&ir);
    let ub_errors = reports.iter().filter(|r| matches!(r.severity, pydead_bib::middle::ub_detector::UBSeverity::Error)).count();
    let ub_warnings = reports.iter().filter(|r| matches!(r.severity, pydead_bib::middle::ub_detector::UBSeverity::Warning)).count();
    let ub_infos = reports.iter().filter(|r| matches!(r.severity, pydead_bib::middle::ub_detector::UBSeverity::Info)).count();
    if reports.is_empty() {
        println!("  {}{}✓ CLEAN{} — 0 errors, 0 warnings, 0 infos  {}[{:.3}ms]{}", GREEN, BOLD, RESET, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);
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
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 08:{} {}OPTIMIZER{}", BOLD, BLUE, RESET, CYAN, RESET);
    let optimized = pydead_bib::backend::optimizer::optimize(&ir);
    println!("  {}folded:{}  {} constants", DIM, RESET, optimized.stats.constants_folded);
    println!("  {}dead:{}    {} removed", DIM, RESET, optimized.stats.dead_code_removed);
    println!("  {}SIMD:{}    {} vectorized  {}[{:.3}ms]{}", DIM, RESET, optimized.stats.simd_vectorized, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 09: Register Allocator ──────────────────────────
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 09:{} {}REGISTER ALLOCATOR{}", BOLD, BLUE, RESET, CYAN, RESET);
    let allocated = pydead_bib::backend::reg_alloc::allocate(&optimized);
    println!("  {}vars:{}    {} → {}{}{} regs, {} spills",
        DIM, RESET, allocated.stats.total_vars, GREEN, allocated.stats.registers_used, RESET, allocated.stats.spills);
    for func in &allocated.functions {
        println!("  {}├─{} {} {}(stack: {}B, regs: {}){}", DIM, RESET, func.name, DIM, func.stack_size, func.reg_map.len(), RESET);
    }
    println!("  {}[{:.3}ms]{}", DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 10: ISA Compiler ────────────────────────────────
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 10:{} {}ISA COMPILER (x86-64){}", BOLD, BLUE, RESET, CYAN, RESET);
    let compiled = pydead_bib::backend::isa::compile(&allocated, target);
    println!("  {}.text:{}   {}{}{} bytes", DIM, RESET, BOLD, compiled.stats.total_bytes, RESET);
    println!("  {}funcs:{}   {}", DIM, RESET, compiled.stats.functions_compiled);
    println!("  {}instrs:{}  {}  {}[{:.3}ms]{}", DIM, RESET, compiled.stats.instructions_emitted, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 11: BG Stamp ────────────────────────────────────
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 11:{} {}BINARY GUARDIAN{}", BOLD, BLUE, RESET, CYAN, RESET);
    let stamped = pydead_bib::backend::bg::stamp(&compiled);
    println!("  {}magic:{}   {}0x{:08X}{}", DIM, RESET, MAGENTA, stamped.stamp.magic, RESET);
    println!("  {}ver:{}     0x{:04X}", DIM, RESET, stamped.stamp.version);
    println!("  {}chksum:{}  {}0x{:08X}{}  {}[{:.3}ms]{}", DIM, RESET, YELLOW, stamped.stamp.checksum, RESET, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 12: Output (PE/ELF/Po) ─────────────────────────
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 12:{} {}OUTPUT{}", BOLD, BLUE, RESET, CYAN, RESET);
    let binary = pydead_bib::backend::output::emit(&stamped);
    let stats = pydead_bib::backend::output::binary_stats(&binary, &stamped);
    println!("  {}format:{}  {}", DIM, RESET, stats.target);
    println!("  {}.text:{}   {} bytes", DIM, RESET, stats.text_bytes);
    println!("  {}.data:{}   {} bytes", DIM, RESET, stats.data_bytes);
    println!("  {}total:{}   {}{}{} bytes  {}[{:.3}ms]{}", DIM, RESET, BOLD, stats.total_bytes, RESET, DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);

    // ── Phase 13: Write binary ────────────────────────────────
    let t0 = std::time::Instant::now();
    println!("{}{}▸ Phase 13:{} {}WRITE{}", BOLD, BLUE, RESET, CYAN, RESET);
    fs::write(&output_file, &binary)?;
    println!("  {}→{} {}{}{} ({} bytes)  {}[{:.3}ms]{}", GREEN, RESET, BOLD, output_file, RESET, binary.len(), DIM, t0.elapsed().as_secs_f64()*1000.0, RESET);
    println!();

    // ── Time to RAM metric ──────────────────────────────────
    let total_pipeline = pipeline_start.elapsed();
    println!("  {}{}⚡ time-to-binary:{} {}{:.3}ms{}", BOLD, MAGENTA, RESET, BOLD, total_pipeline.as_secs_f64()*1000.0, RESET);

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
// JIT KILLER v2.0 — "El CPU no piensa — ya sabe"
// ============================================================
fn jit_execute(input_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(input_file)
        .map_err(|e| format!("Cannot read '{}': {}", input_file, e))?;

    let pipeline_start = std::time::Instant::now();

    // MEJORA 6: CPU Feature Detection — detect once
    let cpu = pydead_bib::backend::jit::detect_cpu_features();

    // MEJORA 3: Thermal Cache — hash source
    let source_hash = pydead_bib::backend::jit::hash_source(&source);

    println!();
    println!("{}{}╔════════════════════════════════════════════════════════════════╗{}", BOLD, MAGENTA, RESET);
    println!("{}{}║   PyDead-BIB JIT KILLER v2.0 💀🦈                             ║{}", BOLD, MAGENTA, RESET);
    println!("{}{}║   \"El CPU no piensa — ya sabe. La RAM no espera — ya recibe\" ║{}", DIM, MAGENTA, RESET);
    println!("{}{}╚════════════════════════════════════════════════════════════════╝{}", BOLD, MAGENTA, RESET);
    println!("  {}Source:{} {}{}{}", DIM, RESET, BOLD, input_file, RESET);
    println!("  {}Mode:{}   {}in-memory (no .exe){}", DIM, RESET, GREEN, RESET);
    println!("  {}CPU:{}    {}{}{}", DIM, RESET, CYAN, cpu.brand, RESET);
    println!("  {}AVX2:{}   {} {}SSE4.2:{} {} {}BMI2:{} {}",
        DIM, RESET,
        if cpu.has_avx2 { format!("{}✓{}", GREEN, RESET) } else { format!("{}✗{}", RED, RESET) },
        DIM, RESET,
        if cpu.has_sse42 { format!("{}✓{}", GREEN, RESET) } else { format!("{}✗{}", RED, RESET) },
        DIM, RESET,
        if cpu.has_bmi2 { format!("{}✓{}", GREEN, RESET) } else { format!("{}✗{}", RED, RESET) });
    println!("  {}Hash:{}   {}0x{:016X}{}", DIM, RESET, DIM, source_hash, RESET);
    println!();

    // Preprocess
    let t0 = std::time::Instant::now();
    let mut preprocessor = pydead_bib::frontend::python::py_preprocessor::PyPreprocessor::new();
    let preprocessed = preprocessor.process(&source);
    let t_preprocess = t0.elapsed();

    // Lex
    let t0 = std::time::Instant::now();
    let mut lexer = pydead_bib::frontend::python::py_lexer::PyLexer::new(&preprocessed);
    let tokens = lexer.tokenize();
    let t_lex = t0.elapsed();

    // Parse
    let t0 = std::time::Instant::now();
    let mut parser = pydead_bib::frontend::python::py_parser::PyParser::new(tokens);
    let ast = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
    let t_parse = t0.elapsed();

    // Type inference
    let t0 = std::time::Instant::now();
    let mut inferencer = pydead_bib::frontend::python::py_types::PyTypeInferencer::new();
    let typed_ast = inferencer.infer(&ast);
    let t_types = t0.elapsed();

    // IR gen
    let t0 = std::time::Instant::now();
    let ir = compile_python_to_ir(&typed_ast).map_err(|e| format!("IR error: {}", e))?;
    let t_ir = t0.elapsed();

    // Optimize
    let t0 = std::time::Instant::now();
    let mut ir = ir;
    for func in ir.functions.iter_mut() {
        pydead_bib::middle::ir::optimize_function(func);
    }
    let t_opt = t0.elapsed();

    // UB detect
    let t0 = std::time::Instant::now();
    let mut ub_detector = pydead_bib::middle::ub_detector::PyUBDetector::new()
        .with_file(input_file.to_string());
    let reports = ub_detector.analyze(&ir);
    let ub_errors = reports.iter().filter(|r| matches!(r.severity, pydead_bib::middle::ub_detector::UBSeverity::Error)).count();
    if ub_errors > 0 {
        return Err(format!("{} UB error(s)", ub_errors).into());
    }
    let t_ub = t0.elapsed();

    // Optimize pass 2
    let t0 = std::time::Instant::now();
    let optimized = pydead_bib::backend::optimizer::optimize(&ir);
    let t_opt2 = t0.elapsed();

    // Register allocate
    let t0 = std::time::Instant::now();
    let allocated = pydead_bib::backend::reg_alloc::allocate(&optimized);
    let t_regalloc = t0.elapsed();

    // ISA compile
    let t0 = std::time::Instant::now();
    let target = Target::from_str("windows");
    let compiled = pydead_bib::backend::isa::compile(&allocated, target);
    let t_isa = t0.elapsed();

    let compile_elapsed = pipeline_start.elapsed();

    println!("  {}{}▸ Compile pipeline:{}", BOLD, BLUE, RESET);
    println!("  {}  preprocess:{} {:.3}ms", DIM, RESET, t_preprocess.as_secs_f64()*1000.0);
    println!("  {}  lex:{}        {:.3}ms", DIM, RESET, t_lex.as_secs_f64()*1000.0);
    println!("  {}  parse:{}      {:.3}ms", DIM, RESET, t_parse.as_secs_f64()*1000.0);
    println!("  {}  types:{}      {:.3}ms", DIM, RESET, t_types.as_secs_f64()*1000.0);
    println!("  {}  IR gen:{}     {:.3}ms  ({} funcs, {} stmts)", DIM, RESET, t_ir.as_secs_f64()*1000.0, ir.functions.len(), ir.statement_count());
    println!("  {}  optimize:{}   {:.3}ms", DIM, RESET, t_opt.as_secs_f64()*1000.0);
    println!("  {}  UB detect:{}  {:.3}ms", DIM, RESET, t_ub.as_secs_f64()*1000.0);
    println!("  {}  optimize2:{}  {:.3}ms", DIM, RESET, t_opt2.as_secs_f64()*1000.0);
    println!("  {}  regalloc:{}   {:.3}ms  ({} regs, {} spills)", DIM, RESET, t_regalloc.as_secs_f64()*1000.0, allocated.stats.registers_used, allocated.stats.spills);
    println!("  {}  ISA x86-64:{} {:.3}ms  ({} bytes .text, {} bytes .data)", DIM, RESET, t_isa.as_secs_f64()*1000.0, compiled.stats.total_bytes, compiled.data.len());
    println!("  {}{}⚡ compile:{} {}{:.3}ms{}", BOLD, MAGENTA, RESET, BOLD, compile_elapsed.as_secs_f64()*1000.0, RESET);
    println!();

    // JIT execute — MEJORA 7: Instant Entry
    println!("  {}{}▸ JIT KILLER:{} dispatch table → instant image → VirtualAlloc → JMP", BOLD, MAGENTA, RESET);

    match pydead_bib::backend::jit::execute_in_memory_with_stats(
        &compiled.text,
        &compiled.data,
        compiled.entry_point,
        &compiled.data_fixups,
        &compiled.data_labels,
        &compiled.iat_fixups,
        source_hash,
    ) {
        Ok((code, stats)) => {
            println!();
            println!("  {}{}▸ JIT Stats:{}", BOLD, YELLOW, RESET);
            println!("  {}  alloc:{}   {:.3}ms  (.text RWX, .data RW)", DIM, RESET, stats.alloc_ms);
            println!("  {}  patch:{}   {:.3}ms  (instant image pre-patched)", DIM, RESET, stats.patch_ms);
            println!("  {}  exec:{}    {:.3}ms", DIM, RESET, stats.exec_ms);
            println!("  {}  .text:{}   {} bytes", DIM, RESET, stats.text_bytes);
            println!("  {}  .data:{}   {} bytes", DIM, RESET, stats.data_bytes);
            println!("  {}  cache:{}   {}", DIM, RESET, if stats.cache_hit { format!("{}HIT{}", GREEN, RESET) } else { format!("{}COLD{}", YELLOW, RESET) });
            let total_ms = compile_elapsed.as_secs_f64()*1000.0 + stats.total_ms;
            println!();
            println!("  {}{}⚡ time-to-RAM:{} {}{:.3}ms{} (compile {:.3}ms + JIT {:.3}ms)",
                BOLD, MAGENTA, RESET, BOLD, total_ms, RESET,
                compile_elapsed.as_secs_f64()*1000.0, stats.total_ms);
            println!("  {}{}✓ JIT complete{} (exit: {})", GREEN, BOLD, RESET, code);
        }
        Err(e) => {
            println!("  {}{}✗ JIT error:{} {}", RED, BOLD, RESET, e);
        }
    }

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
        // Metal_Dead — Full AI system rewritten for PyDead-BIB
        ("Metal_Dead/__init__.py", "MD Init"),
        ("Metal_Dead/__main__.py", "MD Main"),
        ("Metal_Dead/core/__init__.py", "MD Core Init"),
        ("Metal_Dead/core/tokenizer.py", "MD Tokenizer"),
        ("Metal_Dead/core/model.py", "MD Model"),
        ("Metal_Dead/core/memory.py", "MD Memory"),
        ("Metal_Dead/core/context.py", "MD Context"),
        ("Metal_Dead/core/intelligence.py", "MD Intelligence"),
        ("Metal_Dead/core/metal_dead.py", "MD Core"),
        ("Metal_Dead/core/metal_dead_smart.py", "MD Smart"),
        ("Metal_Dead/core/metal_dead_cpu.py", "MD CPU"),
        ("Metal_Dead/core/cpu_compute.py", "MD CPU Compute"),
        ("Metal_Dead/integrations/__init__.py", "MD Integ Init"),
        ("Metal_Dead/integrations/gpu_compute.py", "MD GPU"),
        ("Metal_Dead/integrations/gpu_advanced.py", "MD GPU Adv"),
        ("Metal_Dead/integrations/adead_accelerator.py", "MD Accelerator"),
        ("Metal_Dead/integrations/metal_dead_smart_gpu.py", "MD Smart GPU"),
        ("Metal_Dead/ui/__init__.py", "MD UI Init"),
        ("Metal_Dead/ui/chat.py", "MD Chat"),
        ("Metal_Dead/ui/cli.py", "MD CLI"),
        ("Metal_Dead/jarvis/__init__.py", "MD Jarvis Init"),
        ("Metal_Dead/jarvis/jarvis.py", "MD JARVIS"),
        ("Metal_Dead/tools/__init__.py", "MD Tools Init"),
        ("Metal_Dead/tools/web_search.py", "MD Web Search"),
        ("Metal_Dead/tools/file_manager.py", "MD File Mgr"),
        ("Metal_Dead/tools/data_analyst.py", "MD Data Analyst"),
        ("Metal_Dead/integrations/llm_bridge.py", "MD LLM Bridge"),
        // v4.0 — FASE 1: Global State Tracker
        ("tests/test_globals.py", "Globals"),
        // v4.0 — FASE 3: Type Inferencer v2
        ("tests/test_inheritance_v2.py", "Inherit v2"),
        // v4.0 — FASE 4: GPU Dispatch
        ("tests/test_gpu_dispatch.py", "GPU Dispatch"),
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
        println!("{}{}║   ✅ TOTAL: {}/{} PASS                                       ║{}", BOLD, GREEN, passed, total, RESET);
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
