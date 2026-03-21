// ============================================================
// ELF Output — Linux Executable
// ============================================================
// Delegates to backend::cpu::elf for actual ELF generation
// ============================================================

pub struct ElfOutput;

impl ElfOutput {
    pub fn new() -> Self {
        Self
    }

    /// Genera un ELF executable
    pub fn generate(
        &self,
        code: &[u8],
        data: &[u8],
        output_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        crate::backend::cpu::elf::generate_elf(code, data, output_path)
    }
}

impl Default for ElfOutput {
    fn default() -> Self {
        Self::new()
    }
}
