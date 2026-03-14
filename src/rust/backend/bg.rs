// ============================================================
// PyDead-BIB Binary Guardian (BG) — Heredado de ADead-BIB v8.0
// ============================================================
// Stamps compiled binaries with integrity metadata
// BG magic: 0x42494221 ("BIB!")
// Po magic: 0x506F4F53 ("PoOS") for FastOS targets
// ============================================================

use crate::backend::isa::{CompiledProgram, Target};

// ── BG Stamp ──────────────────────────────────────────────────
pub const BG_MAGIC: u32 = 0x42494221;     // "BIB!"
pub const PO_MAGIC: u32 = 0x506F4F53;     // "PoOS"
pub const PYDEAD_VERSION: u16 = 0x0102;   // v1.2

#[repr(C)]
pub struct BGStamp {
    pub magic: u32,
    pub version: u16,
    pub flags: u16,
    pub text_size: u32,
    pub data_size: u32,
    pub entry_point: u32,
    pub checksum: u32,
}

impl BGStamp {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(24);
        bytes.extend_from_slice(&self.magic.to_le_bytes());
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.flags.to_le_bytes());
        bytes.extend_from_slice(&self.text_size.to_le_bytes());
        bytes.extend_from_slice(&self.data_size.to_le_bytes());
        bytes.extend_from_slice(&self.entry_point.to_le_bytes());
        bytes.extend_from_slice(&self.checksum.to_le_bytes());
        bytes
    }
}

// ── Stamp a compiled program ──────────────────────────────────
pub fn stamp(program: &CompiledProgram) -> StampedProgram {
    let flags = match program.target {
        Target::Windows => 0x0001,
        Target::Linux => 0x0002,
        Target::FastOS64 => 0x0010,
        Target::FastOS128 => 0x0020,
        Target::FastOS256 => 0x0040,
    };

    let checksum = compute_checksum(&program.text, &program.data);

    let bg = BGStamp {
        magic: if matches!(program.target, Target::FastOS64 | Target::FastOS128 | Target::FastOS256) {
            PO_MAGIC
        } else {
            BG_MAGIC
        },
        version: PYDEAD_VERSION,
        flags,
        text_size: program.text.len() as u32,
        data_size: program.data.len() as u32,
        entry_point: program.entry_point,
        checksum,
    };

    StampedProgram {
        stamp: bg,
        text: program.text.clone(),
        data: program.data.clone(),
        data_labels: program.data_labels.clone(),
        functions: program.functions.iter().map(|f| (f.name.clone(), f.offset, f.size)).collect(),
        entry_point: program.entry_point,
        target: program.target,
        iat_fixups: program.iat_fixups.clone(),
        data_fixups: program.data_fixups.clone(),
    }
}

pub struct StampedProgram {
    pub stamp: BGStamp,
    pub text: Vec<u8>,
    pub data: Vec<u8>,
    pub data_labels: Vec<(String, u32)>,
    pub functions: Vec<(String, u32, u32)>,
    pub entry_point: u32,
    pub target: Target,
    pub iat_fixups: Vec<(u32, usize)>,
    pub data_fixups: Vec<(u32, String)>,
}

fn compute_checksum(text: &[u8], data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for &b in text.iter().chain(data.iter()) {
        sum = sum.wrapping_add(b as u32);
        sum = sum.wrapping_mul(31);
    }
    sum ^ 0xDEAD_B1B0
}
