use super::PyToIR;
use crate::frontend::python::ast::*;
use crate::middle::ir::*;
use super::{IRProgram, IRGlobal, IRConstant, fresh_temp};

impl PyToIR {
    pub fn convert_expr_to_instr(&mut self, expr: &PyExpr, program: &mut IRProgram) -> IRInstruction {
        match expr {
            PyExpr::IntLiteral(n) => IRInstruction::LoadConst(IRConstValue::Int(*n)),
            PyExpr::FloatLiteral(f) => IRInstruction::LoadConst(IRConstValue::Float(*f)),
            PyExpr::BoolLiteral(b) => IRInstruction::LoadConst(IRConstValue::Bool(*b)),
            PyExpr::NoneLiteral => IRInstruction::LoadConst(IRConstValue::None),
            PyExpr::StringLiteral(s) => {
                let label = self.add_string(s, program);
                IRInstruction::LoadString(label)
            }
            PyExpr::Name(name) => {
                if self.global_vars.contains(name) || self.all_globals.contains(name) {
                    IRInstruction::GlobalLoad(name.clone())
                } else {
                    IRInstruction::Load(name.clone())
                }
            }
            PyExpr::BinOp { op, left, right } => {
                // v4.5: str + str → __pyb_str_concat
                let left_is_str = self.is_str_expr(left);
                let right_is_str = self.is_str_expr(right);
                if matches!(op, PyBinOp::Add) && (left_is_str || right_is_str) {
                    let l = self.convert_expr_to_instr(left, program);
                    let r = self.convert_expr_to_instr(right, program);
                    return IRInstruction::Call {
                        func: "__pyb_str_concat".to_string(),
                        args: vec![l, r],
                    };
                }
                // v4.5: str * int → __pyb_str_repeat
                if matches!(op, PyBinOp::Mul) && left_is_str {
                    let l = self.convert_expr_to_instr(left, program);
                    let r = self.convert_expr_to_instr(right, program);
                    return IRInstruction::Call {
                        func: "__pyb_str_repeat".to_string(),
                        args: vec![l, r],
                    };
                }
                let l = self.convert_expr_to_instr(left, program);
                let r = self.convert_expr_to_instr(right, program);
                IRInstruction::BinOp {
                    op: self.convert_binop(op),
                    left: Box::new(l),
                    right: Box::new(r),
                }
            }
            PyExpr::Call { func: call_fn, args, .. } => {
                let func_name = match call_fn.as_ref() {
                    PyExpr::Name(n) => n.clone(),
                    PyExpr::Attribute { value, attr } => {
                        match value.as_ref() {
                            PyExpr::Name(obj) => format!("{}.{}", obj, attr),
                            PyExpr::Attribute { value: v2, attr: a2 } => {
                                if let PyExpr::Name(obj2) = v2.as_ref() {
                                    format!("{}.{}.{}", obj2, a2, attr)
                                } else {
                                    format!("?.{}.{}", a2, attr)
                                }
                            }
                            _ => "unknown".to_string(),
                        }
                    }
                    _ => "unknown".to_string(),
                };

                // ── math module functions ──────────────────────
                match func_name.as_str() {
                    "math.sqrt" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_sqrt".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.floor" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_floor".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.ceil" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_ceil".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.sin" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_sin".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.cos" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_cos".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.log" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_log".to_string(),
                            args: vec![arg],
                        };
                    }
                    "math.pow" if args.len() >= 2 => {
                        let base = self.convert_expr_to_instr(&args[0], program);
                        let exp = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::BinOp {
                            op: IROp::Pow,
                            left: Box::new(base),
                            right: Box::new(exp),
                        };
                    }
                    "math.abs" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_abs".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── os module ─────────────────────────────
                    "os.getcwd" => {
                        return IRInstruction::Call {
                            func: "__pyb_os_getcwd".to_string(),
                            args: vec![],
                        };
                    }
                    "os.path.exists" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_path_exists".to_string(),
                            args: vec![arg],
                        };
                    }
                    "os.getpid" => {
                        return IRInstruction::Call {
                            func: "__pyb_os_getpid".to_string(),
                            args: vec![],
                        };
                    }
                    "os.makedirs" | "os.mkdir" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_mkdir".to_string(),
                            args: vec![arg],
                        };
                    }
                    "os.remove" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_remove".to_string(),
                            args: vec![arg],
                        };
                    }
                    "os.rename" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_rename".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "os.environ.get" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_os_environ_get".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── sys module ────────────────────────────
                    "sys.exit" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_sys_exit".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── random module ─────────────────────────
                    "random.randint" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__pyb_random_randint".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "random.seed" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_random_seed".to_string(),
                            args: vec![arg],
                        };
                    }
                    "random.random" => {
                        return IRInstruction::Call {
                            func: "__pyb_random_next".to_string(),
                            args: vec![],
                        };
                    }
                    "random.choice" if !args.is_empty() => {
                        // choice(list) = list[randint(0, len-1)]
                        let lst = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_random_next".to_string(),
                            args: vec![lst],
                        };
                    }
                    // ── json module ───────────────────────────
                    "json.loads" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_json_loads".to_string(),
                            args: vec![arg],
                        };
                    }
                    "json.dumps" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_json_dumps".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── asyncio module ────────────────────────
                    "asyncio.run" if !args.is_empty() => {
                        // asyncio.run(coro()) → just call the coroutine directly
                        let coro = self.convert_expr_to_instr(&args[0], program);
                        return coro;
                    }
                    // ── numpy module ──────────────────────────
                    "np.array" | "numpy.array" if !args.is_empty() => {
                        // np.array([...]) → create list from elements
                        if let PyExpr::List(elts) = &args[0] {
                            // Build a list with the elements
                            return IRInstruction::Call {
                                func: "__pyb_list_new".to_string(),
                                args: vec![],
                            };
                        }
                        return IRInstruction::Call {
                            func: "__pyb_list_new".to_string(),
                            args: vec![],
                        };
                    }
                    "np.sum" | "numpy.sum" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_sum".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.max" | "numpy.max" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_max".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.min" | "numpy.min" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_min".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.dot" | "numpy.dot" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_dot".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "np.sqrt" | "numpy.sqrt" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__math_sqrt".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.zeros" | "numpy.zeros" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_listcomp_range".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.ones" | "numpy.ones" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_listcomp_range".to_string(),
                            args: vec![arg],
                        };
                    }
                    "np.mean" | "numpy.mean" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_sum".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── ctypes module ─────────────────────────
                    "ctypes.CDLL" if !args.is_empty() => {
                        // v4.0: Detect GPU DLLs → dispatch
                        if let PyExpr::StringLiteral(path_str) = &args[0] {
                            if path_str.contains("nvcuda") || path_str.contains("cuda") {
                                return IRInstruction::GpuInit;
                            }
                            if path_str.contains("vulkan") {
                                return IRInstruction::VkInit;
                            }
                        }
                        let path = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_dll_load".to_string(),
                            args: vec![path],
                        };
                    }
                    "vk.vkCreateInstance" | "vulkan.vkCreateInstance" => {
                        return IRInstruction::VkInit;
                    }
                    "vk.vkEnumeratePhysicalDevices" | "vulkan.vkEnumeratePhysicalDevices" => {
                        return IRInstruction::VkDeviceGet;
                    }
                    "vk.vkCreateDevice" | "vulkan.vkCreateDevice" => {
                        return IRInstruction::VkDeviceCreate;
                    }
                    "vk.vkCreateBuffer" | "vulkan.vkCreateBuffer" if !args.is_empty() => {
                        let size = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::VkBufferCreate { size: Box::new(size) };
                    }
                    "vk.vkMapMemory" | "vulkan.vkMapMemory" => {
                        let size = if args.len() >= 3 {
                            self.convert_expr_to_instr(&args[2], program)
                        } else {
                            IRInstruction::LoadConst(IRConstValue::Int(0))
                        };
                        return IRInstruction::VkBufferWrite {
                            dst: "vk_buf".to_string(),
                            src: "host_ptr".to_string(),
                            size: Box::new(size),
                        };
                    }
                    "vk.vkCreateShaderModule" | "vulkan.vkCreateShaderModule" => {
                        let path = if !args.is_empty() {
                            if let PyExpr::StringLiteral(s) = &args[0] { s.clone() } else { "shader.spv".to_string() }
                        } else { "shader.spv".to_string() };
                        return IRInstruction::VkShaderLoad { spirv_path: path };
                    }
                    "vk.vkCmdDispatch" | "vulkan.vkCmdDispatch" => {
                        let x = if args.len() > 0 { self.convert_expr_to_instr(&args[0], program) } else { IRInstruction::LoadConst(IRConstValue::Int(1)) };
                        let y = if args.len() > 1 { self.convert_expr_to_instr(&args[1], program) } else { IRInstruction::LoadConst(IRConstValue::Int(1)) };
                        let z = if args.len() > 2 { self.convert_expr_to_instr(&args[2], program) } else { IRInstruction::LoadConst(IRConstValue::Int(1)) };
                        return IRInstruction::VkDispatch {
                            shader: "compute".to_string(),
                            x: Box::new(x), y: Box::new(y), z: Box::new(z),
                        };
                    }
                    "vk.vkFreeMemory" | "vulkan.vkFreeMemory" => {
                        return IRInstruction::VkBufferFree { ptr: "vk_buf".to_string() };
                    }
                    "vk.vkDestroyDevice" | "vulkan.vkDestroyDevice" |
                    "vk.vkDestroyInstance" | "vulkan.vkDestroyInstance" => {
                        return IRInstruction::VkDestroy;
                    }
                    // v4.0 FASE 4: GPU dispatch functions
                    "cuda.cuInit" | "cu.cuInit" => {
                        return IRInstruction::GpuInit;
                    }
                    "cuda.cuDeviceGet" | "cu.cuDeviceGet" => {
                        return IRInstruction::GpuDeviceGet;
                    }
                    "cuda.cuCtxCreate" | "cu.cuCtxCreate" => {
                        return IRInstruction::GpuCtxCreate;
                    }
                    "cuda.cuMemAlloc" | "cu.cuMemAlloc" if !args.is_empty() => {
                        let size = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::GpuMalloc { size: Box::new(size) };
                    }
                    "cuda.cuMemcpyHtoD" | "cu.cuMemcpyHtoD" => {
                        let size = if args.len() >= 3 {
                            self.convert_expr_to_instr(&args[2], program)
                        } else {
                            IRInstruction::LoadConst(IRConstValue::Int(0))
                        };
                        return IRInstruction::GpuMemcpyHtoD {
                            dst: "gpu_ptr".to_string(),
                            src: "host_ptr".to_string(),
                            size: Box::new(size),
                        };
                    }
                    "cuda.cuMemcpyDtoH" | "cu.cuMemcpyDtoH" => {
                        let size = if args.len() >= 3 {
                            self.convert_expr_to_instr(&args[2], program)
                        } else {
                            IRInstruction::LoadConst(IRConstValue::Int(0))
                        };
                        return IRInstruction::GpuMemcpyDtoH {
                            dst: "host_ptr".to_string(),
                            src: "gpu_ptr".to_string(),
                            size: Box::new(size),
                        };
                    }
                    "cuda.cuLaunchKernel" | "cu.cuLaunchKernel" => {
                        let ir_args: Vec<IRInstruction> = args.iter()
                            .map(|a| self.convert_expr_to_instr(a, program))
                            .collect();
                        return IRInstruction::GpuLaunch {
                            kernel: "cuda_kernel".to_string(),
                            args: ir_args,
                        };
                    }
                    "cuda.cuMemFree" | "cu.cuMemFree" => {
                        return IRInstruction::GpuFree { ptr: "gpu_ptr".to_string() };
                    }
                    "cuda.cuCtxDestroy" | "cu.cuCtxDestroy" => {
                        return IRInstruction::GpuCtxDestroy;
                    }
                    "ctypes.c_int" if !args.is_empty() => {
                        // ctypes.c_int(42) → just return the int
                        return self.convert_expr_to_instr(&args[0], program);
                    }
                    "ctypes.c_double" if !args.is_empty() => {
                        return self.convert_expr_to_instr(&args[0], program);
                    }
                    // ── functools ─────────────────────────────
                    "functools.lru_cache" => {
                        // @lru_cache decorator — passthrough, handled at class level
                        return IRInstruction::Nop;
                    }
                    // ── sum() builtin ─────────────────────────
                    "sum" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_np_sum".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── next() builtin ────────────────────────
                    "next" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_gen_next".to_string(),
                            args: vec![arg],
                        };
                    }
                    // ── open() builtin ────────────────────────
                    "open" if !args.is_empty() => {
                        let path_arg = self.convert_expr_to_instr(&args[0], program);
                        let mode_val = if args.len() >= 2 {
                            // "r" → 0, "w" → 1
                            if let PyExpr::StringLiteral(m) = &args[1] {
                                if m == "w" { 1i64 } else { 0i64 }
                            } else { 0i64 }
                        } else { 0i64 };
                        return IRInstruction::Call {
                            func: "__pyb_file_open".to_string(),
                            args: vec![
                                path_arg,
                                IRInstruction::LoadConst(IRConstValue::Int(mode_val)),
                            ],
                        };
                    }
                    // ── abs, min, max, chr, ord ────────────────
                    "abs" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__builtin_abs".to_string(),
                            args: vec![arg],
                        };
                    }
                    "min" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__builtin_min".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "max" if args.len() >= 2 => {
                        let a = self.convert_expr_to_instr(&args[0], program);
                        let b = self.convert_expr_to_instr(&args[1], program);
                        return IRInstruction::Call {
                            func: "__builtin_max".to_string(),
                            args: vec![a, b],
                        };
                    }
                    "chr" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__builtin_chr".to_string(),
                            args: vec![arg],
                        };
                    }
                    "ord" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__builtin_ord".to_string(),
                            args: vec![arg],
                        };
                    }
                    "len" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        // Type-aware len dispatch
                        let len_func = if let PyExpr::Name(n) = &args[0] {
                            if self.dict_vars.contains(n) {
                                "__pyb_dict_len_builtin"
                            } else if self.str_heap_vars.contains(n) || self.string_vars.contains_key(n) {
                                "__pyb_str_len_builtin"
                            } else {
                                "__builtin_len"
                            }
                        } else if matches!(&args[0], PyExpr::StringLiteral(_)) {
                            "__pyb_str_len_builtin"
                        } else {
                            "__builtin_len"
                        };
                        return IRInstruction::Call {
                            func: len_func.to_string(),
                            args: vec![arg],
                        };
                    }
                    // v4.5 — str() builtin: int→str conversion
                    "str" if !args.is_empty() => {
                        let arg = self.convert_expr_to_instr(&args[0], program);
                        return IRInstruction::Call {
                            func: "__pyb_int_to_str".to_string(),
                            args: vec![arg],
                        };
                    }
                    _ => {
                        // Check if it's a constructor call: ClassName(args...)
                        if self.class_names.contains(&func_name) {
                            let num_fields = self.class_fields.get(&func_name)
                                .map(|f| f.len()).unwrap_or(0);
                            let alloc_size = (num_fields + 1) * 8; // +1 for class_id
                            let init_name = format!("{}____init__", func_name);
                            // Build args for __init__: first arg will be the new obj ptr (placeholder)
                            let mut init_args = vec![
                                IRInstruction::LoadConst(IRConstValue::Int(alloc_size as i64)),
                            ];
                            for a in args {
                                init_args.push(self.convert_expr_to_instr(a, program));
                            }
                            return IRInstruction::Call {
                                func: format!("__pyb_obj_new::{}", init_name),
                                args: init_args,
                            };
                        }
                        // Check obj.method() calls
                        if func_name.contains('.') {
                            let parts: Vec<&str> = func_name.splitn(2, '.').collect();
                            if parts.len() == 2 {
                                let obj_name = parts[0];
                                let method = parts[1];

                                // File method calls: f.read(), f.write(), f.close()
                                if self.file_vars.contains(obj_name) {
                                    match method {
                                        "read" => {
                                            return IRInstruction::Call {
                                                func: "__pyb_file_read".to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string())],
                                            };
                                        }
                                        "write" if !args.is_empty() => {
                                            let arg = self.convert_expr_to_instr(&args[0], program);
                                            let str_label = if let PyExpr::StringLiteral(s) = &args[0] {
                                                Some(self.add_string(s, program))
                                            } else { None };
                                            if let Some(label) = str_label {
                                                return IRInstruction::Call {
                                                    func: "__pyb_file_write".to_string(),
                                                    args: vec![
                                                        IRInstruction::Load(obj_name.to_string()),
                                                        IRInstruction::LoadString(label.clone()),
                                                        IRInstruction::LoadConst(IRConstValue::Int(
                                                            if let PyExpr::StringLiteral(s) = &args[0] { s.len() as i64 } else { 0 }
                                                        )),
                                                    ],
                                                };
                                            }
                                            return IRInstruction::Call {
                                                func: "__pyb_file_write".to_string(),
                                                args: vec![
                                                    IRInstruction::Load(obj_name.to_string()),
                                                    arg,
                                                    IRInstruction::LoadConst(IRConstValue::Int(0)),
                                                ],
                                            };
                                        }
                                        "close" => {
                                            return IRInstruction::Call {
                                                func: "__pyb_file_close".to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string())],
                                            };
                                        }
                                        _ => {}
                                    }
                                }

                                // String method calls: s.upper(), s.lower(), s.find(), s.replace()
                                if self.str_heap_vars.contains(obj_name) || self.string_vars.contains_key(obj_name) {
                                    match method {
                                        "upper" => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            return IRInstruction::Call {
                                                func: "__pyb_str_upper".to_string(),
                                                args: vec![src],
                                            };
                                        }
                                        "lower" => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            return IRInstruction::Call {
                                                func: "__pyb_str_lower".to_string(),
                                                args: vec![src],
                                            };
                                        }
                                        "find" if !args.is_empty() => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let needle = self.convert_expr_to_instr(&args[0], program);
                                            return IRInstruction::Call {
                                                func: "__pyb_str_find".to_string(),
                                                args: vec![src, needle],
                                            };
                                        }
                                        "replace" if args.len() >= 2 => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let old = self.convert_expr_to_instr(&args[0], program);
                                            let new = self.convert_expr_to_instr(&args[1], program);
                                            return IRInstruction::Call {
                                                func: "__pyb_str_replace".to_string(),
                                                args: vec![src, old, new],
                                            };
                                        }
                                        "startswith" if !args.is_empty() => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let needle = self.convert_expr_to_instr(&args[0], program);
                                            return IRInstruction::Call {
                                                func: "__pyb_str_startswith".to_string(),
                                                args: vec![src, needle],
                                            };
                                        }
                                        "endswith" if !args.is_empty() => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let needle = self.convert_expr_to_instr(&args[0], program);
                                            return IRInstruction::Call {
                                                func: "__pyb_str_endswith".to_string(),
                                                args: vec![src, needle],
                                            };
                                        }
                                        // v4.5 — new str methods
                                        "strip" => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            return IRInstruction::Call {
                                                func: "__pyb_str_strip".to_string(),
                                                args: vec![src],
                                            };
                                        }
                                        "split" if !args.is_empty() => {
                                            let src = if let Some(label) = self.string_vars.get(obj_name) {
                                                IRInstruction::LoadString(label.clone())
                                            } else {
                                                IRInstruction::Load(obj_name.to_string())
                                            };
                                            let sep = self.convert_expr_to_instr(&args[0], program);
                                            return IRInstruction::Call {
                                                func: "__pyb_str_split".to_string(),
                                                args: vec![src, sep],
                                            };
                                        }
                                        _ => {}
                                    }
                                }

                                // v4.5 — List method calls: l.sort(), l.reverse(), l.pop(), l.append(), l.contains()
                                if self.dict_vars.contains(obj_name) {
                                    // Dict method calls
                                    match method {
                                        "get" if !args.is_empty() => {
                                            let key = self.convert_expr_to_instr(&args[0], program);
                                            let get_func = if self.is_str_expr(&args[0]) { "__pyb_dict_str_get" } else { "__pyb_dict_get" };
                                            if args.len() >= 2 {
                                                // d.get(key, default) — for now just use dict_get (returns 0 if not found)
                                                return IRInstruction::Call {
                                                    func: get_func.to_string(),
                                                    args: vec![IRInstruction::Load(obj_name.to_string()), key],
                                                };
                                            }
                                            return IRInstruction::Call {
                                                func: get_func.to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string()), key],
                                            };
                                        }
                                        "keys" | "values" | "items" => {
                                            // Return dict ptr for iteration
                                            return IRInstruction::Load(obj_name.to_string());
                                        }
                                        _ => {}
                                    }
                                } else if !self.file_vars.contains(obj_name) && !self.class_vars.contains_key(obj_name)
                                    && !self.str_heap_vars.contains(obj_name) && !self.string_vars.contains_key(obj_name) {
                                    // Assume it's a list variable for list methods
                                    match method {
                                        "append" if !args.is_empty() => {
                                            let val = self.convert_expr_to_instr(&args[0], program);
                                            return IRInstruction::Call {
                                                func: "__pyb_list_append".to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string()), val],
                                            };
                                        }
                                        "sort" => {
                                            return IRInstruction::Call {
                                                func: "__pyb_list_sort".to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string())],
                                            };
                                        }
                                        "reverse" => {
                                            return IRInstruction::Call {
                                                func: "__pyb_list_reverse".to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string())],
                                            };
                                        }
                                        "pop" => {
                                            return IRInstruction::Call {
                                                func: "__pyb_list_pop".to_string(),
                                                args: vec![IRInstruction::Load(obj_name.to_string())],
                                            };
                                        }
                                        _ => {}
                                    }
                                }


                                // Class instance method calls
                                if let Some(cls) = self.class_vars.get(obj_name) {
                                    let cls = cls.clone();
                                    let full_method = format!("{}__{}", cls, method);
                                    // Use GlobalLoad if obj_name is a global variable
                                    let obj_instr = if self.all_globals.contains(obj_name) {
                                        IRInstruction::GlobalLoad(obj_name.to_string())
                                    } else {
                                        IRInstruction::Load(obj_name.to_string())
                                    };
                                    let mut call_args = vec![obj_instr];
                                    for a in args {
                                        call_args.push(self.convert_expr_to_instr(a, program));
                                    }
                                    return IRInstruction::Call {
                                        func: full_method,
                                        args: call_args,
                                    };
                                }
                            }
                        }
                    }
                }

                let ir_args: Vec<IRInstruction> = args.iter()
                    .map(|a| self.convert_expr_to_instr(a, program))
                    .collect();
                IRInstruction::Call {
                    func: func_name,
                    args: ir_args,
                }
            }
            PyExpr::Compare { left, ops, comparators } => {
                // v4.5: `x in list` → __pyb_list_contains, `x in str` → __pyb_str_contains
                if let (Some(op), Some(right)) = (ops.first(), comparators.first()) {
                    if matches!(op, PyCmpOp::In | PyCmpOp::NotIn) {
                        let container = self.convert_expr_to_instr(right, program);
                        let needle = self.convert_expr_to_instr(left, program);
                        let container_is_str = self.is_str_expr(right) || if let PyExpr::Name(n) = right {
                            self.string_vars.contains_key(n) || self.str_heap_vars.contains(n)
                        } else { false };
                        let container_is_dict = if let PyExpr::Name(n) = right {
                            self.dict_vars.contains(n)
                        } else { false };
                        let stub = if container_is_str {
                            "__pyb_str_contains"
                        } else if container_is_dict {
                            "__pyb_dict_contains"
                        } else {
                            "__pyb_list_contains"
                        };
                        let result = IRInstruction::Call {
                            func: stub.to_string(),
                            args: vec![container, needle],
                        };
                        if matches!(op, PyCmpOp::NotIn) {
                            // NOT: XOR result with 1
                            return IRInstruction::BinOp {
                                op: IROp::Xor,
                                left: Box::new(result),
                                right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(1))),
                            };
                        }
                        return result;
                    }
                    let l = self.convert_expr_to_instr(left, program);
                    let r = self.convert_expr_to_instr(right, program);
                    IRInstruction::Compare {
                        op: self.convert_cmpop(op),
                        left: Box::new(l),
                        right: Box::new(r),
                    }
                } else {
                    self.convert_expr_to_instr(left, program)
                }
            }
            PyExpr::UnaryOp { op, operand } => {
                match op {
                    PyUnaryOp::Neg => {
                        // -x → 0 - x
                        if let PyExpr::IntLiteral(n) = operand.as_ref() {
                            return IRInstruction::LoadConst(IRConstValue::Int(-n));
                        }
                        if let PyExpr::FloatLiteral(f) = operand.as_ref() {
                            return IRInstruction::LoadConst(IRConstValue::Float(-f));
                        }
                        let inner = self.convert_expr_to_instr(operand, program);
                        IRInstruction::BinOp {
                            op: IROp::Sub,
                            left: Box::new(IRInstruction::LoadConst(IRConstValue::Int(0))),
                            right: Box::new(inner),
                        }
                    }
                    PyUnaryOp::Pos => self.convert_expr_to_instr(operand, program),
                    PyUnaryOp::Invert => {
                        let inner = self.convert_expr_to_instr(operand, program);
                        IRInstruction::BinOp {
                            op: IROp::Xor,
                            left: Box::new(inner),
                            right: Box::new(IRInstruction::LoadConst(IRConstValue::Int(-1))),
                        }
                    }
                    _ => self.convert_expr_to_instr(operand, program),
                }
            }
            PyExpr::Subscript { value, slice } => {
                let obj_instr = self.convert_expr_to_instr(value, program);
                let idx_instr = self.convert_expr_to_instr(slice, program);
                // Check if this is a dict or list subscript
                let is_dict = if let PyExpr::Name(n) = value.as_ref() {
                    self.dict_vars.contains(n)
                } else { false };
                if is_dict {
                    let get_func = if self.is_str_expr(slice) { "__pyb_dict_str_get" } else { "__pyb_dict_get" };
                    IRInstruction::Call {
                        func: get_func.to_string(),
                        args: vec![obj_instr, idx_instr],
                    }
                } else {
                    IRInstruction::Call {
                        func: "__pyb_list_get".to_string(),
                        args: vec![obj_instr, idx_instr],
                    }
                }
            }
            PyExpr::Attribute { value, attr } => {
                if let PyExpr::Name(obj) = value.as_ref() {
                    match (obj.as_str(), attr.as_str()) {
                        ("math", "pi") => IRInstruction::LoadConst(IRConstValue::Float(std::f64::consts::PI)),
                        ("math", "e") => IRInstruction::LoadConst(IRConstValue::Float(std::f64::consts::E)),
                        ("math", "inf") => IRInstruction::LoadConst(IRConstValue::Float(f64::INFINITY)),
                        ("math", "tau") => IRInstruction::LoadConst(IRConstValue::Float(std::f64::consts::TAU)),
                        ("sys", "platform") => {
                            let label = self.add_string(if cfg!(target_os = "windows") { "win32" } else { "linux" }, program);
                            return IRInstruction::LoadString(label);
                        }
                        ("sys", "version") => {
                            let label = self.add_string("PyDead-BIB 2.0.0", program);
                            return IRInstruction::LoadString(label);
                        }
                        ("sys", "maxsize") => IRInstruction::LoadConst(IRConstValue::Int(i64::MAX)),
                        _ => {
                            // Check if obj is a class instance → read field
                            if let Some(cls) = self.class_vars.get(obj) {
                                let cls = cls.clone();
                                let offset = if let Some(fields) = self.class_fields.get(&cls) {
                                    fields.iter().position(|f| f == attr).unwrap_or(0)
                                } else { 0 };
                                let byte_offset = (offset as i64 + 1) * 8;
                                IRInstruction::Call {
                                    func: "__pyb_obj_get_field".to_string(),
                                    args: vec![
                                        IRInstruction::Load(obj.clone()),
                                        IRInstruction::LoadConst(IRConstValue::Int(byte_offset)),
                                    ],
                                }
                            } else if obj == "self" {
                                // self.x inside a method — need class context
                                // We'll use a convention: look through all classes for this field
                                let mut byte_offset = 8i64; // default: first field
                                for (cls_name, fields) in &self.class_fields {
                                    if let Some(pos) = fields.iter().position(|f| f == attr) {
                                        byte_offset = (pos as i64 + 1) * 8;
                                        break;
                                    }
                                }
                                IRInstruction::Call {
                                    func: "__pyb_obj_get_field".to_string(),
                                    args: vec![
                                        IRInstruction::Load("self".to_string()),
                                        IRInstruction::LoadConst(IRConstValue::Int(byte_offset)),
                                    ],
                                }
                            } else {
                                IRInstruction::Nop
                            }
                        }
                    }
                } else {
                    IRInstruction::Nop
                }
            }
            // ── List comprehension ─────────────────────────────
            PyExpr::ListComp { element, generators } => {
                // [expr for var in range(n)] → create list, loop, append
                // For simple cases: [x**2 for x in range(n)]
                if let Some(gen) = generators.first() {
                    if let PyExpr::Call { func: iter_fn, args: iter_args, .. } = &gen.iter {
                        if let PyExpr::Name(fn_name) = iter_fn.as_ref() {
                            if fn_name == "range" && !iter_args.is_empty() {
                                // Compile as: list_new, for i in range(n): list_append(list, element(i))
                                // Return the call to build the list
                                let stop = self.convert_expr_to_instr(&iter_args[iter_args.len().min(2) - 1], program);
                                return IRInstruction::Call {
                                    func: "__pyb_listcomp_range".to_string(),
                                    args: vec![stop],
                                };
                            }
                        }
                    }
                }
                // Fallback: just create empty list
                IRInstruction::Call {
                    func: "__pyb_list_new".to_string(),
                    args: vec![],
                }
            }
            // ── Await expression ──────────────────────────────
            PyExpr::Await(inner) => {
                // For now, await just evaluates the inner expression
                self.convert_expr_to_instr(inner, program)
            }
            // ── Yield expression ──────────────────────────────
            PyExpr::Yield(val) => {
                if let Some(v) = val {
                    self.convert_expr_to_instr(v, program)
                } else {
                    IRInstruction::LoadConst(IRConstValue::None)
                }
            }
            // ── Conditional expression (ternary) ──────────────
            PyExpr::IfExpr { test, body, orelse } => {
                let cond = self.convert_expr_to_instr(test, program);
                let then_val = self.convert_expr_to_instr(body, program);
                let else_val = self.convert_expr_to_instr(orelse, program);
                // For now, compile as: test ? then : else using nested Call
                // Simple approach: evaluate test, if true return body, else orelse
                // This is simplified — full ternary needs conditional jump in ISA
                then_val
            }
            _ => IRInstruction::Nop,
        }
    }

    pub fn extract_int_literal(&self, expr: &PyExpr) -> Option<i64> {
        match expr {
            PyExpr::IntLiteral(n) => Some(*n),
            PyExpr::UnaryOp { op: PyUnaryOp::Neg, operand } => {
                if let PyExpr::IntLiteral(n) = operand.as_ref() {
                    Some(-n)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn is_float_expr(&self, expr: &PyExpr) -> bool {
        match expr {
            PyExpr::FloatLiteral(_) => true,
            PyExpr::Attribute { value, attr } => {
                if let PyExpr::Name(obj) = value.as_ref() {
                    obj == "math" && matches!(attr.as_str(), "pi" | "e" | "inf" | "tau")
                } else { false }
            }
            PyExpr::Call { func, .. } => {
                if let PyExpr::Attribute { value, attr } = func.as_ref() {
                    if let PyExpr::Name(obj) = value.as_ref() {
                        return obj == "math" && matches!(attr.as_str(),
                            "sqrt" | "sin" | "cos" | "log" | "abs" | "pow"
                        );
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub fn is_str_expr(&self, expr: &PyExpr) -> bool {
        match expr {
            PyExpr::StringLiteral(_) | PyExpr::FString { .. } => true,
            PyExpr::Name(n) => {
                self.str_heap_vars.contains(n) || self.string_vars.contains_key(n)
            }
            PyExpr::Call { func, .. } => {
                if let PyExpr::Name(n) = func.as_ref() {
                    return matches!(n.as_str(), "str");
                }
                if let PyExpr::Attribute { attr, .. } = func.as_ref() {
                    return matches!(attr.as_str(), "upper" | "lower" | "strip" | "replace"
                        | "join" | "lstrip" | "rstrip" | "format" | "read" | "startswith" | "endswith");
                }
                false
            }
            PyExpr::BinOp { op: PyBinOp::Add, left, right } => {
                self.is_str_expr(left) || self.is_str_expr(right)
            }
            _ => false,
        }
    }

    pub fn infer_expr_type(&self, expr: &PyExpr) -> IRType {
        match expr {
            PyExpr::IntLiteral(_) => IRType::I64,
            PyExpr::FloatLiteral(_) => IRType::F64,
            PyExpr::BoolLiteral(_) => IRType::I8,
            PyExpr::StringLiteral(_) | PyExpr::FString { .. } => IRType::Ptr,
            PyExpr::NoneLiteral => IRType::Void,
            PyExpr::List(_) => IRType::Ptr,
            PyExpr::Dict { .. } => IRType::Ptr,
            _ => IRType::I64,
        }
    }

    pub fn expr_to_constant(&self, expr: &PyExpr) -> Option<IRConstant> {
        match expr {
            PyExpr::IntLiteral(n) => Some(IRConstant::Int(*n)),
            PyExpr::FloatLiteral(f) => Some(IRConstant::Float(*f)),
            PyExpr::BoolLiteral(b) => Some(IRConstant::Bool(*b)),
            PyExpr::StringLiteral(s) => Some(IRConstant::Str(s.clone())),
            PyExpr::NoneLiteral => Some(IRConstant::None),
            _ => std::option::Option::None,
        }
    }

    pub fn convert_binop(&self, op: &PyBinOp) -> IROp {
        match op {
            PyBinOp::Add => IROp::Add,
            PyBinOp::Sub => IROp::Sub,
            PyBinOp::Mul => IROp::Mul,
            PyBinOp::Div => IROp::Div,
            PyBinOp::FloorDiv => IROp::FloorDiv,
            PyBinOp::Mod => IROp::Mod,
            PyBinOp::Pow => IROp::Pow,
            PyBinOp::LShift => IROp::Shl,
            PyBinOp::RShift => IROp::Shr,
            PyBinOp::BitOr => IROp::Or,
            PyBinOp::BitXor => IROp::Xor,
            PyBinOp::BitAnd => IROp::And,
            PyBinOp::MatMul => IROp::MatMul,
        }
    }

    pub fn convert_cmpop(&self, op: &PyCmpOp) -> IRCmpOp {
        match op {
            PyCmpOp::Eq => IRCmpOp::Eq,
            PyCmpOp::NotEq => IRCmpOp::Ne,
            PyCmpOp::Lt => IRCmpOp::Lt,
            PyCmpOp::LtE => IRCmpOp::Le,
            PyCmpOp::Gt => IRCmpOp::Gt,
            PyCmpOp::GtE => IRCmpOp::Ge,
            PyCmpOp::Is => IRCmpOp::Eq,
            PyCmpOp::IsNot => IRCmpOp::Ne,
            PyCmpOp::In => IRCmpOp::In,
            PyCmpOp::NotIn => IRCmpOp::NotIn,
        }
    }
}
