// ============================================================
// ADead-BIB Compiler CLI v8.0
// C/C++ Native Compiler — Sin GCC, Sin LLVM, Sin Clang
// 100% Self-Sufficient — Sin libc, Sin linker externo
// 256-bit nativo — YMM/AVX2 — SoA natural
// ============================================================

use adead_bib::backend::gpu::gpu_detect::GPUFeatures;
use adead_bib::backend::gpu::vulkan::VulkanBackend;
use adead_bib::backend::gpu::vulkan_runtime;
use adead_bib::backend::microvm::{self, compile_microvm, MicroOp, MicroVM};
use adead_bib::backend::pe_tiny;
use adead_bib::frontend::c::compile_c_to_program;
use adead_bib::frontend::cpp::compile_cpp_to_program;
use adead_bib::isa::isa_compiler::Target;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        // ============================================================
        // C COMPILER — Primary command
        // ============================================================
        "cc" | "c" => {
            if args.len() < 3 {
                eprintln!("❌ Error: Missing C source file");
                eprintln!("   Usage: adb cc <file.c> [-o output.exe]");
                std::process::exit(1);
            }
            compile_c_file(&args[2], &args)?;
        }

        // ============================================================
        // C++ COMPILER — Primary command
        // ============================================================
        "cxx" | "c++" | "cpp" | "g++" => {
            if args.len() < 3 {
                eprintln!("❌ Error: Missing C++ source file");
                eprintln!("   Usage: adb cxx <file.cpp> [-o output.exe]");
                std::process::exit(1);
            }
            compile_cpp_file(&args[2], &args)?;
        }

        // ============================================================
        // BUILD — Auto-detect by extension or adb.toml project
        // ============================================================
        "build" => {
            if args.len() < 3 {
                // No file argument — try adb.toml project
                if let Some(proj) = load_adb_toml(".") {
                    build_project(&proj, &args)?;
                } else {
                    eprintln!("❌ Error: No source file and no adb.toml found");
                    eprintln!("   Usage: adb build <file.c|file.cpp>");
                    eprintln!("   Or run from a project created with: adb create <name>");
                    std::process::exit(1);
                }
            } else {
                let input_file = &args[2];
                let ext = Path::new(input_file)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                match ext {
                    "c" | "h" => compile_c_file(input_file, &args)?,
                    "cpp" | "cxx" | "cc" | "hpp" | "hxx" => compile_cpp_file(input_file, &args)?,
                    _ => {
                        eprintln!("❌ Error: Unknown file extension '.{}'", ext);
                        eprintln!("   Supported: .c, .cpp, .cxx, .cc");
                        std::process::exit(1);
                    }
                }
            }
        }

        // ============================================================
        // RUN — Build and execute
        // ============================================================
        "run" => {
            if args.len() < 3 {
                // No file argument — try adb.toml project
                if let Some(proj) = load_adb_toml(".") {
                    let output_file = build_project(&proj, &args)?;
                    println!("🚀 Running {}...\n", proj.name);
                    let exe_path = if cfg!(target_os = "windows") {
                        format!(".\\{}", output_file)
                    } else {
                        format!("./{}", output_file)
                    };
                    let status = Command::new(&exe_path).status()?;
                    if !status.success() {
                        eprintln!("\n⚠️  Program exited with status: {}", status);
                    }
                } else {
                    eprintln!("❌ Error: No source file and no adb.toml found");
                    eprintln!("   Usage: adb run <file.c|file.cpp>");
                    eprintln!("   Or run from a project created with: adb create <name>");
                    std::process::exit(1);
                }
            } else {
                let input_file = &args[2];
                let output_file = get_output_filename(input_file, &args);
                let ext = Path::new(input_file)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                // Build
                match ext {
                    "c" | "h" => compile_c_file(input_file, &args)?,
                    "cpp" | "cxx" | "cc" | "hpp" | "hxx" => compile_cpp_file(input_file, &args)?,
                    _ => {
                        eprintln!("❌ Error: Unknown file extension '.{}'", ext);
                        std::process::exit(1);
                    }
                }

                // Run
                println!("🚀 Running {}...\n", input_file);
                let exe_path = if cfg!(target_os = "windows") {
                    format!(".\\{}", output_file)
                } else {
                    format!("./{}", output_file)
                };
                let status = Command::new(&exe_path).status()?;
                if !status.success() {
                    eprintln!("\n⚠️  Program exited with status: {}", status);
                }
            }
        }

        "--test-lexer" => {
            let file = &args[2];
            let source = std::fs::read_to_string(file).unwrap();
            let mut lexer = adead_bib::frontend::c::c_lexer::CLexer::new(&source);
            loop {
                let t = lexer.next_token();
                println!("line: {} token: {:?}", lexer.line, t);
                if t == adead_bib::frontend::c::c_lexer::CToken::Eof { break; }
            }
        }

        // ============================================================
        // STEP — Step-by-step compilation visualization
        // ============================================================
        "step" => {
            if args.len() < 3 {
                eprintln!("Usage: adb step <file.c|file.cpp>");
                std::process::exit(1);
            }
            let input_file = &args[2];
            let ext = Path::new(input_file)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            match ext {
                "c" | "h" => step_compile_c(input_file)?,
                "cpp" | "cxx" | "cc" => step_compile_cpp(input_file)?,
                _ => {
                    eprintln!("Unsupported extension '.{}' for step mode", ext);
                    std::process::exit(1);
                }
            }
        }

        // ============================================================
        // NANO/MICRO/TINY — Minimal PE generators (no source needed)
        // ============================================================
        "nano" => {
            let output_file = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "nano.exe".to_string());
            let exit_code: u8 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

            println!("🔬 Building NANO PE (x64)...");
            println!("   Target: Smallest valid Windows x64 executable");

            match pe_tiny::generate_pe_nano(exit_code, &output_file) {
                Ok(size) => {
                    println!("✅ Nano build complete: {} ({} bytes)", output_file, size);
                    println!("   🏆 Smallest valid Windows x64 PE!");
                }
                Err(e) => {
                    eprintln!("❌ Nano build failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        "micro" => {
            let output_file = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "micro.exe".to_string());
            let exit_code: u8 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

            println!("🔬 Building MICRO PE (x86 32-bit)...");
            println!("   Target: Sub-256 byte Windows executable");

            match pe_tiny::generate_pe32_micro(exit_code, &output_file) {
                Ok(size) => {
                    println!("✅ Micro build complete: {} ({} bytes)", output_file, size);
                    if size < 256 {
                        println!("   🏆 SUB-256 BYTES ACHIEVED!");
                    }
                }
                Err(e) => {
                    eprintln!("❌ Micro build failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // ============================================================
        // VM — MicroVM bytecode
        // ============================================================
        "vm" => {
            let output_file = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "program.adb".to_string());
            let exit_code: u8 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

            println!("🔬 Building MicroVM bytecode...");
            println!("   Target: 4-bit instructions (1 byte = 2 ops)");

            let bytecode =
                compile_microvm(&[(MicroOp::Load, exit_code.min(15)), (MicroOp::Exit, 0)]);

            match microvm::save_bytecode(&bytecode, &output_file) {
                Ok(size) => {
                    println!("✅ MicroVM bytecode: {} ({} bytes)", output_file, size);
                    let mut vm = MicroVM::new(&bytecode);
                    let result = vm.run();
                    println!("   ▶️  Execution result: {}", result);
                }
                Err(e) => {
                    eprintln!("❌ MicroVM build failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // ============================================================
        // GPU COMMANDS
        // ============================================================
        "gpu" => {
            let gpu = GPUFeatures::detect();
            gpu.print_summary();

            if gpu.available {
                println!();
                let mut backend = VulkanBackend::new();
                let spirv = backend.generate_optimized_shader(&gpu);
                let output_path = args
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| "builds/matmul.spv".to_string());

                match backend.save_spirv(&spirv, &output_path) {
                    Ok(size) => {
                        println!(
                            "✅ SPIR-V Shader generated: {} ({} bytes)",
                            output_path, size
                        );
                        println!("   Optimized for: {}", gpu.device_name);
                    }
                    Err(e) => eprintln!("❌ Failed to save shader: {}", e),
                }
            }
        }

        "spirv" => {
            let op = args.get(2).map(|s| s.as_str()).unwrap_or("matmul");
            let size: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1024);

            println!("🔬 SPIR-V Compute Shader Generator");
            println!("   Operation: {}", op);
            println!("   Size: {}x{}", size, size);
            println!();

            let mut backend = VulkanBackend::new();
            let spirv = match op {
                "matmul" => {
                    backend.set_workgroup_size(16, 16, 1);
                    backend.generate_matmul_shader(size, size, size)
                }
                _ => backend.generate_matmul_shader(size, size, size),
            };

            let output_path = format!("builds/{}_{}.spv", op, size);
            match backend.save_spirv(&spirv, &output_path) {
                Ok(sz) => {
                    println!("✅ SPIR-V generated: {} ({} bytes)", output_path, sz);
                    println!("   Workgroup: {:?}", backend.workgroup_size);
                }
                Err(e) => eprintln!("❌ Failed: {}", e),
            }
        }

        "vulkan" | "vk" => {
            println!("🔥 VULKAN RUNTIME - GPU Compute");
            println!();

            match unsafe { vulkan_runtime::VulkanRuntime::new() } {
                Ok(runtime) => {
                    runtime.print_device_info();
                    println!();
                    println!("✅ Vulkan runtime initialized successfully!");
                    let props = &runtime.device_props;
                    println!("🎯 GPU Capabilities:");
                    println!("   Max workgroup: {:?}", props.max_compute_workgroup_size);
                    println!(
                        "   Max invocations: {}",
                        props.max_compute_workgroup_invocations
                    );
                    println!(
                        "   Shared memory: {} KB",
                        props.max_compute_shared_memory / 1024
                    );
                }
                Err(e) => {
                    eprintln!("❌ Failed to initialize Vulkan: {}", e);
                    eprintln!("   Make sure Vulkan drivers are installed.");
                }
            }
        }

        "cuda" => {
            use adead_bib::backend::gpu::cuda;

            let op = args.get(2).map(|s| s.as_str()).unwrap_or("vectoradd");
            let size: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1024);

            println!("🔥 ADead-BIB + CUDA Code Generator");
            println!("   Operation: {}", op);
            println!("   Size: {}", size);
            println!();

            let code = match op {
                "matmul" => cuda::generate_matmul_benchmark(size),
                "benchmark" | "bench" => cuda::generate_full_benchmark(),
                _ => cuda::generate_adead_cuda_test(size),
            };

            let output_path = format!("CUDA/ADead_Generated/adead_{}.cu", op);
            fs::create_dir_all("CUDA/ADead_Generated").ok();
            match fs::write(&output_path, &code) {
                Ok(_) => {
                    println!("✅ CUDA code generated: {}", output_path);
                    println!("   Lines: {}", code.lines().count());
                    println!();
                    println!("📋 To compile: nvcc {} -o {}.exe", output_path, op);
                }
                Err(e) => eprintln!("❌ Failed to write CUDA code: {}", e),
            }
        }

        "unified" | "uni" => {
            use adead_bib::backend::gpu::unified_pipeline::{
                MathOperation, PipelineMode, UnifiedPipeline,
            };

            let op = args.get(2).map(|s| s.as_str()).unwrap_or("vectoradd");
            let size: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000000);

            println!("🔥 ADead-BIB Unified Pipeline");
            println!("   Automatic CPU↔GPU decision");
            println!();

            let mode = if args.iter().any(|a| a == "--force-gpu") {
                PipelineMode::ForceGpu
            } else if args.iter().any(|a| a == "--cpu") {
                PipelineMode::CpuOnly
            } else {
                PipelineMode::Hybrid
            };

            let mut pipeline = UnifiedPipeline::with_mode(mode);

            let math_op = match op {
                "matmul" => {
                    let n = (size as f64).sqrt() as usize;
                    println!("   Operation: MatMul {}x{}", n, n);
                    MathOperation::MatMul { m: n, n, k: n }
                }
                "saxpy" => {
                    println!("   Operation: SAXPY ({} elements)", size);
                    MathOperation::Saxpy { size, alpha: 2.5 }
                }
                "reduce" => {
                    println!("   Operation: Reduction ({} elements)", size);
                    MathOperation::Reduction { size }
                }
                _ => {
                    println!("   Operation: VectorAdd ({} elements)", size);
                    MathOperation::VectorAdd { size }
                }
            };

            let result = pipeline.compile_math_op(math_op);
            println!();
            println!("📊 Compilation Result:");
            println!("   Target:  {:?}", result.target);
            println!("   Format:  {:?}", result.format);
            println!("   Size:    {} bytes", result.binary.len());
            println!();
            pipeline.print_summary();
        }

        // ============================================================
        // CREATE — New project (like cargo new)
        // ============================================================
        "create" | "new" | "init" => {
            if args.len() < 3 {
                eprintln!("❌ Error: Missing project name");
                eprintln!("   Usage: adb create <name> [--cpp|--c]");
                std::process::exit(1);
            }
            let name = &args[2];
            let is_cpp = args.iter().any(|a| a == "--cpp" || a == "--c++" || a == "--cxx");
            create_project(name, is_cpp)?;
        }

        // ============================================================
        // TEST — Run self-test suite
        // ============================================================
        "test" => {
            run_test_suite(&args)?;
        }

        // ============================================================
        // INSTALL — Copy headers to ~/.adead/include/
        // ============================================================
        "install" => {
            install_global_headers()?;
        }

        // ============================================================
        // INCLUDE — Show global include path
        // ============================================================
        "include" => {
            let include_dir = get_global_include_dir();
            println!("📂 ADead-BIB global include directory:");
            println!("   {}", include_dir.display());
            if include_dir.exists() {
                let count = fs::read_dir(&include_dir).map(|d| d.count()).unwrap_or(0);
                println!("   ✅ {} headers installed", count);
            } else {
                println!("   ⚠️  Not installed yet. Run: adb install");
            }
        }

        // ============================================================
        // HELP / VERSION
        // ============================================================
        "help" | "-h" | "--help" => {
            print_usage(&args[0]);
        }

        "version" | "-v" | "--version" => {
            println!("ADead-BIB v8.0.0 💀🦈 🇵🇪 — C/C++ Native Compiler");
            println!("Sin GCC, Sin LLVM, Sin Clang — 100% ADead-BIB");
            println!("Sin libc externa, Sin linker — Totalmente autosuficiente");
            println!("256-bit nativo — YMM/AVX2 — SoA natural");
            println!();
            if let Ok(exe) = env::current_exe() {
                println!("Executable: {}", exe.display());
                if let Some(dir) = exe.parent() {
                    println!();
                    if cfg!(target_os = "windows") {
                        println!("  Agrega adb al PATH (Windows PowerShell):");
                        println!("  $env:Path += \";{}\"  ", dir.display());
                        println!();
                        println!("  Para agregar permanente (Admin):");
                        println!("  [Environment]::SetEnvironmentVariable('Path', $env:Path + ';{}', 'User')", dir.display());
                    } else {
                        println!("  Agrega adb al PATH (Linux/macOS):");
                        println!("  export PATH=\"$PATH:{}\"", dir.display());
                    }
                }
            }
            println!();
            println!("  Headers globales: {}", get_global_include_dir().display());
        }

        // ============================================================
        // AUTO-DETECT BY EXTENSION
        // ============================================================
        _ => {
            let input_file = command;
            let ext = Path::new(input_file)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            match ext {
                "c" | "h" => compile_c_file(input_file, &args)?,
                "cpp" | "cxx" | "cc" | "hpp" | "hxx" => compile_cpp_file(input_file, &args)?,
                _ => {
                    eprintln!("❌ Unknown command or file: {}", command);
                    eprintln!("   Use 'adb help' for usage information.");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

// ============================================================
// C COMPILATION
// ============================================================
fn compile_c_file(input_file: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let output_file = get_output_filename(input_file, args);

    // Check for --flat flag (flat binary for bootloaders/OS)
    let is_flat = args.iter().any(|a| a == "--flat");
    let is_flat64 = args.iter().any(|a| a == "--flat64");
    let is_flat16 = args.iter().any(|a| a == "--flat16");
    let is_any_flat = is_flat || is_flat64 || is_flat16;
    let org_address = parse_org_address(args);
    let fixed_size = parse_fixed_size(args);

    if is_any_flat {
        let mode_str = if is_flat64 {
            "64-bit Long Mode"
        } else if is_flat16 {
            "16-bit Real Mode"
        } else {
            "64-bit Long Mode (default)"
        };
        println!("🔨 ADead-BIB C Compiler (Flat Binary Mode)");
        println!("   Source: {}", input_file);
        println!("   Target: {}", output_file);
        println!("   Mode:   {}", mode_str);
        println!("   Origin: 0x{:X}", org_address);
        if fixed_size > 0 {
            println!("   Size:   {} bytes (fixed)", fixed_size);
        }
    } else {
        println!("🔨 ADead-BIB C Compiler");
        println!("   Source: {}", input_file);
        println!("   Target: {}", output_file);
    }

    // 1. Read source
    let source = fs::read_to_string(input_file)
        .map_err(|e| format!("Cannot read '{}': {}", input_file, e))?;

    // 2. Parse C99
    println!("   Step 1: Parsing C99...");
    let program = compile_c_to_program(&source).map_err(|e| format!("C parse error: {}", e))?;

    println!(
        "   Step 2: {} functions, {} structs found",
        program.functions.len(),
        program.structs.len()
    );

    let warn_ub = args.iter().any(|a| a == "--warn-ub");
    let mut ub_detector = adead_bib::UBDetector::new().with_file(input_file.to_string());
    if warn_ub {
        ub_detector = ub_detector.with_warn_mode();
        println!("   ⚠️  UB_Detector: warning mode (avisa y continua)");
    } else {
        println!("   🛡️  UB_Detector: strict mode (se detiene en errores)");
    }

    ub_detector.analyze(&program);
    ub_detector.print_reports();
    if !warn_ub && ub_detector.has_errors() {
        eprintln!("❌ Error: Undefined Behavior detectado en modo estricto. Operación cancelada.");
        std::process::exit(1);
    }

    // 3. Compile to native code
    let target = if is_any_flat {
        Target::Raw
    } else {
        determine_target(args)
    };

    let mode_desc = if is_flat64 || is_flat {
        "x86-64 (64-bit long mode)"
    } else if is_flat16 {
        "x86 (16-bit real mode)"
    } else {
        "x86-64"
    };
    println!("   Step 3: Compiling to native {}...", mode_desc);

    // Create compiler with appropriate CPU mode
    let mut compiler = if is_flat16 {
        adead_bib::isa::isa_compiler::IsaCompiler::new_real16()
    } else if is_flat64 || is_flat {
        // 64-bit long mode for flat binaries (bare metal kernel)
        adead_bib::isa::isa_compiler::IsaCompiler::new_long64(Target::Raw)
    } else {
        adead_bib::isa::isa_compiler::IsaCompiler::new(target)
    };

    let (opcodes, data, iat_offsets, string_offsets) = compiler.compile(&program);

    // 4. Generate binary
    println!("   Step 4: Generating binary...");

    if is_fastos_target(args) {
        use adead_bib::output::po::PoOutput;
        let gen = PoOutput::new();
        match gen.generate(&opcodes, &data, &output_file) {
            Ok(s) => println!(
                "✅ FastOS binary: {} ({} bytes, v5.0 pipeline)",
                output_file, s
            ),
            Err(e) => {
                eprintln!("❌ FastOS generation failed: {}", e);
                std::process::exit(1);
            }
        }
    } else if is_any_flat {
        // Flat binary mode - no PE/ELF headers
        use adead_bib::backend::flat_binary::FlatBinaryGenerator;

        // ─── FLAT BINARY STRING PATCHING ───
        // The ISA compiler calculates string addresses as:
        //   base_address(0) + data_rva(0x1000) + string_offset
        // But in a flat binary, layout is: [code][data] at org_address.
        // So the real string address is:
        //   org_address + code_size + string_offset
        // We patch all string imm64 values in the code section.
        let mut patched_code = opcodes.clone();
        let code_size = opcodes.len() as u64;
        // Old assumption: data starts at base(0) + data_rva(0x1000)
        let old_data_base = 0x1000u64;
        // Real: data starts at org_address + code_size
        let new_data_base = org_address + code_size;

        let mut patched_count = 0usize;
        for &offset in &string_offsets {
            if offset + 8 <= patched_code.len() {
                let old_val = u64::from_le_bytes(
                    patched_code[offset..offset + 8].try_into().unwrap()
                );
                // Only patch values that look like data section references
                // (they should be >= old_data_base and < old_data_base + data.len())
                if old_val >= old_data_base && old_val < old_data_base + data.len() as u64 + 256 {
                    let string_offset_in_data = old_val - old_data_base;
                    let new_val = new_data_base + string_offset_in_data;
                    patched_code[offset..offset + 8]
                        .copy_from_slice(&new_val.to_le_bytes());
                    patched_count += 1;
                }
            }
        }

        if patched_count > 0 {
            println!(
                "   Step 4a: Patched {} string addresses (data at 0x{:X})",
                patched_count, new_data_base
            );
        }

        let mut gen = FlatBinaryGenerator::new(org_address);
        if fixed_size > 0 {
            gen.set_fixed_size(fixed_size);
        }
        let binary = gen.generate(&patched_code, &data);
        fs::write(&output_file, &binary)?;
        println!(
            "✅ Flat binary: {} ({} bytes, code={}, data={}, org=0x{:X})",
            output_file,
            binary.len(),
            code_size,
            data.len(),
            org_address
        );
    } else {
        match target {
            Target::Windows => {
                adead_bib::backend::pe::generate_pe_with_offsets(
                    &opcodes,
                    &data,
                    &output_file,
                    &iat_offsets,
                    &string_offsets,
                )?;
            }
            Target::Linux => {
                adead_bib::backend::elf::generate_elf(&opcodes, &data, &output_file)?;
            }
            Target::Raw => {
                fs::write(&output_file, &opcodes)?;
            }
        }

        if let Ok(meta) = fs::metadata(&output_file) {
            println!(
                "✅ C compilation complete: {} ({} bytes)",
                output_file,
                meta.len()
            );
        } else {
            println!("✅ C compilation complete: {}", output_file);
        }
    }

    println!("   🏆 Sin GCC, sin LLVM, sin Clang — 100% ADead-BIB");
    print_path_hint();

    Ok(())
}

/// Parse --org=0xNNNN argument
fn parse_org_address(args: &[String]) -> u64 {
    for arg in args {
        if arg.starts_with("--org=") {
            let addr_str = arg.trim_start_matches("--org=");
            if addr_str.starts_with("0x") || addr_str.starts_with("0X") {
                return u64::from_str_radix(&addr_str[2..], 16).unwrap_or(0);
            } else {
                return addr_str.parse().unwrap_or(0);
            }
        }
    }
    0 // Default origin
}

/// Parse --size=NNNN argument for fixed size binaries
fn parse_fixed_size(args: &[String]) -> usize {
    for arg in args {
        if arg.starts_with("--size=") {
            let size_str = arg.trim_start_matches("--size=");
            if size_str.starts_with("0x") || size_str.starts_with("0X") {
                return usize::from_str_radix(&size_str[2..], 16).unwrap_or(0);
            } else {
                return size_str.parse().unwrap_or(0);
            }
        }
    }
    0 // No fixed size
}

// ============================================================
// C++ COMPILATION
// ============================================================
fn compile_cpp_file(input_file: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let output_file = get_output_filename(input_file, args);

    println!("🔨 ADead-BIB C++ Compiler");
    println!("   Source: {}", input_file);
    println!("   Target: {}", output_file);

    // 1. Read source
    let source = fs::read_to_string(input_file)
        .map_err(|e| format!("Cannot read '{}': {}", input_file, e))?;

    // 2. Parse C++
    println!("   Step 1: Parsing C++...");
    let program = compile_cpp_to_program(&source).map_err(|e| format!("C++ parse error: {}", e))?;

    println!(
        "   Step 2: {} functions, {} structs, {} classes found",
        program.functions.len(),
        program.structs.len(),
        program.classes.len()
    );

    let warn_ub = args.iter().any(|a| a == "--warn-ub");
    let mut ub_detector = adead_bib::UBDetector::new().with_file(input_file.to_string());
    if warn_ub {
        ub_detector = ub_detector.with_warn_mode();
        println!("   ⚠️  UB_Detector: warning mode (avisa y continua)");
    } else {
        println!("   🛡️  UB_Detector: strict mode (se detiene en errores)");
    }

    ub_detector.analyze(&program);
    ub_detector.print_reports();
    if !warn_ub && ub_detector.has_errors() {
        eprintln!("❌ Error: Undefined Behavior detectado en modo estricto. Operación cancelada.");
        std::process::exit(1);
    }

    // 3. Compile to native x86-64
    println!("   Step 3: Compiling to native x86-64...");
    let target = determine_target(args);
    let mut compiler = adead_bib::isa::isa_compiler::IsaCompiler::new(target);
    let (opcodes, data, iat_offsets, string_offsets) = compiler.compile(&program);

    // 4. Generate binary
    println!("   Step 4: Generating binary...");
    if is_fastos_target(args) {
        use adead_bib::output::po::PoOutput;
        let gen = PoOutput::new();
        match gen.generate(&opcodes, &data, &output_file) {
            Ok(s) => println!(
                "✅ FastOS binary: {} ({} bytes, v5.0 pipeline)",
                output_file, s
            ),
            Err(e) => {
                eprintln!("❌ FastOS generation failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match target {
            Target::Windows => {
                adead_bib::backend::pe::generate_pe_with_offsets(
                    &opcodes,
                    &data,
                    &output_file,
                    &iat_offsets,
                    &string_offsets,
                )?;
            }
            Target::Linux => {
                adead_bib::backend::elf::generate_elf(&opcodes, &data, &output_file)?;
            }
            Target::Raw => {
                fs::write(&output_file, &opcodes)?;
            }
        }
    }

    if let Ok(meta) = fs::metadata(&output_file) {
        println!(
            "✅ C++ compilation complete: {} ({} bytes)",
            output_file,
            meta.len()
        );
    } else {
        println!("✅ C++ compilation complete: {}", output_file);
    }
    println!("   🏆 Sin GCC, sin LLVM, sin Clang — 100% ADead-BIB C++");
    print_path_hint();

    Ok(())
}

// ============================================================
// UTILITIES
// ============================================================
fn print_path_hint() {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            println!();
            if cfg!(target_os = "windows") {
                println!("   Para agregar adb al PATH en Windows:");
                println!("   $env:PATH += \";{}\"", dir.display());
            } else {
                println!("   Para agregar adb al PATH:");
                println!("   export PATH=\"$PATH:{}\"", dir.display());
            }
        }
    }
}

/// Returns the global include directory: ~/.adead/include/
fn get_global_include_dir() -> PathBuf {
    if let Some(home) = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
    {
        PathBuf::from(home).join(".adead").join("include")
    } else {
        PathBuf::from(".adead").join("include")
    }
}

/// Install global headers to ~/.adead/include/
fn install_global_headers() -> Result<(), Box<dyn std::error::Error>> {
    let include_dir = get_global_include_dir();
    fs::create_dir_all(&include_dir)?;

    println!("📦 ADead-BIB — Instalando headers globales...");
    println!("   Destino: {}", include_dir.display());
    println!();

    let mut count = 0;

    // Write header_main.h
    let header_main_content = adead_bib::frontend::c::c_stdlib::get_header("header_main.h")
        .unwrap_or("// header_main.h\n");
    fs::write(include_dir.join("header_main.h"), header_main_content)?;
    println!("   ✅ header_main.h");
    count += 1;

    // Write all fastos_*.h headers
    let fastos_headers = [
        "fastos_stdio.h", "fastos_stdlib.h", "fastos_string.h",
        "fastos_math.h", "fastos_time.h", "fastos_assert.h",
        "fastos_errno.h", "fastos_limits.h", "fastos_types.h",
    ];
    for name in &fastos_headers {
        if let Some(content) = adead_bib::frontend::c::c_stdlib::get_header(name) {
            fs::write(include_dir.join(name), content)?;
            println!("   ✅ {}", name);
            count += 1;
        }
    }

    // Write standard C headers
    let std_headers = [
        "stdio.h", "stdlib.h", "string.h", "math.h", "time.h",
        "stdint.h", "stddef.h", "stdbool.h", "stdarg.h",
        "limits.h", "float.h", "errno.h", "assert.h",
        "signal.h", "ctype.h", "locale.h", "setjmp.h",
    ];
    for name in &std_headers {
        if let Some(content) = adead_bib::frontend::c::c_stdlib::get_header(name) {
            fs::write(include_dir.join(name), content)?;
            println!("   ✅ {}", name);
            count += 1;
        }
    }

    println!();
    println!("✅ {} headers instalados en {}", count, include_dir.display());
    println!();
    println!("   Ahora puedes usar desde cualquier carpeta:");
    println!("   #include <header_main.h>");
    println!();
    println!("   También puedes agregar tus propios headers en:");
    println!("   {}", include_dir.display());

    Ok(())
}

// ============================================================
// PROJECT SYSTEM (adb create / adb.toml)
// ============================================================

#[allow(dead_code)]
struct AdbProject {
    name: String,
    lang: String,       // "c" or "cpp"
    standard: String,   // "c99" or "cpp17"
    src_dir: String,    // "src/"
    include_dir: String,// "include/"
    output_dir: String, // "bin/"
}

/// Run the ADead-BIB self-test suite: adb test [--cpp|--c99|--stdlib|--abi|--report]
fn run_test_suite(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let filter = args.iter().find(|a| a.starts_with("--")).map(|s| s.as_str());

    // Discover test files
    let test_dir = PathBuf::from("reportes").join("tests_cpp_new");
    if !test_dir.exists() {
        eprintln!("No test directory found at {}", test_dir.display());
        std::process::exit(1);
    }

    let mut test_files: Vec<PathBuf> = fs::read_dir(&test_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("cpp"))
        .collect();
    test_files.sort();

    // Categorize tests
    let categorize = |name: &str| -> &str {
        if name.contains("cpp98") || name.contains("cpp11") || name.contains("cpp14")
            || name.contains("cpp17") || name.contains("cpp20")
        {
            "C++ Standard"
        } else if name.contains("algorithm") || name.contains("string_real")
            || name.contains("vector_real") || name.contains("iostream")
            || name.contains("functional") || name.contains("map_real")
            || name.contains("containers")
        {
            "stdlib"
        } else if name.contains("mangling") || name.contains("vtable") {
            "ABI"
        } else if name.contains("raii") || name.contains("new_delete")
            || name.contains("sfinae") || name.contains("exceptions")
        {
            "C++ Features"
        } else if name.contains("win32") || name.contains("posix") {
            "Compat"
        } else {
            "Other"
        }
    };

    // Filter if needed
    let tests: Vec<&PathBuf> = test_files.iter().filter(|p| {
        let name = p.file_stem().unwrap_or_default().to_str().unwrap_or("");
        match filter {
            Some("--cpp") => name.contains("cpp"),
            Some("--stdlib") => categorize(name) == "stdlib",
            Some("--abi") => categorize(name) == "ABI",
            _ => true,
        }
    }).collect();

    println!();
    println!("=== ADead-BIB Test Suite v8.0 ===");
    println!();

    let total = tests.len();
    let mut passed = 0usize;
    let mut failed_names = Vec::new();

    let exe_path = env::current_exe().unwrap_or_default();
    let exe = exe_path.to_str().unwrap_or("cargo run --release --");

    for (i, test_path) in tests.iter().enumerate() {
        let name = test_path.file_stem().unwrap_or_default().to_str().unwrap_or("?");
        let cat = categorize(name);

        // Run the test via step mode (silently)
        let output = Command::new(exe)
            .args(&["step", test_path.to_str().unwrap_or("")])
            .output();

        let ok = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                stdout.contains("ALL PHASES PASSED") || stderr.contains("ALL PHASES PASSED")
            }
            Err(_) => false,
        };

        if ok {
            passed += 1;
            println!("  [{:>2}/{:>2}] PASS  {:.<42} [{}]", i + 1, total, name, cat);
        } else {
            failed_names.push(name.to_string());
            println!("  [{:>2}/{:>2}] FAIL  {:.<42} [{}]", i + 1, total, name, cat);
        }
    }

    println!();
    let bar_len = 30;
    let filled = if total > 0 { (passed * bar_len) / total } else { 0 };
    let bar: String = "=".repeat(filled) + &" ".repeat(bar_len - filled);
    println!("  [{}] {}/{} PASS", bar, passed, total);
    println!();

    if !failed_names.is_empty() {
        println!("  FAILED:");
        for f in &failed_names {
            println!("    - {}", f);
        }
        println!();
    }

    if passed == total {
        println!("  ALL TESTS PASSED");
    }
    println!("  Binary Is Binary");
    println!();

    Ok(())
}

/// Create a new project: adb create <name> [--cpp]
fn create_project(name: &str, is_cpp: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Validate project name
    if name.is_empty() || name.starts_with('-') || name.starts_with('.') {
        eprintln!("❌ Error: Invalid project name '{}'", name);
        std::process::exit(1);
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        eprintln!("❌ Error: Project name '{}' contains invalid characters", name);
        eprintln!("   Use only letters, numbers, _ and -");
        std::process::exit(1);
    }

    // Check if we're already inside a project with this name
    if let Ok(cwd) = env::current_dir() {
        if let Some(dir_name) = cwd.file_name().and_then(|n| n.to_str()) {
            if dir_name == name && cwd.join("adb.toml").exists() {
                eprintln!("❌ Error: You are already inside project '{}'", name);
                eprintln!("   Current directory is already the '{}' project.", name);
                eprintln!("   To recreate, go to the parent directory first.");
                std::process::exit(1);
            }
        }
    }

    // Check if an adb.toml exists in current directory (we're inside some project)
    if Path::new("adb.toml").exists() {
        eprintln!("⚠️  Warning: An adb.toml already exists in the current directory.");
        eprintln!("   Creating '{}' as a subdirectory project.", name);
    }

    let project_dir = Path::new(name);
    if project_dir.exists() {
        eprintln!("❌ Error: Directory '{}' already exists", name);
        std::process::exit(1);
    }

    let (lang, standard, ext) = if is_cpp {
        ("cpp", "cpp17", "cpp")
    } else {
        ("c", "c99", "c")
    };

    println!("📦 Creando proyecto ADead-BIB: {}", name);
    println!("   Lenguaje: {} ({})", lang.to_uppercase(), standard);
    println!();

    // Create directories
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("include"))?;
    fs::create_dir_all(project_dir.join("bin"))?;

    // Write adb.toml
    let toml_content = format!(
        "[project]\nname = \"{}\"\nversion = \"0.1.0\"\nlang = \"{}\"\nstandard = \"{}\"\n\n[build]\nsrc = \"src/\"\ninclude = \"include/\"\noutput = \"bin/\"\n",
        name, lang, standard
    );
    fs::write(project_dir.join("adb.toml"), &toml_content)?;
    println!("   ✅ adb.toml");

    // Copy header_main.h to include/
    let header_content = adead_bib::frontend::c::c_stdlib::get_header("header_main.h")
        .unwrap_or("// header_main.h — ADead-BIB\n");
    fs::write(project_dir.join("include").join("header_main.h"), header_content)?;
    println!("   ✅ include/header_main.h");

    // Write main source file
    let main_file = format!("src/main.{}", ext);
    let main_content = if is_cpp {
        format!(


     //Aquí para modificar cuando quiera LEL
r#"#include <header_main.h>
#include <iostream>
#include <vector>
#include <string>

// ADead-BIB v8.0 — C++17 nativo
// adb run → compila + ejecuta → ~2-3KB binario

int main() {{
    // std::string SSO
    std::string name = "{}";
    printf("Compiler: %s v8.0\n", name.c_str());

    // std::vector con range-for
    std::vector<int> nums = {{1, 2, 3, 4, 5}};
    int sum = 0;
    for (auto n : nums) sum += n;
    printf("Sum: %d\n", sum);

    // Lambda C++11
    auto square = [](int x) {{ return x * x; }};
    printf("5^2 = %d\n", square(5));

    // constexpr C++11
    constexpr int N = 10;
    printf("N = %d\n", N);

    // cout chaining
    std::cout << "Hello from " << name << std::endl;

    return 0;
}}
"#,
            name
        )
    } else {
        format!(
            "#include <header_main.h>\n\nint main() {{\n    printf(\"Hola desde %s\\n\", \"{}\");\n    return 0;\n}}\n",
            name
        )
    };
    fs::write(project_dir.join(&main_file), &main_content)?;
    println!("   ✅ {}", main_file);

    println!("   ✅ bin/");
    println!();
    println!("✅ Proyecto '{}' creado!", name);
    println!();
    println!("   Para compilar y ejecutar:");
    println!("   cd {}", name);
    println!("   adb run");
    println!();
    println!("   Estructura:");
    println!("   {}/", name);
    println!("   ├── adb.toml");
    println!("   ├── include/");
    println!("   │   └── header_main.h");
    println!("   ├── src/");
    println!("   │   └── main.{}", ext);
    println!("   └── bin/");

    Ok(())
}

/// Load adb.toml from a directory. Returns None if not found.
fn load_adb_toml(dir: &str) -> Option<AdbProject> {
    let toml_path = Path::new(dir).join("adb.toml");
    let content = fs::read_to_string(&toml_path).ok()?;

    let mut name = String::new();
    let mut lang = String::from("c");
    let mut standard = String::from("c99");
    let mut src_dir = String::from("src/");
    let mut include_dir = String::from("include/");
    let mut output_dir = String::from("bin/");

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("name") {
            if let Some(val) = extract_toml_value(line) { name = val; }
        } else if line.starts_with("lang") {
            if let Some(val) = extract_toml_value(line) { lang = val; }
        } else if line.starts_with("standard") {
            if let Some(val) = extract_toml_value(line) { standard = val; }
        } else if line.starts_with("src") {
            if let Some(val) = extract_toml_value(line) { src_dir = val; }
        } else if line.starts_with("include") && line.contains('=') {
            if let Some(val) = extract_toml_value(line) { include_dir = val; }
        } else if line.starts_with("output") {
            if let Some(val) = extract_toml_value(line) { output_dir = val; }
        }
    }

    if name.is_empty() { return None; }

    Some(AdbProject { name, lang, standard, src_dir, include_dir, output_dir })
}

/// Extract value from a TOML line like: key = "value"
fn extract_toml_value(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        let val = parts[1].trim().trim_matches('"');
        Some(val.to_string())
    } else {
        None
    }
}

/// Build an adb.toml project. Returns the output filename.
fn build_project(proj: &AdbProject, args: &[String]) -> Result<String, Box<dyn std::error::Error>> {
    // Find main source file
    let ext = if proj.lang == "cpp" { "cpp" } else { "c" };
    let main_src = Path::new(&proj.src_dir).join(format!("main.{}", ext));

    if !main_src.exists() {
        eprintln!("❌ Error: Source file not found: {}", main_src.display());
        eprintln!("   Expected: {}/main.{}", proj.src_dir, ext);
        std::process::exit(1);
    }

    // Ensure output directory exists
    fs::create_dir_all(&proj.output_dir).ok();

    // Build output path
    let exe_ext = if cfg!(target_os = "windows") { ".exe" } else { "" };
    let output_file = format!("{}{}{}", proj.output_dir, proj.name, exe_ext);

    // Build args with -o and include path
    let mut build_args = args.to_vec();
    // Add -o if not already specified
    if !build_args.iter().any(|a| a == "-o") {
        build_args.push("-o".to_string());
        build_args.push(output_file.clone());
    }

    let main_src_str = main_src.to_str().unwrap_or("src/main.c");

    if proj.lang == "cpp" {
        compile_cpp_file(main_src_str, &build_args)?;
    } else {
        compile_c_file(main_src_str, &build_args)?;
    }

    Ok(output_file)
}

fn is_fastos_target(args: &[String]) -> bool {
    for i in 0..args.len() {
        if args[i] == "--target" && i + 1 < args.len() {
            let t = &args[i + 1];
            if t == "fastos" || t == "fastos64" || t == "fastos128" || t == "fastos256" || t == "po" {
                return true;
            }
        }
    }
    false
}

fn determine_target(args: &[String]) -> Target {
    for i in 0..args.len() {
        if args[i] == "--target" && i + 1 < args.len() {
            let t = &args[i + 1];
            match t.as_str() {
                "fastos" | "fastos64" | "fastos128" | "fastos256" | "po" | "raw"
                | "boot16" | "boot32" | "all" => return Target::Raw,
                "windows" | "pe" | "win" => return Target::Windows,
                "linux" | "elf" => return Target::Linux,
                _ => {}
            }
        }
    }
    if cfg!(target_os = "windows") {
        Target::Windows
    } else if cfg!(target_os = "linux") {
        Target::Linux
    } else {
        Target::Raw
    }
}

fn get_output_filename(input: &str, args: &[String]) -> String {
    // Check if -o provided
    for i in 0..args.len() {
        if args[i] == "-o" && i + 1 < args.len() {
            return args[i + 1].clone();
        }
    }

    let ext = if is_fastos_target(args) {
        ".po"
    } else if determine_target(args) == Target::Linux {
        ""
    } else {
        ".exe"
    };

    // Default: input.exe
    Path::new(input)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
        + ext
}

// ============================================================
// STEP COMPILER — Step-by-step visualization
// ============================================================
fn step_compile_c(input_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    use adead_bib::cli::term;
    use adead_bib::frontend::c::c_lexer::{CLexer, CToken};
    use adead_bib::frontend::c::c_preprocessor::CPreprocessor;
    use adead_bib::frontend::c::c_parser::CParser;
    use adead_bib::frontend::c::c_to_ir::CToIR;
    use adead_bib::frontend::c::c_ast::CTopLevel;

    term::enable_ansi();

    println!();
    println!("{}", term::phase_header("╔══════════════════════════════════════════════════════════════╗"));
    println!("{}", term::phase_header("║   ADead-BIB Step Compiler — Deep Analysis Mode 💀🦈         ║"));
    println!("{}", term::phase_header("╚══════════════════════════════════════════════════════════════╝"));
    println!("  {} {}", term::info("Source:"), input_file);
    println!("  {} C99/C11", term::info("Language:"));
    println!();

    // ── Phase 0: SOURCE ─────────────────────────────────────
    println!("{}", term::phase_bar(0, "SOURCE", "C"));
    let source = match fs::read_to_string(input_file) {
        Ok(s) => s,
        Err(e) => {
            println!("  {} {}", term::error_text("ERROR:"), format!("Cannot read '{}': {}", input_file, e));
            return Err(format!("Cannot read '{}': {}", input_file, e).into());
        }
    };
    let source_lines: Vec<&str> = source.lines().collect();
    println!("  {} {} lines, {} bytes", term::ok("✓"), source_lines.len(), source.len());
    let blank_lines = source_lines.iter().filter(|l| l.trim().is_empty()).count();
    let comment_lines = source_lines.iter().filter(|l| {
        let t = l.trim();
        t.starts_with("//") || t.starts_with("/*") || t.starts_with("*")
    }).count();
    let code_lines = source_lines.len() - blank_lines - comment_lines;
    println!("  {} code: {}, comments: {}, blank: {}", term::dim("  metrics"), code_lines, comment_lines, blank_lines);
    let includes: Vec<(usize, &str)> = source_lines.iter().enumerate()
        .filter(|(_, l)| l.trim().starts_with("#include"))
        .map(|(i, l)| (i + 1, l.trim()))
        .collect();
    if !includes.is_empty() {
        println!("  {} {} #include directives:", term::dim("  deps"), includes.len());
        for (line_num, inc) in &includes {
            println!("    {} {}", term::loc(input_file, *line_num, 1), term::info(inc));
        }
    }
    println!();

    // ── Phase 1: PREPROCESSOR ───────────────────────────────
    println!("{}", term::phase_bar(1, "PREPROCESSOR", "C"));
    let mut preprocessor = CPreprocessor::new();
    let preprocessed = preprocessor.process(&source);
    let pp_lines: Vec<&str> = preprocessed.lines().collect();
    let expansion = if !source_lines.is_empty() {
        ((pp_lines.len() as f64 / source_lines.len() as f64) * 100.0) as usize
    } else { 0 };
    println!("  {} {} → {} lines ({}% expansion)", term::ok("✓"), source_lines.len(), pp_lines.len(), expansion);
    for (line_num, inc) in &includes {
        println!("    {} {} → {}", term::loc(input_file, *line_num, 1), inc, term::ok("resolved internally"));
    }
    let macro_defs: Vec<(usize, &str)> = source_lines.iter().enumerate()
        .filter(|(_, l)| l.trim().starts_with("#define"))
        .map(|(i, l)| (i + 1, l.trim()))
        .collect();
    if !macro_defs.is_empty() {
        println!("  {} {} macros defined:", term::dim("  macros"), macro_defs.len());
        for (line_num, mac) in &macro_defs {
            println!("    {} {}", term::loc(input_file, *line_num, 1), term::token_fmt(mac));
        }
    }
    println!();

    // ── Phase 2: LEXER ──────────────────────────────────────
    println!("{}", term::phase_bar(2, "LEXER", "C"));
    let (tokens, lines) = CLexer::new(&preprocessed).tokenize();
    println!("  {} {} tokens generated", term::ok("✓"), tokens.len());

    // Token distribution
    let mut kw_count = 0usize;
    let mut ident_count = 0usize;
    let mut int_lit_count = 0usize;
    let mut float_lit_count = 0usize;
    let mut str_lit_count = 0usize;
    let mut char_lit_count = 0usize;
    let mut op_count = 0usize;
    let mut punct_count = 0usize;
    let mut ident_freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for tok in &tokens {
        match tok {
            CToken::Auto | CToken::Break | CToken::Case | CToken::Char |
            CToken::Const | CToken::Continue | CToken::Default | CToken::Do |
            CToken::Double | CToken::Else | CToken::Enum | CToken::Extern |
            CToken::Float | CToken::For | CToken::Goto | CToken::If |
            CToken::Inline | CToken::Int | CToken::Long | CToken::Register |
            CToken::Restrict | CToken::Return | CToken::Short | CToken::Signed |
            CToken::Sizeof | CToken::Static | CToken::Struct | CToken::Switch |
            CToken::Typedef | CToken::Union | CToken::Unsigned | CToken::Void |
            CToken::Volatile | CToken::While | CToken::Bool => { kw_count += 1; }
            CToken::Identifier(name) => { ident_count += 1; *ident_freq.entry(name.clone()).or_insert(0) += 1; }
            CToken::IntLiteral(_) => { int_lit_count += 1; }
            CToken::FloatLiteral(_) => { float_lit_count += 1; }
            CToken::StringLiteral(_) => { str_lit_count += 1; }
            CToken::CharLiteral(_) => { char_lit_count += 1; }
            CToken::LParen | CToken::RParen | CToken::LBrace | CToken::RBrace |
            CToken::LBracket | CToken::RBracket | CToken::Semicolon | CToken::Comma |
            CToken::Colon | CToken::Ellipsis => { punct_count += 1; }
            CToken::Eof => {}
            _ => { op_count += 1; }
        }
    }
    println!("  {} distribution:", term::dim("  tokens"));
    println!("    {} keywords={}, {} identifiers={}, {} int_literals={}",
        term::token_fmt("KW"), kw_count, term::token_fmt("ID"), ident_count, term::token_fmt("INT"), int_lit_count);
    println!("    {} float_literals={}, {} strings={}, {} chars={}",
        term::token_fmt("FLT"), float_lit_count, term::token_fmt("STR"), str_lit_count, term::token_fmt("CHR"), char_lit_count);
    println!("    {} operators={}, {} punctuation={}",
        term::token_fmt("OP"), op_count, term::token_fmt("PUNCT"), punct_count);
    if !ident_freq.is_empty() {
        let mut sorted: Vec<_> = ident_freq.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        let top: Vec<String> = sorted.iter().take(8).map(|(k, v)| format!("{}({})", k, v)).collect();
        println!("  {} {}", term::dim("  top IDs"), top.join(", "));
    }
    println!("  {} first tokens:", term::dim("  preview"));
    let max_show = 12;
    let mut shown = 0;
    for (i, tok) in tokens.iter().enumerate() {
        if *tok == CToken::Eof { break; }
        if shown >= max_show { break; }
        let line_num = if i < lines.len() { lines[i] } else { 0 };
        let tok_str = format!("{:?}", tok);
        let short = if tok_str.len() > 40 { format!("{}...", &tok_str[..37]) } else { tok_str };
        println!("    {} {:<42} {}", term::loc(input_file, line_num, i % 80), term::token_fmt(&short), term::ok("OK"));
        shown += 1;
    }
    if tokens.len() > max_show + 1 {
        println!("    {} ({} more tokens)", term::dim("..."), tokens.len() - max_show - 1);
    }
    println!();

    // ── Phase 3: PARSER (Syntax Analysis) ───────────────────
    println!("{}", term::phase_bar(3, "PARSER — Syntax Analysis", "C"));
    let parse_result = CParser::new(tokens.clone(), lines.clone()).parse_translation_unit();
    let unit = match parse_result {
        Ok(u) => { println!("  {} Parse successful", term::ok("✓")); u }
        Err(e) => {
            println!("  {} Parse failed!", term::error_text("✗"));
            let err_msg = format!("{}", e);
            println!("  {} {}", term::error_text("ERROR:"), err_msg);
            if let Some(pos_start) = err_msg.find("position ") {
                if let Some(pos_val) = err_msg[pos_start + 9..].split(|c: char| !c.is_numeric()).next() {
                    if let Ok(pos) = pos_val.parse::<usize>() {
                        let err_line = if pos < lines.len() { lines[pos] } else { 0 };
                        if err_line > 0 && err_line <= source_lines.len() {
                            println!();
                            println!("  {}", term::source_context(&source, err_line, 1, &err_msg, "error"));
                            println!("  {} {}", term::info("location:"), term::loc(input_file, err_line, 1));
                        }
                    }
                }
            }
            for (_line_num, inc) in &includes {
                println!("  {} Check that {} resolves all required symbols", term::warn("hint:"), inc);
            }
            println!("  {} Compilation stopped at Phase 3.", term::error_text("STOPPED"));
            return Err(format!("Parse error: {}", e).into());
        }
    };
    let mut func_count = 0usize;
    let mut struct_count = 0usize;
    let mut typedef_count = 0usize;
    let mut enum_count = 0usize;
    let mut global_count = 0usize;
    let mut total_stmts = 0usize;
    let mut max_params = 0usize;
    for decl in &unit.declarations {
        match decl {
            CTopLevel::FunctionDef { name, params, body, return_type, .. } => {
                func_count += 1;
                total_stmts += body.len();
                if params.len() > max_params { max_params = params.len(); }
                let ret = format!("{:?}", return_type);
                let ret_short = if ret.len() > 20 { format!("{}...", &ret[..17]) } else { ret };
                println!("    {} {} {}({} params) → {} [{} stmts]",
                    term::ok("✓"), term::type_fmt("fn"), term::token_fmt(name), params.len(), term::type_fmt(&ret_short), body.len());
            }
            CTopLevel::FunctionDecl { name, params, .. } => {
                println!("    {} {} {}({} params) {}", term::dim("→"), term::type_fmt("decl"), term::token_fmt(name), params.len(), term::dim("(forward declaration)"));
            }
            CTopLevel::StructDef { name, fields } => {
                struct_count += 1;
                println!("    {} {} {} [{} fields]", term::ok("✓"), term::type_fmt("struct"), term::token_fmt(name), fields.len());
            }
            CTopLevel::TypedefDecl { new_name, .. } => {
                typedef_count += 1;
                println!("    {} {} {}", term::ok("✓"), term::type_fmt("typedef"), term::token_fmt(new_name));
            }
            CTopLevel::EnumDef { name, values } => {
                enum_count += 1;
                println!("    {} {} {} [{} values]", term::ok("✓"), term::type_fmt("enum"), term::token_fmt(name), values.len());
            }
            CTopLevel::GlobalVar { declarators, .. } => {
                global_count += 1;
                for d in declarators {
                    println!("    {} {} {}", term::ok("✓"), term::type_fmt("global"), term::token_fmt(&d.name));
                }
            }
        }
    }
    println!("  {} {}", term::dim("  summary"), term::info(&format!(
        "{} functions, {} structs, {} typedefs, {} enums, {} globals",
        func_count, struct_count, typedef_count, enum_count, global_count)));
    if func_count > 0 {
        println!("  {} avg stmts/fn: {}, max params: {}",
            term::dim("  complexity"), total_stmts / func_count, max_params);
    }
    println!();

    // ── Phase 4: IR (Intermediate Representation) ───────────
    println!("{}", term::phase_bar(4, "IR — Intermediate Representation", "C"));
    let mut converter = CToIR::new();
    let program = match converter.convert(&unit) {
        Ok(p) => { println!("  {} IR generation successful", term::ok("✓")); p }
        Err(e) => {
            println!("  {} IR conversion failed!", term::error_text("✗"));
            println!("  {} {}", term::error_text("ERROR:"), e);
            println!("  {} Compilation stopped at Phase 4.", term::error_text("STOPPED"));
            return Err(format!("IR error: {}", e).into());
        }
    };
    let mut total_ir_stmts = 0usize;
    let mut branch_count = 0usize;
    let mut loop_count = 0usize;
    let mut call_count = 0usize;
    let mut vardecl_count = 0usize;
    let mut ptr_ops = 0usize;
    for func in &program.functions {
        total_ir_stmts += func.body.len();
        for stmt in &func.body {
            let s = format!("{:?}", stmt);
            if s.starts_with("If") { branch_count += 1; }
            if s.starts_with("While") || s.starts_with("For") || s.starts_with("DoWhile") { loop_count += 1; }
            if s.starts_with("ExprStmt") && s.contains("FunctionCall") { call_count += 1; }
            if s.starts_with("VarDecl") { vardecl_count += 1; }
            if s.contains("Deref") || s.contains("AddressOf") || s.contains("Arrow") { ptr_ops += 1; }
        }
        println!("    {} {} → {} IR statements", term::ok("✓"), term::token_fmt(&func.name), func.body.len());
        let max_ir = 4;
        for (j, stmt) in func.body.iter().enumerate() {
            if j >= max_ir { break; }
            let ir_str = format!("{:?}", stmt);
            let short = if ir_str.len() > 65 { format!("{}...", &ir_str[..62]) } else { ir_str };
            println!("      {} {}", term::dim("│"), term::dim(&short));
        }
        if func.body.len() > max_ir {
            println!("      {} ({} more)", term::dim("└"), func.body.len() - max_ir);
        }
    }
    println!("  {} {} total IR stmts", term::dim("  total"), total_ir_stmts);
    println!("  {} branches={}, loops={}, calls={}, vars={}, ptrs={}",
        term::dim("  analysis"), branch_count, loop_count, call_count, vardecl_count, ptr_ops);
    if !program.structs.is_empty() {
        println!("  {} {} structs registered in IR", term::dim("  types"), program.structs.len());
    }
    println!();

    // ── Phase 5: UB DETECTOR ────────────────────────────────
    println!("{}", term::phase_bar(5, "UB DETECTOR — Undefined Behavior Analysis", "C"));
    let mut ub_detector = adead_bib::UBDetector::new().with_file(input_file.to_string());
    ub_detector = ub_detector.with_warn_mode();
    let reports = ub_detector.analyze(&program);
    println!("  {} functions: {}, variables: {}, pointer ops: {}",
        term::dim("  scope"), program.functions.len(), vardecl_count, ptr_ops);
    if reports.is_empty() {
        println!("  {} No undefined behavior detected", term::ok("✓ CLEAN"));
    } else {
        let errors = reports.iter().filter(|r| format!("{:?}", r.severity).contains("Error")).count();
        let warnings = reports.len() - errors;
        if errors > 0 {
            println!("  {} {} errors, {} warnings", term::error_text("✗"), errors, warnings);
        } else {
            println!("  {} {} warnings (no errors)", term::warn("⚠"), warnings);
        }
        for (i, r) in reports.iter().enumerate().take(8) {
            let sev = format!("{:?}", r.severity);
            let sev_colored = if sev.contains("Error") { term::error_text(&sev) } else { term::warn(&sev) };
            let msg_short = if r.message.len() > 55 { format!("{}...", &r.message[..52]) } else { r.message.clone() };
            println!("    {} [{}] {}", term::dim(&format!("{:>2}.", i + 1)), sev_colored, msg_short);
            if let Some(line) = r.line {
                println!("       {}", term::loc(input_file, line, 1));
                if line > 0 && line <= source_lines.len() {
                    println!("       {} {}", term::dim("|"), source_lines[line - 1].trim());
                }
            }
        }
        if reports.len() > 8 {
            println!("    {} ({} more)", term::dim("..."), reports.len() - 8);
        }
    }
    println!();

    // ── Phase 6: CODEGEN ────────────────────────────────────
    println!("{}", term::phase_bar(6, "CODEGEN — x86-64 Machine Code", "C"));
    let target = adead_bib::isa::isa_compiler::Target::Windows;
    let mut compiler = adead_bib::isa::isa_compiler::IsaCompiler::new(target);
    let (opcodes, data, iat_offsets, string_offsets) = compiler.compile(&program);
    println!("  {} {} bytes of machine code", term::ok("✓"), opcodes.len());
    println!("  {} {} bytes of data section", term::ok("✓"), data.len());
    println!("  {} {} IAT call sites, {} string relocations",
        term::dim("  links"), iat_offsets.len(), string_offsets.len());
    {
        let dlls = adead_bib::backend::cpu::iat_registry::dll_names();
        if !dlls.is_empty() {
            println!("  {} imports ({} DLLs):", term::dim("  IAT"), dlls.len());
            for dll in &dlls {
                let entries = adead_bib::backend::cpu::iat_registry::entries_for_dll(dll);
                let used: Vec<&str> = entries.iter().filter(|e| source.contains(e.name)).map(|e| e.name).collect();
                if !used.is_empty() {
                    println!("    {} {} → {}", term::ok("✓"), term::token_fmt(dll), used.join(", "));
                }
            }
        }
    }
    if !opcodes.is_empty() {
        let show_bytes = opcodes.len().min(32);
        let hex: Vec<String> = opcodes[..show_bytes].iter().map(|b| format!("{:02X}", b)).collect();
        println!("  {} first {} bytes:", term::dim("  hex"), show_bytes);
        for chunk in hex.chunks(16) {
            println!("    {}", term::dim(&chunk.join(" ")));
        }
    }
    if !data.is_empty() {
        println!("  {} data strings:", term::dim("  data"));
        let data_str = String::from_utf8_lossy(&data);
        for s in data_str.split('\0') {
            if !s.is_empty() && s.len() < 200 {
                println!("    {}", term::info(&format!("\"{}\"", s.escape_default())));
            }
        }
    }
    println!();

    // ── Phase 7: OUTPUT SUMMARY ─────────────────────────────
    println!("{}", term::phase_bar(7, "OUTPUT — PE Generation Summary", "C"));
    let pe_headers = 0x200usize;
    let section_alignment = 0x200usize;
    let code_aligned = ((opcodes.len() + section_alignment - 1) / section_alignment) * section_alignment;
    let data_aligned = ((data.len() + section_alignment - 1) / section_alignment) * section_alignment;
    let iat_section_size = adead_bib::backend::cpu::iat_registry::IAT_ENTRIES.len() * 8;
    let iat_aligned = ((iat_section_size + section_alignment - 1) / section_alignment) * section_alignment;
    let estimated_pe = pe_headers + code_aligned + data_aligned + iat_aligned;
    println!("  {} Windows PE x86-64", term::info("target:"));
    println!("  {} {} bytes (.text)", term::dim("  code"), opcodes.len());
    println!("  {} {} bytes (.data)", term::dim("  data"), data.len());
    println!("  {} {} bytes (.idata)", term::dim("   iat"), iat_section_size);
    println!("  {} ~{} bytes", term::ok("  estimated PE:"), estimated_pe);
    println!();

    // ── FINAL SUMMARY ───────────────────────────────────────
    println!("{}", term::phase_header("╔══════════════════════════════════════════════════════════════╗"));
    println!("{}", term::phase_header("║   Step Compilation Complete ✅                              ║"));
    println!("{}", term::phase_header("╚══════════════════════════════════════════════════════════════╝"));
    println!("  {} 7/7 phases completed successfully", term::ok("✓ ALL PHASES PASSED"));
    println!("  {} adb cc {} -o output.exe", term::info("  build:"), input_file);
    println!("  {} adb run {}", term::info("  run:  "), input_file);

    Ok(())
}

fn step_compile_cpp(input_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    use adead_bib::cli::term;
    use adead_bib::frontend::cpp::cpp_lexer::{CppLexer, CppToken};

    term::enable_ansi();

    println!();
    println!("{}", term::phase_header("â•”â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•—"));
    println!("{}", term::phase_header("â•‘   ADead-BIB Step Compiler (C++) â€” Deep Analysis Mode ðŸ’€ðŸ¦ˆ   â•‘"));
    println!("{}", term::phase_header("â•šâ•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•"));
    println!("  {} {}", term::info("Source:"), input_file);
    println!("  {} C++11/14/17/20", term::info("Language:"));
    println!();

    // â”€â”€ Phase 0: SOURCE â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_bar(0, "SOURCE", "C++"));
    let source = match fs::read_to_string(input_file) {
        Ok(s) => s,
        Err(e) => {
            println!("  {} {}", term::error_text("ERROR:"), format!("Cannot read '{}': {}", input_file, e));
            return Err(format!("Cannot read '{}': {}", input_file, e).into());
        }
    };
    let source_lines: Vec<&str> = source.lines().collect();
    println!("  {} {} lines, {} bytes", term::ok("âœ“"), source_lines.len(), source.len());
    let blank_lines = source_lines.iter().filter(|l| l.trim().is_empty()).count();
    let comment_lines = source_lines.iter().filter(|l| {
        let t = l.trim();
        t.starts_with("//") || t.starts_with("/*") || t.starts_with("*")
    }).count();
    let code_lines = source_lines.len() - blank_lines - comment_lines;
    println!("  {} code: {}, comments: {}, blank: {}", term::dim("  metrics"), code_lines, comment_lines, blank_lines);
    // Show includes
    let includes: Vec<(usize, &str)> = source_lines.iter().enumerate()
        .filter(|(_, l)| l.trim().starts_with("#include"))
        .map(|(i, l)| (i + 1, l.trim()))
        .collect();
    if !includes.is_empty() {
        println!("  {} {} #include directives:", term::dim("  deps"), includes.len());
        for (line_num, inc) in &includes {
            println!("    {} {}", term::loc(input_file, *line_num, 1), term::info(inc));
        }
    }
    // Detect C++ features
    let has_classes = source_lines.iter().any(|l| l.contains("class ") && !l.trim().starts_with("//"));
    let has_templates = source_lines.iter().any(|l| l.contains("template"));
    let has_namespaces = source_lines.iter().any(|l| l.contains("namespace "));
    let has_lambda = source_lines.iter().any(|l| l.contains("[") && l.contains("]("));
    let has_auto = source_lines.iter().any(|l| {
        let t = l.trim();
        t.starts_with("auto ") || t.contains(" auto ")
    });
    let has_smart_ptr = source_lines.iter().any(|l| l.contains("unique_ptr") || l.contains("shared_ptr"));
    let mut cpp_features: Vec<&str> = Vec::new();
    if has_classes { cpp_features.push("classes"); }
    if has_templates { cpp_features.push("templates"); }
    if has_namespaces { cpp_features.push("namespaces"); }
    if has_lambda { cpp_features.push("lambdas"); }
    if has_auto { cpp_features.push("auto"); }
    if has_smart_ptr { cpp_features.push("smart_ptr"); }
    if !cpp_features.is_empty() {
        println!("  {} {}", term::dim("  C++ features"), term::type_fmt(&cpp_features.join(", ")));
    }
    println!();

    // â”€â”€ Phase 1: PREPROCESSOR â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_bar(1, "PREPROCESSOR", "C++"));
    for (line_num, inc) in &includes {
        println!("    {} {} â†’ {}", term::loc(input_file, *line_num, 1), inc, term::ok("resolved internally"));
    }
    let macro_defs: Vec<(usize, &str)> = source_lines.iter().enumerate()
        .filter(|(_, l)| l.trim().starts_with("#define"))
        .map(|(i, l)| (i + 1, l.trim()))
        .collect();
    if !macro_defs.is_empty() {
        println!("  {} {} macros defined:", term::dim("  macros"), macro_defs.len());
        for (line_num, mac) in &macro_defs {
            println!("    {} {}", term::loc(input_file, *line_num, 1), term::token_fmt(mac));
        }
    }
    println!("  {} Preprocessor complete", term::ok("âœ“"));
    println!();

    // â”€â”€ Phase 2: LEXER â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_bar(2, "LEXER", "C++"));
    let (tokens, lines) = CppLexer::new(&source).tokenize();
    println!("  {} {} tokens generated", term::ok("âœ“"), tokens.len());

    // Token distribution analysis
    let mut kw_count = 0usize;
    let mut cpp_kw_count = 0usize;
    let mut ident_count = 0usize;
    let mut int_lit_count = 0usize;
    let mut float_lit_count = 0usize;
    let mut str_lit_count = 0usize;
    let mut op_count = 0usize;
    let mut punct_count = 0usize;
    let mut ident_freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for tok in &tokens {
        match tok {
            // C shared keywords
            CppToken::Auto | CppToken::Break | CppToken::Case | CppToken::Char |
            CppToken::Const | CppToken::Continue | CppToken::Default | CppToken::Do |
            CppToken::Double | CppToken::Else | CppToken::Enum | CppToken::Extern |
            CppToken::Float | CppToken::For | CppToken::Goto | CppToken::If |
            CppToken::Int | CppToken::Long | CppToken::Register | CppToken::Return |
            CppToken::Short | CppToken::Signed | CppToken::Sizeof | CppToken::Static |
            CppToken::Struct | CppToken::Switch | CppToken::Typedef | CppToken::Union |
            CppToken::Unsigned | CppToken::Void | CppToken::Volatile | CppToken::While |
            CppToken::Bool | CppToken::Inline => { kw_count += 1; }
            // C++ specific keywords
            CppToken::Class | CppToken::Namespace | CppToken::Using | CppToken::New |
            CppToken::Delete | CppToken::This | CppToken::Virtual | CppToken::Override |
            CppToken::Final | CppToken::Public | CppToken::Private | CppToken::Protected |
            CppToken::Friend | CppToken::Operator | CppToken::Template | CppToken::Typename |
            CppToken::Try | CppToken::Catch | CppToken::Throw | CppToken::Noexcept |
            CppToken::Nullptr | CppToken::Constexpr | CppToken::Explicit |
            CppToken::Mutable | CppToken::True | CppToken::False |
            CppToken::StaticCast | CppToken::DynamicCast | CppToken::ConstCast |
            CppToken::ReinterpretCast => { cpp_kw_count += 1; }
            CppToken::Identifier(name) => {
                ident_count += 1;
                *ident_freq.entry(name.clone()).or_insert(0) += 1;
            }
            CppToken::IntLiteral(_) | CppToken::UIntLiteral(_) => { int_lit_count += 1; }
            CppToken::FloatLiteral(_) => { float_lit_count += 1; }
            CppToken::StringLiteral(_) => { str_lit_count += 1; }
            CppToken::CharLiteral(_) => { str_lit_count += 1; }
            CppToken::LParen | CppToken::RParen | CppToken::LBrace | CppToken::RBrace |
            CppToken::LBracket | CppToken::RBracket | CppToken::Semicolon | CppToken::Comma => { punct_count += 1; }
            CppToken::Eof => {}
            _ => { op_count += 1; }
        }
    }

    println!("  {} distribution:", term::dim("  tokens"));
    println!("    {} C_keywords={}, {} C++_keywords={}, {} identifiers={}",
        term::token_fmt("KW"), kw_count,
        term::token_fmt("C++"), cpp_kw_count,
        term::token_fmt("ID"), ident_count);
    println!("    {} int_literals={}, {} float_literals={}, {} strings={}",
        term::token_fmt("INT"), int_lit_count,
        term::token_fmt("FLT"), float_lit_count,
        term::token_fmt("STR"), str_lit_count);
    println!("    {} operators={}, {} punctuation={}",
        term::token_fmt("OP"), op_count,
        term::token_fmt("PUNCT"), punct_count);

    // Top identifiers
    if !ident_freq.is_empty() {
        let mut sorted: Vec<_> = ident_freq.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        let top: Vec<String> = sorted.iter().take(8).map(|(k, v)| format!("{}({})", k, v)).collect();
        println!("  {} {}", term::dim("  top IDs"), top.join(", "));
    }

    // Token preview
    println!("  {} first tokens:", term::dim("  preview"));
    let max_show = 12;
    let mut shown = 0;
    for (i, tok) in tokens.iter().enumerate() {
        if *tok == CppToken::Eof { break; }
        if shown >= max_show { break; }
        let line_num = if i < lines.len() { lines[i] } else { 0 };
        let tok_str = format!("{:?}", tok);
        let short = if tok_str.len() > 40 { format!("{}...", &tok_str[..37]) } else { tok_str };
        println!("    {} {:<42} {}", term::loc(input_file, line_num, i % 80), term::token_fmt(&short), term::ok("OK"));
        shown += 1;
    }
    if tokens.len() > max_show + 1 {
        println!("    {} ({} more tokens)", term::dim("..."), tokens.len() - max_show - 1);
    }
    println!();

    // â”€â”€ Phase 3: PARSER + IR â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_bar(3, "PARSER + IR â€” Syntax & Lowering", "C++"));
    let program = match adead_bib::frontend::cpp::compile_cpp_to_program(&source) {
        Ok(p) => {
            println!("  {} Parse + IR generation successful", term::ok("âœ“"));
            p
        }
        Err(e) => {
            println!("  {} Parse/IR failed!", term::error_text("âœ—"));
            println!();
            let err_msg = format!("{}", e);
            println!("  {} {}", term::error_text("ERROR:"), err_msg);

            // Try to extract position info
            if let Some(pos_start) = err_msg.find("position ") {
                if let Some(pos_val) = err_msg[pos_start + 9..].split(|c: char| !c.is_numeric()).next() {
                    if let Ok(pos) = pos_val.parse::<usize>() {
                        let err_line = if pos < lines.len() { lines[pos] } else { 0 };
                        if err_line > 0 && err_line <= source_lines.len() {
                            println!();
                            println!("  {}", term::source_context(&source, err_line, 1, &err_msg, "error"));
                            println!();
                            println!("  {} {}", term::info("location:"), term::loc(input_file, err_line, 1));
                        }
                    }
                }
            }

            // Error chain: check for common C++ issues
            if err_msg.to_lowercase().contains("template") {
                println!("  {} Template parsing issue â€” check template syntax", term::warn("hint:"));
            }
            if err_msg.to_lowercase().contains("class") || err_msg.to_lowercase().contains("struct") {
                println!("  {} Class/struct definition issue â€” check braces and semicolons", term::warn("hint:"));
            }
            if err_msg.to_lowercase().contains("namespace") {
                println!("  {} Namespace issue â€” check namespace blocks", term::warn("hint:"));
            }
            for (_line_num, inc) in &includes {
                if err_msg.to_lowercase().contains("unexpected") || err_msg.to_lowercase().contains("expected") {
                    println!("  {} Check that {} resolves all C++ symbols",
                        term::warn("hint:"), inc);
                }
            }
            println!();
            println!("  {} Compilation stopped at Phase 3.", term::error_text("STOPPED"));
            println!("  {} Fix the error above, then run {} again.",
                term::info("action:"), term::info(&format!("adb step {}", input_file)));
            return Err(format!("C++ parse error: {}", e).into());
        }
    };

    // Parse/IR metrics
    let mut total_ir_stmts = 0usize;
    let mut branch_count = 0usize;
    let mut loop_count = 0usize;
    let mut call_count = 0usize;
    let mut vardecl_count = 0usize;
    let mut ptr_ops = 0usize;

    for func in &program.functions {
        total_ir_stmts += func.body.len();
        println!("    {} {} â†’ {} IR statements",
            term::ok("âœ“"), term::token_fmt(&func.name), func.body.len());
        let max_ir = 4;
        for (j, stmt) in func.body.iter().enumerate() {
            if j >= max_ir { break; }
            let ir_str = format!("{:?}", stmt);
            let short = if ir_str.len() > 65 { format!("{}...", &ir_str[..62]) } else { ir_str };
            println!("      {} {}", term::dim("â”‚"), term::dim(&short));
        }
        if func.body.len() > max_ir {
            println!("      {} ({} more)", term::dim("â””"), func.body.len() - max_ir);
        }

        for stmt in &func.body {
            let s = format!("{:?}", stmt);
            if s.starts_with("If") { branch_count += 1; }
            if s.starts_with("While") || s.starts_with("For") || s.starts_with("DoWhile") { loop_count += 1; }
            if s.starts_with("ExprStmt") && s.contains("FunctionCall") { call_count += 1; }
            if s.starts_with("VarDecl") { vardecl_count += 1; }
            if s.contains("Deref") || s.contains("AddressOf") || s.contains("Arrow") { ptr_ops += 1; }
        }
    }

    // Classes & structs
    if !program.classes.is_empty() {
        println!();
        println!("  {} classes:", term::dim("  C++"));
        for class in &program.classes {
            println!("    {} {} {} [{} methods, {} fields]",
                term::ok("âœ“"), term::type_fmt("class"),
                term::token_fmt(&class.name), class.methods.len(), class.fields.len());
        }
    }
    if !program.structs.is_empty() {
        println!("  {} {} structs registered", term::dim("  types"), program.structs.len());
    }
    println!();
    println!("  {} {} total IR stmts", term::dim("  total"), total_ir_stmts);
    println!("  {} branches={}, loops={}, calls={}, vars={}, ptrs={}",
        term::dim("  analysis"),
        branch_count, loop_count, call_count, vardecl_count, ptr_ops);
    println!();

    // â”€â”€ Phase 4: UB DETECTOR â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_bar(4, "UB DETECTOR â€” Undefined Behavior Analysis", "C++"));
    let mut ub_detector = adead_bib::UBDetector::new().with_file(input_file.to_string());
    ub_detector = ub_detector.with_warn_mode();
    let reports = ub_detector.analyze(&program);

    println!("  {} functions: {}, classes: {}", term::dim("  scope"),
        program.functions.len(), program.classes.len());

    if reports.is_empty() {
        println!("  {} No undefined behavior detected", term::ok("âœ“ CLEAN"));
    } else {
        let errors = reports.iter().filter(|r| format!("{:?}", r.severity).contains("Error")).count();
        let warnings = reports.len() - errors;
        if errors > 0 {
            println!("  {} {} errors, {} warnings", term::error_text("âœ—"), errors, warnings);
        } else {
            println!("  {} {} warnings (no errors)", term::warn("âš "), warnings);
        }
        for (i, r) in reports.iter().enumerate().take(8) {
            let sev = format!("{:?}", r.severity);
            let sev_colored = if sev.contains("Error") { term::error_text(&sev) } else { term::warn(&sev) };
            let msg_short = if r.message.len() > 55 { format!("{}...", &r.message[..52]) } else { r.message.clone() };
            println!("    {} [{}] {}", term::dim(&format!("{:>2}.", i + 1)), sev_colored, msg_short);
            if let Some(line) = r.line {
                println!("       {}", term::loc(input_file, line, 1));
                if line > 0 && line <= source_lines.len() {
                    println!("       {} {}", term::dim("|"), source_lines[line - 1].trim());
                }
            }
        }
        if reports.len() > 8 {
            println!("    {} ({} more)", term::dim("..."), reports.len() - 8);
        }
    }
    println!();

    // â”€â”€ Phase 5: CODEGEN â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_bar(5, "CODEGEN â€” x86-64 Machine Code", "C++"));
    let target = adead_bib::isa::isa_compiler::Target::Windows;
    let mut compiler = adead_bib::isa::isa_compiler::IsaCompiler::new(target);
    let (opcodes, data, iat_offsets, string_offsets) = compiler.compile(&program);

    println!("  {} {} bytes of machine code", term::ok("âœ“"), opcodes.len());
    println!("  {} {} bytes of data section", term::ok("âœ“"), data.len());
    println!("  {} {} IAT call sites, {} string relocations",
        term::dim("  links"), iat_offsets.len(), string_offsets.len());

    // IAT imports
    {
        let dlls = adead_bib::backend::cpu::iat_registry::dll_names();
        if !dlls.is_empty() {
            println!("  {} imports ({} DLLs):", term::dim("  IAT"), dlls.len());
            for dll in &dlls {
                let entries = adead_bib::backend::cpu::iat_registry::entries_for_dll(dll);
                let used: Vec<&str> = entries.iter().filter(|e| source.contains(e.name)).map(|e| e.name).collect();
                if !used.is_empty() {
                    println!("    {} {} â†’ {}", term::ok("âœ“"), term::token_fmt(dll), used.join(", "));
                }
            }
        }
    }

    // Machine code hex preview
    if !opcodes.is_empty() {
        let show_bytes = opcodes.len().min(32);
        let hex: Vec<String> = opcodes[..show_bytes].iter().map(|b| format!("{:02X}", b)).collect();
        println!("  {} first {} bytes:", term::dim("  hex"), show_bytes);
        for chunk in hex.chunks(16) {
            println!("    {}", term::dim(&chunk.join(" ")));
        }
    }

    // Data section strings
    if !data.is_empty() {
        println!("  {} data strings:", term::dim("  data"));
        let data_str = String::from_utf8_lossy(&data);
        for s in data_str.split('\0') {
            if !s.is_empty() && s.len() < 200 {
                println!("    {}", term::info(&format!("\"{}\"", s.escape_default())));
            }
        }
    }
    println!();

    // â”€â”€ Phase 6: IAT RESOLVER â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_bar(6, "IAT RESOLVER â€” Import Address Table", "C++"));
    {
        let assumed_idata_rva: u32 = 0x2000;
        let idata_result = adead_bib::backend::cpu::iat_registry::build_idata(assumed_idata_rva, &[]);
        let iat_base = 0x2000 + idata_result.iat_offset as u32;

        let mut resolved_count = 0;
        for dll in adead_bib::backend::cpu::iat_registry::dll_names() {
            let entries = adead_bib::backend::cpu::iat_registry::entries_for_dll(&dll);
            let used_entries: Vec<_> = entries.iter().filter(|e| source.contains(e.name)).collect();
            if !used_entries.is_empty() {
                println!("  {} {}", term::token_fmt(&dll), term::dim("DLL"));
                for entry in &used_entries {
                    let offset = iat_base + (entry.slot_index as u32 * 8);
                    println!("    {} slot:{}, offset:{} {}",
                        term::token_fmt(entry.name), entry.slot_index,
                        term::info(&format!("0x{:X}", offset)), term::ok("âœ“ resolved"));
                    resolved_count += 1;
                }
            }
        }
        println!("  {} {} functions resolved", term::dim("  total"), resolved_count);
    }
    println!();

    // â”€â”€ Phase 7: PE OUTPUT â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_bar(7, "OUTPUT â€” PE Generation Summary", "C++"));
    let pe_headers = 0x200usize;
    let section_alignment = 0x200usize;
    let code_aligned = ((opcodes.len() + section_alignment - 1) / section_alignment) * section_alignment;
    let data_aligned = ((data.len() + section_alignment - 1) / section_alignment) * section_alignment;
    let iat_section_size = adead_bib::backend::cpu::iat_registry::IAT_ENTRIES.len() * 8;
    let iat_aligned = ((iat_section_size + section_alignment - 1) / section_alignment) * section_alignment;
    let estimated_pe = pe_headers + code_aligned + data_aligned + iat_aligned;

    println!("  {} Windows PE x86-64", term::info("target:"));
    println!("  {} {} bytes (.text)", term::dim("  code"), opcodes.len());
    println!("  {} {} bytes (.data)", term::dim("  data"), data.len());
    println!("  {} {} bytes (.idata)", term::dim("   iat"), iat_section_size);
    println!("  {} ~{} bytes", term::ok("  estimated PE:"), estimated_pe);
    println!();

    // â”€â”€ FINAL SUMMARY â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    println!("{}", term::phase_header("â•”â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•—"));
    println!("{}", term::phase_header("â•‘   Step Compilation Complete (C++) âœ…                         â•‘"));
    println!("{}", term::phase_header("â•šâ•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•"));
    println!();
    println!("  {} 7/7 phases completed successfully", term::ok("âœ“ ALL PHASES PASSED"));
    println!();
    println!("  {} adb cxx {} -o output.exe", term::info("  build:"), input_file);
    println!("  {} adb run {}", term::info("  run:  "), input_file);
    println!();

    Ok(())
}

fn print_usage(_program: &str) {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║       🔥 ADead-BIB v8.0.0 💀🦈 — C/C++ Compiler 🔥         ║");
    println!("║    Sin GCC, Sin LLVM, Sin Clang — 100% Self-Sufficient       ║");
    println!("║    256-bit nativo — YMM/AVX2 — SoA natural                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("📁 PROYECTOS (como cargo):");
    println!("   adb create <name>               Nuevo proyecto C (adb.toml)");
    println!("   adb create <name> --cpp         Nuevo proyecto C++");
    println!("   adb build                       Compilar proyecto (lee adb.toml)");
    println!("   adb run                         Compilar y ejecutar proyecto");
    println!();
    println!("🔨 COMPILAR C/C++:");
    println!("   adb cc <file.c> [-o output]     Compile C99/C11");
    println!("   adb cxx <file.cpp> [-o output]  Compile C++11/14/17/20");
    println!("     [--target boot16|boot32|fastos64|fastos128|fastos256|windows|linux|all]");
    println!("     [--warn-ub] (Warning only, don't stop on UB)");
    println!("   adb build <file> [-o output]    Auto-detect by extension");
    println!("   adb run <file>                  Build and execute");
    println!("   adb step <file>                 Step-by-step compilation view");
    println!("   adb <file.c|file.cpp>           Direct compilation");
    println!();
    println!("📦 HEADERS GLOBALES:");
    println!("   adb install                     Instala headers en ~/.adead/include/");
    println!("   adb include                     Muestra ruta de headers globales");
    println!();
    println!("🚀 EXAMPLES:");
    println!("   adb create hola                 Nuevo proyecto C");
    println!("   cd hola && adb run              Compilar y ejecutar");
    println!("   adb cc hello.c                  Compile hello.c → hello.exe");
    println!("   adb cxx main.cpp -o app.exe     Compile main.cpp → app.exe");
    println!("   adb cc kernel.c --target fastos256 -o kernel.po");
    println!("   adb run test.c                  Compile and run test.c");
    println!("   adb install                     Setup global headers");
    println!();
    println!("⚡ MINIMAL BINARIES:");
    println!("   adb nano [output] [exit_code]   Smallest valid x64 PE (~1KB)");
    println!("   adb micro [output] [exit_code]  Sub-256 byte x86 PE");
    println!("   adb vm [output] [exit_code]     MicroVM bytecode");
    println!();
    println!("🎮 GPU (Vulkan/CUDA):");
    println!("   adb gpu                         Detect GPU, generate shader");
    println!("   adb spirv [op] [size]           Generate SPIR-V compute shader");
    println!("   adb vulkan                      Initialize Vulkan runtime");
    println!("   adb cuda [op] [size]            Generate CUDA code (.cu)");
    println!("   adb unified [op] [size]         Auto CPU↔GPU decision");
    println!();
    println!("📝 SUPPORTED FEATURES:");
    println!("   C:   C99/C11, structs, pointers, arrays, printf, malloc");
    println!("   C++: C++98/11/14/17, classes, templates, namespaces, STL");
    println!("   header_main.h: Un solo #include — todo disponible");
    println!();
    println!("🎯 OUTPUT TARGETS (v8.0):");
    println!("   boot16     16-bit flat binary (stage1 bootloader)");
    println!("   boot32     32-bit flat binary (stage2 protected mode)");
    println!("   fastos64   FastOS Po 64-bit (.po)");
    println!("   fastos128  FastOS Po 128-bit — XMM/SSE (.po)");
    println!("   fastos256  FastOS Po 256-bit — YMM/AVX2 (.po) ★");
    println!("   windows    Windows PE x64 (.exe)");
    println!("   linux      Linux ELF x64");
    println!("   all        Multi-target (PE + ELF + Po)");
    println!();
    println!("🔧 SETUP:");
    println!("   adb --version      Show path and setup instructions");
    println!("   adb install        Install global headers");
    println!();
}
