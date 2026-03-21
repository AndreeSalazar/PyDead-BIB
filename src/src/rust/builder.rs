// ============================================================
// ADead-BIB Builder v3.0
// C/C++ Native Compiler — Orchestrator
// ============================================================
// Connects: C/C++ Frontend → Optimizer → ISA Compiler → Backend
// ============================================================

use crate::backend::{elf, pe};
use crate::frontend::ast::{Function, FunctionAttributes, Program};
use crate::frontend::c::compile_c_to_program;
use crate::frontend::cpp::compile_cpp_to_program;
use crate::frontend::type_checker::TypeChecker;
use crate::isa::c_isa::CIsaCompiler;
use crate::isa::cpp_isa::CppIsaCompiler;
use crate::isa::isa_compiler::Target;
use crate::middle::ub_detector::{UBDetector, UBReport};
use crate::optimizer::binary_optimizer::{BinaryOptimizer, OptLevel};
use crate::optimizer::branch_detector::{BranchDetector, BranchPattern};
use crate::optimizer::branchless::BranchlessTransformer;
use crate::optimizer::const_prop::ConstPropagator;
use crate::optimizer::dead_code::DeadCodeEliminator;
use crate::output::OutputFormat;
use std::fs;
use std::path::Path;

/// Source language for compilation
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SourceLanguage {
    C,
    Cpp,
}

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub target: Target,
    pub optimize: bool,
    pub output_path: String,
    pub verbose: bool,
    pub opt_level: OptLevel,
    pub size_optimize: bool,
    pub language: SourceLanguage,
    pub warn_ub: bool,
    pub output_format: OutputFormat,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            target: Target::Windows,
            optimize: true,
            output_path: "output.exe".to_string(),
            verbose: false,
            opt_level: OptLevel::Basic,
            size_optimize: false,
            language: SourceLanguage::C,
            warn_ub: false,
            output_format: OutputFormat::WindowsPE,
        }
    }
}

impl BuildOptions {
    /// Infer output_format from target if not explicitly set
    pub fn infer_output_format(target: Target) -> OutputFormat {
        match target {
            Target::Windows => OutputFormat::WindowsPE,
            Target::Linux => OutputFormat::LinuxELF,
            Target::Raw => OutputFormat::FastOS64,
        }
    }
}

pub struct Builder;

impl Builder {
    /// Compile C source code to executable
    pub fn build_c(source: &str, options: BuildOptions) -> Result<(), Box<dyn std::error::Error>> {
        if options.verbose {
            println!("Starting C build for target: {:?}", options.target);
        }

        // 1. Frontend: C Parsing
        if options.verbose {
            println!("Step 1: Parsing C...");
        }
        let mut program = compile_c_to_program(source)?;

        // Continue with common pipeline
        Self::compile_program(&mut program, options)
    }

    /// Compile C++ source code to executable
    pub fn build_cpp(
        source: &str,
        options: BuildOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if options.verbose {
            println!("Starting C++ build for target: {:?}", options.target);
        }

        // 1. Frontend: C++ Parsing
        if options.verbose {
            println!("Step 1: Parsing C++...");
        }
        let mut program = compile_cpp_to_program(source)?;

        // Convert C++ classes to functions
        if !program.classes.is_empty() {
            Self::convert_classes_to_functions(&mut program);
        }

        // Continue with common pipeline
        Self::compile_program(&mut program, options)
    }

    /// Build from file, auto-detecting language by extension
    pub fn build_file(path: &str, options: BuildOptions) -> Result<(), Box<dyn std::error::Error>> {
        let source = fs::read_to_string(path)?;
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "c" | "h" => Self::build_c(&source, options),
            "cpp" | "cxx" | "cc" | "hpp" | "hxx" => Self::build_cpp(&source, options),
            _ => Err(format!("Unknown file extension: .{}", ext).into()),
        }
    }

    /// Common compilation pipeline (after parsing)
    fn compile_program(
        program: &mut Program,
        options: BuildOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 2. Type Checking (Static Analysis)
        if options.verbose {
            println!("Step 2: Type Checking...");
        }
        let mut type_checker = TypeChecker::new();
        let _types = type_checker.check_program(program);

        // 2.1 UB Detection (ANTES de optimizar — cobertura total)
        if options.verbose {
            println!("Step 2.1: UB Detection...");
        }
        let mut ub_detector = if options.warn_ub {
            UBDetector::new().with_warn_mode()
        } else {
            UBDetector::new()
        };
        let _ub_reports = ub_detector.analyze(program);
        ub_detector.print_reports();
        if !options.warn_ub && ub_detector.has_errors() {
            return Err("UB detected in strict mode. Fix UB or use --warn-ub.".into());
        }

        // 2.5. Constant Folding (always on — safe and always beneficial)
        if options.verbose {
            println!("Step 2.5: Constant Folding...");
        }
        let folder = crate::optimizer::const_fold::ConstFolder::new();
        folder.fold_program(program);

        // 3. Optimization
        if options.optimize {
            if options.verbose {
                println!("Step 3: Optimizing...");
            }
            Self::apply_optimizations(program);
        }

        // 4. Backend: ISA Compilation (Language-specific ISA → bytes)
        //    c_isa.rs  → C99 sizeof real, alignment rules
        //    cpp_isa.rs → C++98 vtable, this pointer, inheritance layout
        //    isa_compiler.rs → ADead-BIB native (8-byte everything, fallback)
        if options.verbose {
            println!("Step 4: ISA Compilation ({:?})...", options.language);
        }
        let (opcodes, data, _iat_offsets, _string_offsets, ir_ref) = match options.language {
            SourceLanguage::C => {
                let mut c_compiler = CIsaCompiler::new(options.target);
                let result = c_compiler.compile(program);
                // We need the IR for ISA optimization — get it before moving
                (result.0, result.1, result.2, result.3, None::<()>)
            }
            SourceLanguage::Cpp => {
                let mut cpp_compiler = CppIsaCompiler::new(options.target);
                let result = cpp_compiler.compile(program);
                (result.0, result.1, result.2, result.3, None::<()>)
            }
        };
        let _ = ir_ref;

        // 4.5. ISA-Level Optimization is now handled internally by each ISA compiler.
        // Binary-level optimization below still applies to raw bytes.
        let optimized_opcodes = opcodes;

        // 4.6. Binary-Level Optimization (operates on raw bytes)
        let final_opcodes = if options.size_optimize {
            let mut binary_opt = BinaryOptimizer::new(options.opt_level);
            let optimized = binary_opt.optimize(&optimized_opcodes);
            if options.verbose {
                let stats = binary_opt.get_stats();
                println!(
                    "   Bytes: {} → {} (saved: {} bytes, {:.1}%)",
                    stats.original_size,
                    stats.optimized_size,
                    stats.bytes_saved,
                    if stats.original_size > 0 {
                        (stats.bytes_saved as f64 / stats.original_size as f64) * 100.0
                    } else {
                        0.0
                    }
                );
            }
            optimized
        } else {
            optimized_opcodes
        };

        // 5. Linking / Binary Generation
        if options.verbose {
            println!("Step 5: Writing Binary to {}...", options.output_path);
        }
        match options.output_format {
            OutputFormat::WindowsPE => {
                pe::generate_pe_with_offsets(
                    &final_opcodes,
                    &data,
                    &options.output_path,
                    &_iat_offsets,
                    &_string_offsets,
                )?;
            }
            OutputFormat::LinuxELF => {
                elf::generate_elf(&final_opcodes, &data, &options.output_path)?;
            }
            OutputFormat::FastOS64 | OutputFormat::FastOS128 | OutputFormat::FastOS256 => {
                let po = crate::output::po::PoOutput::new();
                let size = po.generate(&final_opcodes, &data, &options.output_path)?;
                if options.verbose {
                    println!("   .Po size: {} bytes", size);
                }
            }
            OutputFormat::Boot16 | OutputFormat::Boot32 | OutputFormat::All => {
                // Boot and multi-target not yet implemented in builder
                let po = crate::output::po::PoOutput::new();
                let size = po.generate(&final_opcodes, &data, &options.output_path)?;
                if options.verbose {
                    println!("   .Po size: {} bytes (fallback)", size);
                }
            }
        }

        if options.verbose {
            println!("Build successful!");
        }
        Ok(())
    }

    /// Convert C++ classes to standalone functions
    fn convert_classes_to_functions(program: &mut Program) {
        for class in &program.classes {
            // Convert each method to a function with Class::method name
            for method in &class.methods {
                let func_name = format!("{}::{}", class.name, method.name);
                let func = Function {
                    name: func_name,
                    params: method.params.clone(),
                    return_type: method.return_type.clone(),
                    resolved_return_type: method.resolved_return_type.clone(),
                    body: method.body.clone(),
                    attributes: FunctionAttributes::default(),
                };
                program.functions.push(func);
            }

            // Convert constructor if present
            if let Some(ref constructor) = class.constructor {
                let func_name = format!("{}::__init__", class.name);
                let func = Function {
                    name: func_name,
                    params: constructor.params.clone(),
                    return_type: constructor.return_type.clone(),
                    resolved_return_type: constructor.resolved_return_type.clone(),
                    body: constructor.body.clone(),
                    attributes: FunctionAttributes::default(),
                };
                program.functions.push(func);
            }
        }
    }

    /// Apply AST-level optimizations
    fn apply_optimizations(program: &mut Program) {
        // 1. Constant propagation
        let propagator = ConstPropagator::new();
        propagator.propagate(program);

        // 2. Dead code elimination (SIN explotar UB)
        let dce = DeadCodeEliminator::new();
        dce.eliminate(program);

        // 3. Branchless transforms
        let detector = BranchDetector::new();
        let transformer = BranchlessTransformer::new();

        for func in &mut program.functions {
            let patterns = detector.analyze(&func.body);

            if !patterns.is_empty() {
                let mut new_body = Vec::new();
                let mut i = 0;

                while i < func.body.len() {
                    let mut transformed = false;

                    for pattern in &patterns {
                        if let Some(replacement) =
                            Self::try_transform_stmt(&func.body[i], pattern, &transformer)
                        {
                            new_body.extend(replacement);
                            transformed = true;
                            break;
                        }
                    }

                    if !transformed {
                        new_body.push(func.body[i].clone());
                    }

                    i += 1;
                }

                func.body = new_body;
            }
        }
    }

    /// Try to transform a statement using a detected pattern
    fn try_transform_stmt(
        stmt: &crate::frontend::ast::Stmt,
        pattern: &crate::optimizer::branch_detector::BranchPattern,
        transformer: &BranchlessTransformer,
    ) -> Option<Vec<crate::frontend::ast::Stmt>> {
        match (stmt, pattern) {
            (crate::frontend::ast::Stmt::If { .. }, BranchPattern::ReLU { .. })
            | (crate::frontend::ast::Stmt::If { .. }, BranchPattern::Select { .. }) => {
                Some(transformer.transform(pattern.clone()))
            }
            _ => None,
        }
    }

    /// Parse + type check + UB detection WITHOUT codegen
    pub fn check_file(
        path: &str,
        warn_ub: bool,
        verbose: bool,
    ) -> Result<Vec<UBReport>, Box<dyn std::error::Error>> {
        if verbose {
            println!("Checking file: {}", path);
        }
        let source = fs::read_to_string(path)?;
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let program = match ext {
            "c" | "h" => compile_c_to_program(&source)?,
            "cpp" | "cxx" | "cc" => compile_cpp_to_program(&source)?,
            _ => return Err(format!("Unknown extension: .{}", ext).into()),
        };

        // Type check
        let mut type_checker = TypeChecker::new();
        type_checker.check_program(&program);

        // UB Detection
        let mut detector = if warn_ub {
            UBDetector::new().with_warn_mode()
        } else {
            UBDetector::new()
        };
        let reports = detector.analyze(&program);
        detector.print_reports();

        if !warn_ub && detector.has_errors() {
            Err("UB detected.".into())
        } else {
            Ok(reports)
        }
    }
}
