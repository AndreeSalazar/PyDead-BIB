import os

def fix_rust_syntax():
    # 1. simd_avx2.rs
    simd_path = "src/rust/backend/isa/simd_avx2.rs"
    if os.path.exists(simd_path):
        with open(simd_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
        
        # Add impl Encoder wrapper
        content = "use super::encoder::Encoder;\nuse crate::backend::reg_alloc::X86Reg;\n\nimpl Encoder {\n" + "".join(lines)
        if not content.strip().endswith("}"):
            content += "\n}\n"
            
        with open(simd_path, "w", encoding="utf-8") as f:
            f.write(content)
            
    # 2. x86_core.rs
    core_path = "src/rust/backend/isa/x86_core.rs"
    if os.path.exists(core_path):
        with open(core_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
            
        # x86_core contains compile() and emit_runtime_stubs
        content = "use super::encoder::Encoder;\nuse crate::backend::isa::{Target, CompiledProgram};\nuse crate::backend::reg_alloc::{AllocatedProgram, X86Reg};\n\n" + "".join(lines)
        with open(core_path, "w", encoding="utf-8") as f:
            f.write(content)
            
    # 3. encoder.rs
    enc_path = "src/rust/backend/isa/encoder.rs"
    if os.path.exists(enc_path):
        with open(enc_path, "r", encoding="utf-8") as f:
            content = f.read()
        
        # make struct fields pub(crate)
        content = content.replace("struct Encoder {", "pub(crate) struct Encoder {")
        content = content.replace("    code: Vec<u8>,", "    pub(crate) code: Vec<u8>,")
        content = content.replace("    data: Vec<u8>,", "    pub(crate) data: Vec<u8>,")
        content = content.replace("    data_labels:", "    pub(crate) data_labels:")
        content = content.replace("    label_offsets:", "    pub(crate) label_offsets:")
        content = content.replace("    fixups:", "    pub(crate) fixups:")
        content = content.replace("    iat_fixups:", "    pub(crate) iat_fixups:")
        content = content.replace("    data_fixups:", "    pub(crate) data_fixups:")
        content = content.replace("    stats:", "    pub(crate) stats:")
        
        with open(enc_path, "w", encoding="utf-8") as f:
            f.write(content)

if __name__ == "__main__":
    fix_rust_syntax()
    print("Syntax fixed.")
