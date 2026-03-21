// ============================================================
// PE Output — Windows Executable
// ============================================================
// Delegates to backend::cpu::pe for actual PE generation
// ============================================================

pub struct PeOutput;

impl PeOutput {
    pub fn new() -> Self {
        Self
    }

    /// Genera un PE executable
    pub fn generate(
        &self,
        code: &[u8],
        data: &[u8],
        output_path: &str,
        iat_call_offsets: &[usize],
        string_imm64_offsets: &[usize],
    ) -> Result<(), Box<dyn std::error::Error>> {
        crate::backend::cpu::pe::generate_pe_with_offsets(
            code,
            data,
            output_path,
            iat_call_offsets,
            string_imm64_offsets,
        )
    }
}

impl Default for PeOutput {
    fn default() -> Self {
        Self::new()
    }
}
