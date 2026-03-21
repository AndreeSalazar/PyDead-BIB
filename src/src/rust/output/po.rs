// ============================================================
// Po Output — FastOS Native Format v8.0
// ============================================================
// Formato nativo de FastOS. El formato alien que nadie conoce.
//
// v1.0: 24-byte header — 64-bit standard
// v8.0: 32-byte header — 16/64/128/256-bit support
//
// Po header v8.0 (32 bytes):
//   magic:     0x506F4F53 ('PoOS')       4 bytes
//   version:   0x80 (v8.0)               1 byte
//   bits:      16/64/128/256             1 byte
//   ymm_used:  bitmask YMM0-YMM15       2 bytes
//   code_off:  offset to .text           4 bytes
//   code_size: size of .text             4 bytes
//   data_off:  offset to .data           4 bytes
//   data_size: size of .data             4 bytes
//   soa_map:   offset to SoA table       4 bytes
//   bg_stamp:  BG verification hash      4 bytes
//
// NSA abre binario → Google '0x506F4F53' → 0 resultados → "._."
// ============================================================

/// Po magic: 0x506F4F53 = 'PoOS' (4 bytes, little-endian)
pub const PO_MAGIC: u32 = 0x506F4F53;

/// Legacy magic for v1 compatibility
pub const PO_MAGIC_V1: [u8; 6] = *b"FASTOS";

/// Po versions
pub const PO_VERSION_V1: u8 = 0x10;
pub const PO_VERSION_V2: u8 = 0x20;
pub const PO_VERSION_V8: u8 = 0x80;

/// Bit width identifiers for Po header
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PoBits {
    Bits16 = 16,
    Bits32 = 32,
    Bits64 = 64,
    Bits128 = 128,
    // Note: 256 doesn't fit in u8, so we use a sentinel value
    Bits256 = 0xFF, // 0xFF = 256-bit mode
}

impl PoBits {
    pub fn from_width(w: u32) -> Self {
        match w {
            16 => PoBits::Bits16,
            32 => PoBits::Bits32,
            64 => PoBits::Bits64,
            128 => PoBits::Bits128,
            256 => PoBits::Bits256,
            _ => PoBits::Bits64,
        }
    }

    pub fn to_actual_bits(&self) -> u32 {
        match self {
            PoBits::Bits16 => 16,
            PoBits::Bits32 => 32,
            PoBits::Bits64 => 64,
            PoBits::Bits128 => 128,
            PoBits::Bits256 => 256,
        }
    }
}

/// Header del formato .Po v8.0 (32 bytes exactos)
#[derive(Debug, Clone)]
pub struct PoHeader {
    pub magic: u32,       // 0x506F4F53 'PoOS' (4 bytes)
    pub version: u8,      // 0x80 = v8.0       (1 byte)
    pub bits: u8,         // PoBits             (1 byte)
    pub ymm_used: u16,    // bitmask YMM0-15   (2 bytes)
    pub code_offset: u32, // offset to code     (4 bytes)
    pub code_size: u32,   // code section size  (4 bytes)
    pub data_offset: u32, // offset to data     (4 bytes)
    pub data_size: u32,   // data section size  (4 bytes)
    pub soa_map: u32,     // offset to SoA map  (4 bytes)
    pub bg_stamp: u32,    // BG hash            (4 bytes)
}
// Total: 4+1+1+2+4+4+4+4+4+4 = 32 bytes

impl PoHeader {
    pub fn new_v8(bits: PoBits) -> Self {
        Self {
            magic: PO_MAGIC,
            version: PO_VERSION_V8,
            bits: bits as u8,
            ymm_used: 0,
            code_offset: 32, // header size
            code_size: 0,
            data_offset: 32,
            data_size: 0,
            soa_map: 0,
            bg_stamp: 0,
        }
    }

    /// Serialize header to 32 bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4] = self.version;
        buf[5] = self.bits;
        buf[6..8].copy_from_slice(&self.ymm_used.to_le_bytes());
        buf[8..12].copy_from_slice(&self.code_offset.to_le_bytes());
        buf[12..16].copy_from_slice(&self.code_size.to_le_bytes());
        buf[16..20].copy_from_slice(&self.data_offset.to_le_bytes());
        buf[20..24].copy_from_slice(&self.data_size.to_le_bytes());
        buf[24..28].copy_from_slice(&self.soa_map.to_le_bytes());
        buf[28..32].copy_from_slice(&self.bg_stamp.to_le_bytes());
        buf
    }

    /// Parse header from bytes
    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 32 {
            return None;
        }
        let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if magic != PO_MAGIC {
            return None;
        }
        Some(Self {
            magic,
            version: buf[4],
            bits: buf[5],
            ymm_used: u16::from_le_bytes([buf[6], buf[7]]),
            code_offset: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            code_size: u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
            data_offset: u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]),
            data_size: u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]),
            soa_map: u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]),
            bg_stamp: u32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]),
        })
    }
}

pub struct PoOutput;

impl PoOutput {
    pub fn new() -> Self {
        Self
    }

    /// Genera un binario .Po v8.0 nativo (32-byte header)
    pub fn generate(
        &self,
        code: &[u8],
        data: &[u8],
        output_path: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        self.generate_v8(code, data, PoBits::Bits64, 0, 0, output_path)
    }

    /// Genera un binario .Po v8.0 con parámetros completos
    pub fn generate_v8(
        &self,
        code: &[u8],
        data: &[u8],
        bits: PoBits,
        ymm_used: u16,
        bg_stamp: u32,
        output_path: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let header_size = 32u32;
        let code_offset = header_size;
        let code_size = code.len() as u32;
        let data_offset = code_offset + code_size;
        let data_size = data.len() as u32;

        let header = PoHeader {
            magic: PO_MAGIC,
            version: PO_VERSION_V8,
            bits: bits as u8,
            ymm_used,
            code_offset,
            code_size,
            data_offset,
            data_size,
            soa_map: 0, // no SoA map section yet
            bg_stamp,
        };

        let mut binary = Vec::new();
        binary.extend_from_slice(&header.to_bytes()); // 32 bytes
        binary.extend_from_slice(code);
        binary.extend_from_slice(data);

        let total = binary.len();
        std::fs::write(output_path, &binary)?;
        Ok(total)
    }

    /// Generate legacy v1 format (24-byte header) for backward compat
    pub fn generate_v1(
        &self,
        code: &[u8],
        data: &[u8],
        output_path: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let header_size = 24u32;
        let code_offset = header_size;
        let code_size = code.len() as u32;
        let data_offset = code_offset + code_size;
        let data_size = data.len() as u32;

        let mut binary = Vec::new();
        binary.extend_from_slice(&PO_MAGIC_V1);
        binary.extend_from_slice(&1u16.to_le_bytes());
        binary.extend_from_slice(&code_offset.to_le_bytes());
        binary.extend_from_slice(&code_size.to_le_bytes());
        binary.extend_from_slice(&data_offset.to_le_bytes());
        binary.extend_from_slice(&data_size.to_le_bytes());
        binary.extend_from_slice(code);
        binary.extend_from_slice(data);

        let total = binary.len();
        std::fs::write(output_path, &binary)?;
        Ok(total)
    }
}

impl Default for PoOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_po_v8_header_size() {
        // 4+1+1+2+4+4+4+4+4+4 = 32 bytes
        let header = PoHeader::new_v8(PoBits::Bits256);
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_po_magic() {
        let header = PoHeader::new_v8(PoBits::Bits64);
        let bytes = header.to_bytes();
        // Magic: 0x506F4F53 LE = [0x53, 0x4F, 0x6F, 0x50]
        assert_eq!(bytes[0], 0x53);
        assert_eq!(bytes[1], 0x4F);
        assert_eq!(bytes[2], 0x6F);
        assert_eq!(bytes[3], 0x50);
    }

    #[test]
    fn test_po_roundtrip() {
        let mut header = PoHeader::new_v8(PoBits::Bits256);
        header.ymm_used = 0x000F; // YMM0-3
        header.bg_stamp = 0xDEADBEEF;
        header.code_size = 1024;
        header.data_size = 64;

        let bytes = header.to_bytes();
        let parsed = PoHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.magic, PO_MAGIC);
        assert_eq!(parsed.version, PO_VERSION_V8);
        assert_eq!(parsed.bits, PoBits::Bits256 as u8);
        assert_eq!(parsed.ymm_used, 0x000F);
        assert_eq!(parsed.bg_stamp, 0xDEADBEEF);
        assert_eq!(parsed.code_size, 1024);
        assert_eq!(parsed.data_size, 64);
    }

    #[test]
    fn test_po_bits() {
        assert_eq!(PoBits::from_width(256).to_actual_bits(), 256);
        assert_eq!(PoBits::from_width(64).to_actual_bits(), 64);
        assert_eq!(PoBits::from_width(16).to_actual_bits(), 16);
    }

    #[test]
    fn test_po_bad_magic() {
        let buf = [0u8; 32]; // all zeros — bad magic
        assert!(PoHeader::from_bytes(&buf).is_none());
    }
}
