// ADead-BIB Runtime - GPU Detection
// Auto-detección de GPU via Vulkan/CUDA/nvidia-smi
// Detección real de hardware para optimización máxima
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com

use std::process::Command;

/// Vendor de GPU detectado
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GPUVendor {
    NVIDIA,
    AMD,
    Intel,
    Unknown,
}

impl GPUVendor {
    pub fn from_id(vendor_id: u32) -> Self {
        match vendor_id {
            0x10DE => Self::NVIDIA,
            0x1002 => Self::AMD,
            0x8086 => Self::Intel,
            _ => Self::Unknown,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::NVIDIA => "NVIDIA",
            Self::AMD => "AMD",
            Self::Intel => "Intel",
            Self::Unknown => "Unknown",
        }
    }
}

/// Arquitectura NVIDIA
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvidiaArch {
    Turing,      // RTX 20xx
    Ampere,      // RTX 30xx
    AdaLovelace, // RTX 40xx
    Unknown,
}

impl NvidiaArch {
    pub fn from_name(name: &str) -> Self {
        let name_lower = name.to_lowercase();
        if name_lower.contains("4090")
            || name_lower.contains("4080")
            || name_lower.contains("4070")
            || name_lower.contains("4060")
        {
            Self::AdaLovelace
        } else if name_lower.contains("3090")
            || name_lower.contains("3080")
            || name_lower.contains("3070")
            || name_lower.contains("3060")
            || name_lower.contains("3050")
        {
            Self::Ampere
        } else if name_lower.contains("2080")
            || name_lower.contains("2070")
            || name_lower.contains("2060")
        {
            Self::Turing
        } else {
            Self::Unknown
        }
    }

    /// Compute capability
    pub fn compute_capability(&self) -> (u32, u32) {
        match self {
            Self::AdaLovelace => (8, 9),
            Self::Ampere => (8, 6),
            Self::Turing => (7, 5),
            Self::Unknown => (5, 0),
        }
    }
}

/// Especificaciones de GPU conocidas
#[derive(Debug, Clone)]
pub struct GPUSpecs {
    pub name: &'static str,
    pub vram_mb: u32,
    pub cuda_cores: u32,
    pub sm_count: u32,
    pub base_clock_mhz: u32,
    pub boost_clock_mhz: u32,
    pub memory_bus_bits: u32,
    pub memory_bandwidth_gbs: f32,
    pub tflops_fp32: f32,
    pub tflops_fp16: f32,
    pub tdp_watts: u32,
    pub arch: NvidiaArch,
}

/// Base de datos de GPUs conocidas
pub fn get_gpu_specs(name: &str) -> Option<GPUSpecs> {
    let name_lower = name.to_lowercase();

    // RTX 30 Series (Ampere)
    if name_lower.contains("3060") && name_lower.contains("12") {
        Some(GPUSpecs {
            name: "NVIDIA GeForce RTX 3060 12GB",
            vram_mb: 12288,
            cuda_cores: 3584,
            sm_count: 28,
            base_clock_mhz: 1320,
            boost_clock_mhz: 1777,
            memory_bus_bits: 192,
            memory_bandwidth_gbs: 360.0,
            tflops_fp32: 12.74,
            tflops_fp16: 25.48, // Con Tensor Cores
            tdp_watts: 170,
            arch: NvidiaArch::Ampere,
        })
    } else if name_lower.contains("3060") {
        Some(GPUSpecs {
            name: "NVIDIA GeForce RTX 3060",
            vram_mb: 12288,
            cuda_cores: 3584,
            sm_count: 28,
            base_clock_mhz: 1320,
            boost_clock_mhz: 1777,
            memory_bus_bits: 192,
            memory_bandwidth_gbs: 360.0,
            tflops_fp32: 12.74,
            tflops_fp16: 25.48,
            tdp_watts: 170,
            arch: NvidiaArch::Ampere,
        })
    } else if name_lower.contains("3070") {
        Some(GPUSpecs {
            name: "NVIDIA GeForce RTX 3070",
            vram_mb: 8192,
            cuda_cores: 5888,
            sm_count: 46,
            base_clock_mhz: 1500,
            boost_clock_mhz: 1725,
            memory_bus_bits: 256,
            memory_bandwidth_gbs: 448.0,
            tflops_fp32: 20.31,
            tflops_fp16: 40.62,
            tdp_watts: 220,
            arch: NvidiaArch::Ampere,
        })
    } else if name_lower.contains("3080") {
        Some(GPUSpecs {
            name: "NVIDIA GeForce RTX 3080",
            vram_mb: 10240,
            cuda_cores: 8704,
            sm_count: 68,
            base_clock_mhz: 1440,
            boost_clock_mhz: 1710,
            memory_bus_bits: 320,
            memory_bandwidth_gbs: 760.0,
            tflops_fp32: 29.77,
            tflops_fp16: 59.54,
            tdp_watts: 320,
            arch: NvidiaArch::Ampere,
        })
    } else if name_lower.contains("3090") {
        Some(GPUSpecs {
            name: "NVIDIA GeForce RTX 3090",
            vram_mb: 24576,
            cuda_cores: 10496,
            sm_count: 82,
            base_clock_mhz: 1395,
            boost_clock_mhz: 1695,
            memory_bus_bits: 384,
            memory_bandwidth_gbs: 936.0,
            tflops_fp32: 35.58,
            tflops_fp16: 71.16,
            tdp_watts: 350,
            arch: NvidiaArch::Ampere,
        })
    // RTX 40 Series (Ada Lovelace)
    } else if name_lower.contains("4090") {
        Some(GPUSpecs {
            name: "NVIDIA GeForce RTX 4090",
            vram_mb: 24576,
            cuda_cores: 16384,
            sm_count: 128,
            base_clock_mhz: 2235,
            boost_clock_mhz: 2520,
            memory_bus_bits: 384,
            memory_bandwidth_gbs: 1008.0,
            tflops_fp32: 82.58,
            tflops_fp16: 165.16,
            tdp_watts: 450,
            arch: NvidiaArch::AdaLovelace,
        })
    } else if name_lower.contains("4080") {
        Some(GPUSpecs {
            name: "NVIDIA GeForce RTX 4080",
            vram_mb: 16384,
            cuda_cores: 9728,
            sm_count: 76,
            base_clock_mhz: 2205,
            boost_clock_mhz: 2505,
            memory_bus_bits: 256,
            memory_bandwidth_gbs: 716.8,
            tflops_fp32: 48.74,
            tflops_fp16: 97.48,
            tdp_watts: 320,
            arch: NvidiaArch::AdaLovelace,
        })
    } else if name_lower.contains("4070") {
        Some(GPUSpecs {
            name: "NVIDIA GeForce RTX 4070",
            vram_mb: 12288,
            cuda_cores: 5888,
            sm_count: 46,
            base_clock_mhz: 1920,
            boost_clock_mhz: 2475,
            memory_bus_bits: 192,
            memory_bandwidth_gbs: 504.0,
            tflops_fp32: 29.15,
            tflops_fp16: 58.30,
            tdp_watts: 200,
            arch: NvidiaArch::AdaLovelace,
        })
    } else {
        None
    }
}

/// Características de la GPU detectada
#[derive(Debug, Clone)]
pub struct GPUFeatures {
    /// GPU disponible
    pub available: bool,
    /// Vulkan disponible
    pub vulkan_available: bool,
    /// CUDA disponible (solo NVIDIA)
    pub cuda_available: bool,
    /// Vendor de la GPU
    pub vendor: GPUVendor,
    /// Nombre del dispositivo
    pub device_name: String,
    /// VRAM total en MB
    pub vram_mb: u32,
    /// Número de compute units/SMs
    pub compute_units: u32,
    /// Tamaño máximo de workgroup
    pub max_workgroup_size: u32,
    /// Soporte FP16
    pub supports_fp16: bool,
    /// Soporte FP64
    pub supports_fp64: bool,
    /// Soporte INT8
    pub supports_int8: bool,
    /// Versión de Vulkan (major.minor)
    pub vulkan_version: (u32, u32),
    /// Versión de CUDA (si aplica)
    pub cuda_version: (u32, u32),
    /// Especificaciones detalladas (si GPU conocida)
    pub specs: Option<GPUSpecs>,
    /// TFLOPS teóricos FP32
    pub theoretical_tflops: f32,
}

impl Default for GPUFeatures {
    fn default() -> Self {
        Self {
            available: false,
            vulkan_available: false,
            cuda_available: false,
            vendor: GPUVendor::Unknown,
            device_name: String::new(),
            vram_mb: 0,
            compute_units: 0,
            max_workgroup_size: 0,
            supports_fp16: false,
            supports_fp64: false,
            supports_int8: false,
            vulkan_version: (0, 0),
            cuda_version: (0, 0),
            specs: None,
            theoretical_tflops: 0.0,
        }
    }
}

impl GPUFeatures {
    /// Detecta GPU disponible (nvidia-smi primero, luego Vulkan)
    pub fn detect() -> Self {
        let mut features = Self::default();

        // Intentar nvidia-smi primero (más preciso para NVIDIA)
        if let Some(nvidia_features) = Self::detect_nvidia_smi() {
            features = nvidia_features;
            features.available = true;
            features.vendor = GPUVendor::NVIDIA;

            // Verificar Vulkan
            if detect_vulkan_simple() {
                features.vulkan_available = true;
                features.vulkan_version = (1, 3); // RTX 30xx soporta Vulkan 1.3
            }

            // Verificar CUDA
            if detect_cuda_simple() {
                features.cuda_available = true;
                features.cuda_version = (12, 0);
            }

            return features;
        }

        // Fallback a detección Vulkan genérica
        if let Some(vk_features) = Self::detect_vulkan() {
            features = vk_features;
            features.vulkan_available = true;
            features.available = true;
        }

        // Si es NVIDIA, intentar CUDA también
        if features.vendor == GPUVendor::NVIDIA {
            if let Some((major, minor)) = Self::detect_cuda_version() {
                features.cuda_available = true;
                features.cuda_version = (major, minor);
            }
        }

        features
    }

    /// Detecta GPU NVIDIA via nvidia-smi
    fn detect_nvidia_smi() -> Option<Self> {
        // Ejecutar nvidia-smi para obtener info de GPU
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,memory.total,driver_version",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next()?;
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

        if parts.len() < 2 {
            return None;
        }

        let device_name = parts[0].to_string();
        let vram_mb: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

        // Buscar specs conocidas
        let specs = get_gpu_specs(&device_name);

        let (compute_units, supports_fp64, tflops) = if let Some(ref s) = specs {
            (s.sm_count, true, s.tflops_fp32)
        } else {
            (0, false, 0.0)
        };

        Some(Self {
            available: true,
            vulkan_available: false, // Se verifica después
            cuda_available: false,   // Se verifica después
            vendor: GPUVendor::NVIDIA,
            device_name,
            vram_mb,
            compute_units,
            max_workgroup_size: 1024,
            supports_fp16: true, // Todas las RTX soportan FP16
            supports_fp64,
            supports_int8: true, // Tensor Cores
            vulkan_version: (0, 0),
            cuda_version: (0, 0),
            specs,
            theoretical_tflops: tflops,
        })
    }

    /// Detecta GPU via Vulkan (verificación de archivos)
    fn detect_vulkan() -> Option<Self> {
        if !detect_vulkan_simple() {
            return None;
        }

        Some(Self {
            available: true,
            vulkan_available: true,
            cuda_available: false,
            vendor: GPUVendor::Unknown,
            device_name: "Vulkan Device (detected)".to_string(),
            vram_mb: 0,
            compute_units: 0,
            max_workgroup_size: 1024,
            supports_fp16: true,
            supports_fp64: false,
            supports_int8: true,
            vulkan_version: (1, 0),
            cuda_version: (0, 0),
            specs: None,
            theoretical_tflops: 0.0,
        })
    }

    /// Detecta versión de CUDA (verificación de archivos)
    fn detect_cuda_version() -> Option<(u32, u32)> {
        if detect_cuda_simple() {
            Some((12, 0))
        } else {
            None
        }
    }

    /// Calcula workgroup size óptimo para esta GPU
    pub fn optimal_workgroup_size(&self) -> (u32, u32, u32) {
        match self.vendor {
            GPUVendor::NVIDIA => {
                // NVIDIA: warp size = 32, óptimo 256 threads (8 warps)
                if self.compute_units >= 28 {
                    (256, 1, 1) // RTX 3060+
                } else {
                    (128, 1, 1)
                }
            }
            GPUVendor::AMD => (64, 1, 1),   // Wavefront 64
            GPUVendor::Intel => (32, 1, 1), // EU threads
            _ => (128, 1, 1),
        }
    }

    /// Calcula workgroup size óptimo para MatMul (2D)
    pub fn optimal_matmul_workgroup(&self) -> (u32, u32, u32) {
        match self.vendor {
            GPUVendor::NVIDIA => (16, 16, 1), // 256 threads, tile 16x16
            GPUVendor::AMD => (16, 16, 1),
            _ => (8, 8, 1),
        }
    }

    /// Estima tiempo para MatMul en ms
    pub fn estimate_matmul_time_ms(&self, m: u32, n: u32, k: u32) -> f64 {
        if self.theoretical_tflops <= 0.0 {
            return 0.0;
        }

        // FLOPs para MatMul: 2 * M * N * K
        let flops = 2.0 * m as f64 * n as f64 * k as f64;
        let tflops = self.theoretical_tflops as f64 * 1e12;

        // Eficiencia típica ~50% para MatMul bien optimizado
        let efficiency = 0.5;
        (flops / (tflops * efficiency)) * 1000.0
    }

    /// Imprime resumen de la GPU
    pub fn print_summary(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                      GPU DETECTION                            ║");
        println!("╠══════════════════════════════════════════════════════════════╣");

        if self.available {
            println!("║ ✅ GPU Available                                             ║");
            let name_display = if self.device_name.len() > 45 {
                &self.device_name[..45]
            } else {
                &self.device_name
            };
            println!("║ Device:    {:<48} ║", name_display);
            println!("║ Vendor:    {:<48} ║", self.vendor.name());
            println!(
                "║ VRAM:      {:<5} MB ({:.1} GB)                               ║",
                self.vram_mb,
                self.vram_mb as f32 / 1024.0
            );
            println!(
                "║ Compute:   {} SMs                                           ║",
                self.compute_units
            );
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!(
                "║ Vulkan:    {} (v{}.{})                                        ║",
                if self.vulkan_available { "✅" } else { "❌" },
                self.vulkan_version.0,
                self.vulkan_version.1
            );
            println!(
                "║ CUDA:      {} (v{}.{})                                        ║",
                if self.cuda_available { "✅" } else { "❌" },
                self.cuda_version.0,
                self.cuda_version.1
            );
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!(
                "║ FP16: {} | FP64: {} | INT8: {}                               ║",
                if self.supports_fp16 { "✅" } else { "❌" },
                if self.supports_fp64 { "✅" } else { "❌" },
                if self.supports_int8 { "✅" } else { "❌" }
            );

            if let Some(ref specs) = self.specs {
                println!("╠══════════════════════════════════════════════════════════════╣");
                println!("║ 📊 SPECIFICATIONS                                            ║");
                println!(
                    "║ CUDA Cores:    {:<6}                                        ║",
                    specs.cuda_cores
                );
                println!(
                    "║ Boost Clock:   {:<4} MHz                                     ║",
                    specs.boost_clock_mhz
                );
                println!(
                    "║ Memory Bus:    {:<3} bit                                      ║",
                    specs.memory_bus_bits
                );
                println!(
                    "║ Bandwidth:     {:<6.1} GB/s                                   ║",
                    specs.memory_bandwidth_gbs
                );
                println!(
                    "║ FP32:          {:<6.2} TFLOPS                                 ║",
                    specs.tflops_fp32
                );
                println!(
                    "║ FP16:          {:<6.2} TFLOPS                                 ║",
                    specs.tflops_fp16
                );
                println!(
                    "║ TDP:           {:<3} W                                        ║",
                    specs.tdp_watts
                );
                println!(
                    "║ Architecture:  {:?}                                       ║",
                    specs.arch
                );
            }

            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ 🎯 OPTIMAL SETTINGS                                          ║");
            let wg = self.optimal_workgroup_size();
            println!(
                "║ Workgroup:     ({}, {}, {})                                    ║",
                wg.0, wg.1, wg.2
            );
            let matmul_wg = self.optimal_matmul_workgroup();
            println!(
                "║ MatMul Tile:   ({}, {}, {})                                    ║",
                matmul_wg.0, matmul_wg.1, matmul_wg.2
            );

            // Estimación de rendimiento
            let matmul_time = self.estimate_matmul_time_ms(1024, 1024, 1024);
            if matmul_time > 0.0 {
                println!(
                    "║ MatMul 1024³:  ~{:.2} ms (estimated)                          ║",
                    matmul_time
                );
            }
        } else {
            println!("║ ❌ No GPU Available                                          ║");
            println!("║ No compatible GPU detected                                   ║");
        }

        println!("╚══════════════════════════════════════════════════════════════╝");
    }
}

/// Detección simple sin libloading (para tests)
pub fn detect_vulkan_simple() -> bool {
    #[cfg(windows)]
    {
        // Verificar si vulkan-1.dll existe en el sistema
        use std::path::Path;
        let system32 = std::env::var("SystemRoot")
            .map(|r| format!("{}\\System32\\vulkan-1.dll", r))
            .unwrap_or_default();
        Path::new(&system32).exists()
    }
    #[cfg(not(windows))]
    {
        use std::path::Path;
        Path::new("/usr/lib/libvulkan.so.1").exists()
            || Path::new("/usr/lib/x86_64-linux-gnu/libvulkan.so.1").exists()
    }
}

/// Detección simple de CUDA
pub fn detect_cuda_simple() -> bool {
    #[cfg(windows)]
    {
        use std::path::Path;
        let system32 = std::env::var("SystemRoot")
            .map(|r| format!("{}\\System32\\nvcuda.dll", r))
            .unwrap_or_default();
        Path::new(&system32).exists()
    }
    #[cfg(not(windows))]
    {
        use std::path::Path;
        Path::new("/usr/lib/libcuda.so").exists()
            || Path::new("/usr/lib/x86_64-linux-gnu/libcuda.so").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vulkan_detection_deterministic() {
        let result1 = detect_vulkan_simple();
        let result2 = detect_vulkan_simple();
        let result3 = detect_vulkan_simple();

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn test_cuda_detection_deterministic() {
        let result1 = detect_cuda_simple();
        let result2 = detect_cuda_simple();
        let result3 = detect_cuda_simple();

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn test_vendor_from_id() {
        assert_eq!(GPUVendor::from_id(0x10DE), GPUVendor::NVIDIA);
        assert_eq!(GPUVendor::from_id(0x1002), GPUVendor::AMD);
        assert_eq!(GPUVendor::from_id(0x8086), GPUVendor::Intel);
    }
}
