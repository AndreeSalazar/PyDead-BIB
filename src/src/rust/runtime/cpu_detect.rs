// ADead-BIB Runtime - CPU Detection
// Auto-detección de características del CPU via CPUID
// Determinista y sin dependencias externas
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com

use std::arch::x86_64::{__cpuid, __cpuid_count};

/// Características detectadas del CPU
#[derive(Debug, Clone)]
pub struct CPUFeatures {
    /// Vendor string (12 bytes): "GenuineIntel" o "AuthenticAMD"
    pub vendor: String,
    /// Nombre del modelo del CPU
    pub brand: String,
    /// Número de cores físicos
    pub cores: u32,
    /// Número de threads lógicos
    pub threads: u32,
    /// Frecuencia base en MHz (si disponible)
    pub base_freq_mhz: u32,

    // SIMD Features
    pub has_sse: bool,
    pub has_sse2: bool,
    pub has_sse3: bool,
    pub has_ssse3: bool,
    pub has_sse4_1: bool,
    pub has_sse4_2: bool,
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512f: bool,
    pub has_avx512bw: bool,
    pub has_avx512vl: bool,
    pub has_fma: bool,

    // Otras features útiles
    pub has_popcnt: bool,
    pub has_bmi1: bool,
    pub has_bmi2: bool,
    pub has_aes: bool,

    // Cache sizes (en KB)
    pub cache_l1d: u32,
    pub cache_l1i: u32,
    pub cache_l2: u32,
    pub cache_l3: u32,
}

impl CPUFeatures {
    /// Detecta todas las características del CPU
    pub fn detect() -> Self {
        unsafe { Self::detect_cpuid() }
    }

    /// Implementación usando CPUID
    unsafe fn detect_cpuid() -> Self {
        let mut features = Self {
            vendor: String::new(),
            brand: String::new(),
            cores: 1,
            threads: 1,
            base_freq_mhz: 0,
            has_sse: false,
            has_sse2: false,
            has_sse3: false,
            has_ssse3: false,
            has_sse4_1: false,
            has_sse4_2: false,
            has_avx: false,
            has_avx2: false,
            has_avx512f: false,
            has_avx512bw: false,
            has_avx512vl: false,
            has_fma: false,
            has_popcnt: false,
            has_bmi1: false,
            has_bmi2: false,
            has_aes: false,
            cache_l1d: 0,
            cache_l1i: 0,
            cache_l2: 0,
            cache_l3: 0,
        };

        // CPUID leaf 0: Vendor string
        let cpuid0 = __cpuid(0);
        let max_leaf = cpuid0.eax;

        // Construir vendor string desde EBX, EDX, ECX
        let vendor_bytes: [u8; 12] = [
            (cpuid0.ebx & 0xFF) as u8,
            ((cpuid0.ebx >> 8) & 0xFF) as u8,
            ((cpuid0.ebx >> 16) & 0xFF) as u8,
            ((cpuid0.ebx >> 24) & 0xFF) as u8,
            (cpuid0.edx & 0xFF) as u8,
            ((cpuid0.edx >> 8) & 0xFF) as u8,
            ((cpuid0.edx >> 16) & 0xFF) as u8,
            ((cpuid0.edx >> 24) & 0xFF) as u8,
            (cpuid0.ecx & 0xFF) as u8,
            ((cpuid0.ecx >> 8) & 0xFF) as u8,
            ((cpuid0.ecx >> 16) & 0xFF) as u8,
            ((cpuid0.ecx >> 24) & 0xFF) as u8,
        ];
        features.vendor = String::from_utf8_lossy(&vendor_bytes).to_string();

        // CPUID leaf 1: Feature flags
        if max_leaf >= 1 {
            let cpuid1 = __cpuid(1);

            // EDX flags
            features.has_sse = (cpuid1.edx & (1 << 25)) != 0;
            features.has_sse2 = (cpuid1.edx & (1 << 26)) != 0;

            // ECX flags
            features.has_sse3 = (cpuid1.ecx & (1 << 0)) != 0;
            features.has_ssse3 = (cpuid1.ecx & (1 << 9)) != 0;
            features.has_sse4_1 = (cpuid1.ecx & (1 << 19)) != 0;
            features.has_sse4_2 = (cpuid1.ecx & (1 << 20)) != 0;
            features.has_popcnt = (cpuid1.ecx & (1 << 23)) != 0;
            features.has_aes = (cpuid1.ecx & (1 << 25)) != 0;
            features.has_avx = (cpuid1.ecx & (1 << 28)) != 0;
            features.has_fma = (cpuid1.ecx & (1 << 12)) != 0;

            // Logical processors
            features.threads = ((cpuid1.ebx >> 16) & 0xFF) as u32;
            if features.threads == 0 {
                features.threads = 1;
            }
        }

        // CPUID leaf 7: Extended features
        if max_leaf >= 7 {
            let cpuid7 = __cpuid_count(7, 0);

            // EBX flags
            features.has_bmi1 = (cpuid7.ebx & (1 << 3)) != 0;
            features.has_avx2 = (cpuid7.ebx & (1 << 5)) != 0;
            features.has_bmi2 = (cpuid7.ebx & (1 << 8)) != 0;
            features.has_avx512f = (cpuid7.ebx & (1 << 16)) != 0;
            features.has_avx512bw = (cpuid7.ebx & (1 << 30)) != 0;

            // ECX flags
            features.has_avx512vl = (cpuid7.ecx & (1 << 1)) != 0;
        }

        // CPUID leaf 4: Cache info (Intel)
        if max_leaf >= 4 && features.vendor.contains("Intel") {
            for i in 0..4 {
                let cache = __cpuid_count(4, i);
                let cache_type = cache.eax & 0x1F;
                if cache_type == 0 {
                    break;
                }

                let level = (cache.eax >> 5) & 0x7;
                let ways = ((cache.ebx >> 22) & 0x3FF) + 1;
                let partitions = ((cache.ebx >> 12) & 0x3FF) + 1;
                let line_size = (cache.ebx & 0xFFF) + 1;
                let sets = cache.ecx + 1;
                let size_kb = (ways * partitions * line_size * sets) / 1024;

                match (level, cache_type) {
                    (1, 1) => features.cache_l1d = size_kb,
                    (1, 2) => features.cache_l1i = size_kb,
                    (2, _) => features.cache_l2 = size_kb,
                    (3, _) => features.cache_l3 = size_kb,
                    _ => {}
                }
            }
        }

        // CPUID extended leaf 0x80000000: Max extended leaf
        let cpuid_ext0 = __cpuid(0x80000000);
        let max_ext_leaf = cpuid_ext0.eax;

        // CPUID extended leaves 0x80000002-0x80000004: Brand string
        if max_ext_leaf >= 0x80000004 {
            let mut brand_bytes = Vec::with_capacity(48);

            for leaf in 0x80000002..=0x80000004 {
                let cpuid = __cpuid(leaf);
                brand_bytes.extend_from_slice(&cpuid.eax.to_le_bytes());
                brand_bytes.extend_from_slice(&cpuid.ebx.to_le_bytes());
                brand_bytes.extend_from_slice(&cpuid.ecx.to_le_bytes());
                brand_bytes.extend_from_slice(&cpuid.edx.to_le_bytes());
            }

            features.brand = String::from_utf8_lossy(&brand_bytes)
                .trim_matches('\0')
                .trim()
                .to_string();
        }

        // AMD cache info (leaf 0x80000006)
        if max_ext_leaf >= 0x80000006 && features.vendor.contains("AMD") {
            let cache = __cpuid(0x80000006);
            features.cache_l2 = ((cache.ecx >> 16) & 0xFFFF) as u32;
            features.cache_l3 = ((cache.edx >> 18) & 0x3FFF) as u32 * 512;
        }

        // Estimar cores físicos
        features.cores = std::thread::available_parallelism()
            .map(|p| p.get() as u32)
            .unwrap_or(1);

        features
    }

    /// Retorna el mejor ancho SIMD disponible (en bits)
    pub fn best_simd_width(&self) -> u32 {
        if self.has_avx512f {
            512
        } else if self.has_avx2 {
            256
        } else if self.has_avx {
            256
        } else if self.has_sse2 {
            128
        } else if self.has_sse {
            128
        } else {
            64
        }
    }

    /// Retorna el nombre del mejor conjunto SIMD disponible
    pub fn best_simd_name(&self) -> &'static str {
        if self.has_avx512f {
            "AVX-512"
        } else if self.has_avx2 {
            "AVX2"
        } else if self.has_avx {
            "AVX"
        } else if self.has_sse4_2 {
            "SSE4.2"
        } else if self.has_sse4_1 {
            "SSE4.1"
        } else if self.has_sse2 {
            "SSE2"
        } else if self.has_sse {
            "SSE"
        } else {
            "Scalar"
        }
    }

    /// Verifica si el CPU soporta FMA (Fused Multiply-Add)
    pub fn supports_fma(&self) -> bool {
        self.has_fma && self.has_avx
    }

    /// Imprime un resumen de las características
    pub fn print_summary(&self) {
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║                    CPU FEATURES                            ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Vendor: {:50} ║", self.vendor);
        println!("║ Brand:  {:50} ║", &self.brand[..self.brand.len().min(50)]);
        println!(
            "║ Cores:  {:3} | Threads: {:3}                               ║",
            self.cores, self.threads
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ SIMD Support:                                              ║");
        println!(
            "║   SSE:    {} | SSE2:   {} | SSE3:   {} | SSSE3:  {}        ║",
            if self.has_sse { "✓" } else { "✗" },
            if self.has_sse2 { "✓" } else { "✗" },
            if self.has_sse3 { "✓" } else { "✗" },
            if self.has_ssse3 { "✓" } else { "✗" }
        );
        println!(
            "║   SSE4.1: {} | SSE4.2: {} | AVX:    {} | AVX2:   {}        ║",
            if self.has_sse4_1 { "✓" } else { "✗" },
            if self.has_sse4_2 { "✓" } else { "✗" },
            if self.has_avx { "✓" } else { "✗" },
            if self.has_avx2 { "✓" } else { "✗" }
        );
        println!(
            "║   AVX-512F: {} | AVX-512BW: {} | FMA: {}                    ║",
            if self.has_avx512f { "✓" } else { "✗" },
            if self.has_avx512bw { "✓" } else { "✗" },
            if self.has_fma { "✓" } else { "✗" }
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!(
            "║ Best SIMD: {:10} ({:3}-bit)                            ║",
            self.best_simd_name(),
            self.best_simd_width()
        );
        println!("╠════════════════════════════════════════════════════════════╣");
        println!(
            "║ Cache: L1d {:4}KB | L1i {:4}KB | L2 {:5}KB | L3 {:5}KB   ║",
            self.cache_l1d, self.cache_l1i, self.cache_l2, self.cache_l3
        );
        println!("╚════════════════════════════════════════════════════════════╝");
    }
}

/// Backend de compute disponible
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComputeBackend {
    CpuScalar,
    CpuSse2,
    CpuAvx,
    CpuAvx2,
    CpuAvx512,
    GpuVulkan,
    GpuCuda,
}

impl ComputeBackend {
    /// Selecciona el mejor backend CPU basado en las features
    pub fn best_cpu(features: &CPUFeatures) -> Self {
        if features.has_avx512f {
            Self::CpuAvx512
        } else if features.has_avx2 {
            Self::CpuAvx2
        } else if features.has_avx {
            Self::CpuAvx
        } else if features.has_sse2 {
            Self::CpuSse2
        } else {
            Self::CpuScalar
        }
    }

    /// Retorna el ancho SIMD del backend
    pub fn simd_width(&self) -> u32 {
        match self {
            Self::CpuAvx512 => 512,
            Self::CpuAvx2 | Self::CpuAvx => 256,
            Self::CpuSse2 => 128,
            Self::CpuScalar => 64,
            Self::GpuVulkan | Self::GpuCuda => 0, // N/A para GPU
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_detection() {
        let features = CPUFeatures::detect();

        // Debe detectar algo
        assert!(!features.vendor.is_empty());
        assert!(features.cores >= 1);
        assert!(features.threads >= 1);

        // SSE2 es mínimo para x86-64
        assert!(features.has_sse2);
    }

    #[test]
    fn test_cpu_detection_deterministic() {
        let f1 = CPUFeatures::detect();
        let f2 = CPUFeatures::detect();
        let f3 = CPUFeatures::detect();

        // Debe ser determinista
        assert_eq!(f1.vendor, f2.vendor);
        assert_eq!(f2.vendor, f3.vendor);
        assert_eq!(f1.has_avx2, f2.has_avx2);
        assert_eq!(f2.has_avx2, f3.has_avx2);
    }

    #[test]
    fn test_best_simd() {
        let features = CPUFeatures::detect();
        let width = features.best_simd_width();

        // Debe ser al menos 128 (SSE2)
        assert!(width >= 128);
    }
}
