// ============================================================
// ADead-BIB Output v8.0 — Binary Generation
// ============================================================
// --target boot16     → .bin flat (16-bit stage1)
// --target boot32     → .bin flat (32-bit stage2)
// --target windows    → .exe PE x64 (64-bit)
// --target linux      → .elf ELF x64 (64-bit)
// --target fastos64   → .po v1 (64-bit standard)
// --target fastos128  → .po v2 (128-bit XMM/SSE)
// --target fastos256  → .po v8.0 (256-bit YMM/AVX2) ★
// --target all        → .exe + .elf + .po simultáneamente
// ============================================================

pub mod elf;
pub mod pe;
pub mod po;

pub use elf::ElfOutput;
pub use pe::PeOutput;
pub use po::PoOutput;

/// Target output format v8.0
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    /// 16-bit flat binary (stage1 boot — 512 bytes)
    Boot16,
    /// 32-bit flat binary (stage2 protected mode)
    Boot32,
    /// FastOS 64-bit (.po v1)
    FastOS64,
    /// FastOS 128-bit (.po v2 — XMM/SSE)
    FastOS128,
    /// FastOS 256-bit (.po v8.0 — YMM/AVX2) ★ ALIEN
    FastOS256,
    /// Windows PE x64 (.exe)
    WindowsPE,
    /// Linux ELF x64
    LinuxELF,
    /// Multi-target: genera .exe + .elf + .po simultáneamente
    All,
}

impl OutputFormat {
    pub fn extension(&self) -> &str {
        match self {
            OutputFormat::Boot16 | OutputFormat::Boot32 => ".bin",
            OutputFormat::FastOS64 | OutputFormat::FastOS128 | OutputFormat::FastOS256 => ".po",
            OutputFormat::WindowsPE => ".exe",
            OutputFormat::LinuxELF => ".elf",
            OutputFormat::All => ".exe", // primary extension for multi
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "boot16" => Some(OutputFormat::Boot16),
            "boot32" => Some(OutputFormat::Boot32),
            "fastos" | "fastos64" | "po" => Some(OutputFormat::FastOS64),
            "fastos128" => Some(OutputFormat::FastOS128),
            "fastos256" => Some(OutputFormat::FastOS256),
            "windows" | "pe" | "win" => Some(OutputFormat::WindowsPE),
            "linux" | "elf" => Some(OutputFormat::LinuxELF),
            "all" => Some(OutputFormat::All),
            _ => None,
        }
    }

    /// Returns true if this is a FastOS target
    pub fn is_fastos(&self) -> bool {
        matches!(
            self,
            OutputFormat::FastOS64 | OutputFormat::FastOS128 | OutputFormat::FastOS256
        )
    }

    /// Returns true if this target uses 256-bit mode
    pub fn is_256bit(&self) -> bool {
        matches!(self, OutputFormat::FastOS256)
    }

    /// Returns the bit width for this target
    pub fn bit_width(&self) -> u32 {
        match self {
            OutputFormat::Boot16 => 16,
            OutputFormat::Boot32 => 32,
            OutputFormat::FastOS64 | OutputFormat::WindowsPE | OutputFormat::LinuxELF => 64,
            OutputFormat::FastOS128 => 128,
            OutputFormat::FastOS256 => 256,
            OutputFormat::All => 64, // default for multi
        }
    }

    /// Human-readable description
    pub fn description(&self) -> &str {
        match self {
            OutputFormat::Boot16 => "16-bit flat binary (boot stage1)",
            OutputFormat::Boot32 => "32-bit flat binary (boot stage2)",
            OutputFormat::FastOS64 => "FastOS Po 64-bit",
            OutputFormat::FastOS128 => "FastOS Po 128-bit (XMM/SSE)",
            OutputFormat::FastOS256 => "FastOS Po 256-bit (YMM/AVX2)",
            OutputFormat::WindowsPE => "Windows PE x64",
            OutputFormat::LinuxELF => "Linux ELF x64",
            OutputFormat::All => "Multi-target (PE + ELF + Po)",
        }
    }
}
