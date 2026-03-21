use crate::runtime::cpu_detect::CPUFeatures;

pub struct SIMDGenerator {
    features: CPUFeatures,
    width: u32,
}

impl SIMDGenerator {
    pub fn new() -> Self {
        let features = CPUFeatures::detect();
        let width = if features.has_avx512f {
            512
        } else if features.has_avx2 {
            256
        } else if features.has_sse2 {
            128
        } else {
            64
        };

        Self { features, width }
    }

    /// Genera código para suma de vectores
    pub fn emit_vec_add(&self, code: &mut Vec<u8>) {
        match self.width {
            512 => {
                // vaddps zmm0, zmm0, zmm1 (AVX-512)
                code.extend_from_slice(&[0x62, 0xF1, 0x7C, 0x48, 0x58, 0xC1]);
            }
            256 => {
                // vaddps ymm0, ymm0, ymm1 (AVX2)
                code.extend_from_slice(&[0xC5, 0xFC, 0x58, 0xC1]);
            }
            128 => {
                // addps xmm0, xmm1 (SSE)
                code.extend_from_slice(&[0x0F, 0x58, 0xC1]);
            }
            _ => {} // Fallback scalar
        }
    }

    /// Genera código para max vectorizado (ReLU)
    pub fn emit_vec_max_zero(&self, code: &mut Vec<u8>) {
        match self.width {
            256 => {
                // vxorps ymm1, ymm1, ymm1    ; ymm1 = 0
                code.extend_from_slice(&[0xC5, 0xF4, 0x57, 0xC9]);
                // vmaxps ymm0, ymm0, ymm1    ; ymm0 = max(ymm0, 0)
                code.extend_from_slice(&[0xC5, 0xFC, 0x5F, 0xC1]);
            }
            _ => {}
        }
    }
}
