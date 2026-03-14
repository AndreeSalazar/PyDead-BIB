// ============================================================
// PyDead-BIB ISA Compiler — Heredado de ADead-BIB v8.0
// ============================================================
// IR → x86-64 machine code bytes
// Direct encoding — sin assembler externo — sin NASM
// Soporta: GP regs, XMM, YMM, VEX prefix
// Windows x64 ABI: shadow space, RCX/RDX/R8/R9
// ============================================================

use crate::middle::ir::{IRConstValue, IRCmpOp, IRInstruction, IROp};
use crate::backend::reg_alloc::{AllocatedFunction, AllocatedProgram, X86Reg};

// ── Compilation target ────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Windows,
    Linux,
    FastOS64,
    FastOS128,
    FastOS256,
}

impl Target {
    pub fn from_str(s: &str) -> Self {
        match s {
            "windows" | "win64" | "pe" => Target::Windows,
            "linux" | "elf" => Target::Linux,
            "fastos64" => Target::FastOS64,
            "fastos128" => Target::FastOS128,
            "fastos256" => Target::FastOS256,
            _ => Target::Windows,
        }
    }
}

// ── Compiled code section ─────────────────────────────────────
pub struct CompiledProgram {
    pub text: Vec<u8>,           // .text section (machine code)
    pub data: Vec<u8>,           // .data section (string literals, constants)
    pub data_labels: Vec<(String, u32)>, // label → offset in .data
    pub functions: Vec<CompiledFunction>,
    pub entry_point: u32,        // offset of _start / main
    pub target: Target,
    pub stats: ISAStats,
}

pub struct CompiledFunction {
    pub name: String,
    pub offset: u32,             // offset in .text
    pub size: u32,               // bytes
}

#[derive(Debug, Default)]
pub struct ISAStats {
    pub total_bytes: usize,
    pub functions_compiled: usize,
    pub instructions_emitted: usize,
}

// ── x86-64 Encoder ────────────────────────────────────────────
struct Encoder {
    code: Vec<u8>,
    data: Vec<u8>,
    data_labels: Vec<(String, u32)>,
    label_offsets: Vec<(String, u32)>,    // label → code offset
    fixups: Vec<(usize, String)>,         // (code offset, target label) for jumps
    stats: ISAStats,
}

impl Encoder {
    fn new() -> Self {
        Self {
            code: Vec::new(),
            data: Vec::new(),
            data_labels: Vec::new(),
            label_offsets: Vec::new(),
            fixups: Vec::new(),
            stats: ISAStats::default(),
        }
    }

    fn current_offset(&self) -> u32 {
        self.code.len() as u32
    }

    // ── Emit raw bytes ────────────────────────────────────
    fn emit(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
        self.stats.instructions_emitted += 1;
    }

    // ── REX prefix for 64-bit operand size ────────────────
    fn rex_w(&mut self) {
        self.emit(&[0x48]);
    }

    fn rex_wr(&mut self, reg: X86Reg) {
        let r = if reg.needs_rex() { 0x44 } else { 0x00 };
        self.emit(&[0x48 | r]);
    }

    // ── MOV imm64 → reg ──────────────────────────────────
    fn mov_imm64(&mut self, reg: X86Reg, val: i64) {
        self.rex_wr(reg);
        self.emit(&[0xB8 + (reg.encoding() & 0x07)]);
        self.emit(&val.to_le_bytes());
    }

    // ── MOV reg → reg ─────────────────────────────────────
    fn mov_reg_reg(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_w();
        self.emit(&[0x89, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    // ── ADD reg, reg ──────────────────────────────────────
    fn add_reg_reg(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_w();
        self.emit(&[0x01, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    // ── SUB reg, reg ──────────────────────────────────────
    fn sub_reg_reg(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_w();
        self.emit(&[0x29, 0xC0 | ((src.encoding() & 7) << 3) | (dst.encoding() & 7)]);
    }

    // ── IMUL reg, reg ─────────────────────────────────────
    fn imul_reg_reg(&mut self, dst: X86Reg, src: X86Reg) {
        self.rex_w();
        self.emit(&[0x0F, 0xAF, 0xC0 | ((dst.encoding() & 7) << 3) | (src.encoding() & 7)]);
    }

    // ── CQO + IDIV reg ───────────────────────────────────
    fn idiv_reg(&mut self, src: X86Reg) {
        self.rex_w();
        self.emit(&[0x99]); // CQO
        self.rex_w();
        self.emit(&[0xF7, 0xF8 | (src.encoding() & 7)]);
    }

    // ── CMP reg, reg ──────────────────────────────────────
    fn cmp_reg_reg(&mut self, a: X86Reg, b: X86Reg) {
        self.rex_w();
        self.emit(&[0x39, 0xC0 | ((b.encoding() & 7) << 3) | (a.encoding() & 7)]);
    }

    // ── Jcc (conditional jump) — 32-bit relative ──────────
    fn jcc(&mut self, condition: u8, label: &str) {
        self.emit(&[0x0F, condition]);
        self.fixups.push((self.code.len(), label.to_string()));
        self.emit(&[0x00, 0x00, 0x00, 0x00]); // placeholder
    }

    // ── JMP rel32 ─────────────────────────────────────────
    fn jmp(&mut self, label: &str) {
        self.emit(&[0xE9]);
        self.fixups.push((self.code.len(), label.to_string()));
        self.emit(&[0x00, 0x00, 0x00, 0x00]);
    }

    // ── CALL rel32 ────────────────────────────────────────
    fn call_label(&mut self, label: &str) {
        self.emit(&[0xE8]);
        self.fixups.push((self.code.len(), label.to_string()));
        self.emit(&[0x00, 0x00, 0x00, 0x00]);
    }

    // ── PUSH / POP ────────────────────────────────────────
    fn push_reg(&mut self, reg: X86Reg) {
        if reg.needs_rex() {
            self.emit(&[0x41]);
        }
        self.emit(&[0x50 + (reg.encoding() & 7)]);
    }

    fn pop_reg(&mut self, reg: X86Reg) {
        if reg.needs_rex() {
            self.emit(&[0x41]);
        }
        self.emit(&[0x58 + (reg.encoding() & 7)]);
    }

    // ── SUB RSP, imm8 ────────────────────────────────────
    fn sub_rsp_imm(&mut self, val: u8) {
        self.rex_w();
        self.emit(&[0x83, 0xEC, val]);
    }

    // ── ADD RSP, imm8 ────────────────────────────────────
    fn add_rsp_imm(&mut self, val: u8) {
        self.rex_w();
        self.emit(&[0x83, 0xC4, val]);
    }

    // ── RET ───────────────────────────────────────────────
    fn ret(&mut self) {
        self.emit(&[0xC3]);
    }

    // ── XOR reg, reg (zero) ───────────────────────────────
    fn xor_reg_reg(&mut self, reg: X86Reg) {
        self.rex_w();
        let r = reg.encoding() & 7;
        self.emit(&[0x31, 0xC0 | (r << 3) | r]);
    }

    // ── Define label at current offset ────────────────────
    fn define_label(&mut self, name: &str) {
        self.label_offsets.push((name.to_string(), self.current_offset()));
    }

    // ── Add string to .data ───────────────────────────────
    fn add_data_string(&mut self, label: &str, s: &str) {
        let offset = self.data.len() as u32;
        self.data_labels.push((label.to_string(), offset));
        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0); // null terminator
    }

    // ── Resolve fixups ────────────────────────────────────
    fn resolve_fixups(&mut self) {
        for (fixup_offset, target_label) in &self.fixups {
            if let Some((_, target_offset)) = self.label_offsets.iter().find(|(n, _)| n == target_label) {
                let rel32 = (*target_offset as i32) - (*fixup_offset as i32 + 4);
                let bytes = rel32.to_le_bytes();
                self.code[*fixup_offset] = bytes[0];
                self.code[*fixup_offset + 1] = bytes[1];
                self.code[*fixup_offset + 2] = bytes[2];
                self.code[*fixup_offset + 3] = bytes[3];
            }
        }
    }
}

// ── Main ISA compiler ─────────────────────────────────────────
pub fn compile(program: &AllocatedProgram, target: Target) -> CompiledProgram {
    let mut enc = Encoder::new();
    let mut compiled_funcs = Vec::new();

    // Add string data
    for (label, content) in &program.string_data {
        enc.add_data_string(label, content);
    }

    // Compile each function
    for func in &program.functions {
        let offset = enc.current_offset();
        compile_function(func, &mut enc, target);
        let size = enc.current_offset() - offset;
        compiled_funcs.push(CompiledFunction {
            name: func.name.clone(),
            offset,
            size,
        });
        enc.stats.functions_compiled += 1;
    }

    // Generate _start entry point
    let entry_offset = enc.current_offset();
    enc.define_label("_start");

    // _start prologue
    enc.push_reg(X86Reg::RBX);
    enc.sub_rsp_imm(40); // shadow space + alignment

    // Call main if it exists
    if program.functions.iter().any(|f| f.name == "main") {
        enc.call_label("main");
    }

    // Epilogue: ExitProcess(RAX) on Windows, syscall on Linux
    match target {
        Target::Windows => {
            // mov rcx, rax (exit code)
            enc.mov_reg_reg(X86Reg::RCX, X86Reg::RAX);
            // We'll use INT 0x29 fast fail as placeholder
            // Real impl calls kernel32!ExitProcess
            enc.add_rsp_imm(40);
            enc.pop_reg(X86Reg::RBX);
            enc.ret();
        }
        Target::Linux => {
            // mov rdi, rax (exit code)
            enc.mov_reg_reg(X86Reg::RDI, X86Reg::RAX);
            // mov rax, 60 (sys_exit)
            enc.mov_imm64(X86Reg::RAX, 60);
            // syscall
            enc.emit(&[0x0F, 0x05]);
        }
        _ => {
            enc.add_rsp_imm(40);
            enc.pop_reg(X86Reg::RBX);
            enc.ret();
        }
    }

    enc.resolve_fixups();

    let total_bytes = enc.code.len();
    enc.stats.total_bytes = total_bytes;

    CompiledProgram {
        text: enc.code,
        data: enc.data,
        data_labels: enc.data_labels,
        functions: compiled_funcs,
        entry_point: entry_offset,
        target,
        stats: enc.stats,
    }
}

fn compile_function(func: &AllocatedFunction, enc: &mut Encoder, _target: Target) {
    enc.define_label(&func.name);

    // Prologue
    enc.push_reg(X86Reg::RBX);
    if func.stack_size > 0 && func.stack_size <= 127 {
        enc.sub_rsp_imm(func.stack_size as u8);
    }

    // Compile body
    for instr in &func.body {
        compile_instruction(instr, func, enc);
    }

    // Epilogue (if no explicit return)
    if !func.body.iter().any(|i| matches!(i, IRInstruction::Return | IRInstruction::ReturnVoid)) {
        if func.stack_size > 0 && func.stack_size <= 127 {
            enc.add_rsp_imm(func.stack_size as u8);
        }
        enc.pop_reg(X86Reg::RBX);
        enc.ret();
    }
}

fn compile_instruction(instr: &IRInstruction, func: &AllocatedFunction, enc: &mut Encoder) {
    match instr {
        IRInstruction::LoadConst(val) => {
            match val {
                IRConstValue::Int(n) => {
                    enc.mov_imm64(X86Reg::RAX, *n);
                }
                IRConstValue::Float(f) => {
                    let bits = f.to_bits() as i64;
                    enc.mov_imm64(X86Reg::RAX, bits);
                }
                IRConstValue::Bool(b) => {
                    if *b { enc.mov_imm64(X86Reg::RAX, 1); }
                    else { enc.xor_reg_reg(X86Reg::RAX); }
                }
                IRConstValue::None => {
                    enc.xor_reg_reg(X86Reg::RAX);
                }
            }
        }
        IRInstruction::BinOp { op, left, right } => {
            // Compile left → RAX
            compile_instruction(left, func, enc);
            enc.push_reg(X86Reg::RAX);
            // Compile right → RAX
            compile_instruction(right, func, enc);
            enc.mov_reg_reg(X86Reg::RCX, X86Reg::RAX);
            enc.pop_reg(X86Reg::RAX);
            // RAX = left, RCX = right
            match op {
                IROp::Add => enc.add_reg_reg(X86Reg::RAX, X86Reg::RCX),
                IROp::Sub => enc.sub_reg_reg(X86Reg::RAX, X86Reg::RCX),
                IROp::Mul => enc.imul_reg_reg(X86Reg::RAX, X86Reg::RCX),
                IROp::Div | IROp::FloorDiv => enc.idiv_reg(X86Reg::RCX),
                IROp::Mod => {
                    enc.idiv_reg(X86Reg::RCX);
                    enc.mov_reg_reg(X86Reg::RAX, X86Reg::RDX); // remainder
                }
                IROp::Shl => {
                    // SHL RAX, CL
                    enc.rex_w();
                    enc.emit(&[0xD3, 0xE0]);
                }
                IROp::Shr => {
                    // SAR RAX, CL
                    enc.rex_w();
                    enc.emit(&[0xD3, 0xF8]);
                }
                IROp::And => {
                    enc.rex_w();
                    enc.emit(&[0x21, 0xC8]); // AND RAX, RCX
                }
                IROp::Or => {
                    enc.rex_w();
                    enc.emit(&[0x09, 0xC8]); // OR RAX, RCX
                }
                IROp::Xor => {
                    enc.rex_w();
                    enc.emit(&[0x31, 0xC8]); // XOR RAX, RCX
                }
                _ => {} // Pow, MatMul — TODO
            }
        }
        IRInstruction::Compare { op, left, right } => {
            compile_instruction(left, func, enc);
            enc.push_reg(X86Reg::RAX);
            compile_instruction(right, func, enc);
            enc.mov_reg_reg(X86Reg::RCX, X86Reg::RAX);
            enc.pop_reg(X86Reg::RAX);
            enc.cmp_reg_reg(X86Reg::RAX, X86Reg::RCX);
            // SETcc AL
            let cc = match op {
                IRCmpOp::Eq => 0x94,  // SETE
                IRCmpOp::Ne => 0x95,  // SETNE
                IRCmpOp::Lt => 0x9C,  // SETL
                IRCmpOp::Le => 0x9E,  // SETLE
                IRCmpOp::Gt => 0x9F,  // SETG
                IRCmpOp::Ge => 0x9D,  // SETGE
                _ => 0x94,
            };
            enc.emit(&[0x0F, cc, 0xC0]); // SETcc AL
            // MOVZX RAX, AL
            enc.rex_w();
            enc.emit(&[0x0F, 0xB6, 0xC0]);
        }
        IRInstruction::Label(name) => {
            enc.define_label(name);
        }
        IRInstruction::Jump(label) => {
            enc.jmp(label);
        }
        IRInstruction::BranchIfFalse(label) => {
            // TEST RAX, RAX
            enc.rex_w();
            enc.emit(&[0x85, 0xC0]);
            // JE label
            enc.jcc(0x84, label);
        }
        IRInstruction::Return => {
            // RAX already has return value
            if func.stack_size > 0 && func.stack_size <= 127 {
                enc.add_rsp_imm(func.stack_size as u8);
            }
            enc.pop_reg(X86Reg::RBX);
            enc.ret();
        }
        IRInstruction::ReturnVoid => {
            enc.xor_reg_reg(X86Reg::RAX);
            if func.stack_size > 0 && func.stack_size <= 127 {
                enc.add_rsp_imm(func.stack_size as u8);
            }
            enc.pop_reg(X86Reg::RBX);
            enc.ret();
        }
        IRInstruction::Load(name) => {
            if let Some((_, reg)) = func.reg_map.iter().find(|(n, _)| n == name) {
                enc.mov_reg_reg(X86Reg::RAX, *reg);
            }
        }
        IRInstruction::Store(name) => {
            if let Some((_, reg)) = func.reg_map.iter().find(|(n, _)| n == name) {
                enc.mov_reg_reg(*reg, X86Reg::RAX);
            }
        }
        IRInstruction::Call { func: callee, args } => {
            // Push args into ABI registers (Windows: RCX, RDX, R8, R9)
            let abi_regs = [X86Reg::RCX, X86Reg::RDX, X86Reg::R8, X86Reg::R9];
            for (i, arg) in args.iter().enumerate().take(4) {
                compile_instruction(arg, func, enc);
                if i < abi_regs.len() {
                    enc.mov_reg_reg(abi_regs[i], X86Reg::RAX);
                }
            }
            enc.call_label(callee);
        }
        IRInstruction::VarDecl { .. } => {
            // Already handled by register allocator
        }
        IRInstruction::LoadString(_label) => {
            // LEA RAX, [rip + data_offset]
            // For now, load address as immediate (will be fixed by PE relocation)
            enc.mov_imm64(X86Reg::RAX, 0); // placeholder — PE loader patches this
        }
        _ => {}
    }
}
