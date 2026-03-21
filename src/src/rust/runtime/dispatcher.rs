// ADead-BIB Runtime - Auto-Dispatcher CPU+GPU
// Selección automática del mejor backend para cada operación
// Determinista y optimizado para máximo rendimiento
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com

use super::cpu_detect::{CPUFeatures, ComputeBackend};
use crate::backend::gpu::gpu_detect::{GPUFeatures, GPUVendor};

/// Umbral de tamaño para usar GPU (en elementos)
const DEFAULT_GPU_THRESHOLD: usize = 1024 * 1024; // 1M elementos

/// Auto-dispatcher que selecciona el mejor backend
#[derive(Debug)]
pub struct AutoDispatcher {
    /// Características del CPU
    pub cpu: CPUFeatures,
    /// Características de la GPU (si disponible)
    pub gpu: Option<GPUFeatures>,
    /// Umbral para usar GPU
    pub gpu_threshold: usize,
    /// Forzar backend específico (None = auto)
    pub forced_backend: Option<ComputeBackend>,
}

impl AutoDispatcher {
    /// Crea un nuevo dispatcher con detección automática
    pub fn new() -> Self {
        let cpu = CPUFeatures::detect();
        let gpu_features = GPUFeatures::detect();
        let gpu = if gpu_features.available {
            Some(gpu_features)
        } else {
            None
        };

        Self {
            cpu,
            gpu,
            gpu_threshold: DEFAULT_GPU_THRESHOLD,
            forced_backend: None,
        }
    }

    /// Crea dispatcher solo CPU (sin GPU)
    pub fn cpu_only() -> Self {
        Self {
            cpu: CPUFeatures::detect(),
            gpu: None,
            gpu_threshold: usize::MAX,
            forced_backend: None,
        }
    }

    /// Configura el umbral para usar GPU
    pub fn with_gpu_threshold(mut self, threshold: usize) -> Self {
        self.gpu_threshold = threshold;
        self
    }

    /// Fuerza un backend específico
    pub fn force_backend(mut self, backend: ComputeBackend) -> Self {
        self.forced_backend = Some(backend);
        self
    }

    /// Selecciona el mejor backend para una operación
    pub fn select(&self, _op: &str, size: usize) -> ComputeBackend {
        // Si hay backend forzado, usarlo
        if let Some(backend) = self.forced_backend {
            return backend;
        }

        // Verificar si GPU es mejor opción
        if size >= self.gpu_threshold {
            if let Some(gpu) = &self.gpu {
                // Preferir CUDA para NVIDIA, Vulkan para otros
                match gpu.vendor {
                    GPUVendor::NVIDIA if gpu.cuda_available => {
                        return ComputeBackend::GpuCuda;
                    }
                    _ if gpu.vulkan_available => {
                        return ComputeBackend::GpuVulkan;
                    }
                    _ => {}
                }
            }
        }

        // Usar mejor backend CPU disponible
        ComputeBackend::best_cpu(&self.cpu)
    }

    /// Selecciona backend para MatMul específicamente
    pub fn select_matmul(&self, m: usize, n: usize, k: usize) -> ComputeBackend {
        let flops = 2 * m * n * k;

        // MatMul se beneficia mucho de GPU para matrices grandes
        if flops >= self.gpu_threshold / 2 {
            if let Some(gpu) = &self.gpu {
                if gpu.cuda_available {
                    return ComputeBackend::GpuCuda;
                }
                if gpu.vulkan_available {
                    return ComputeBackend::GpuVulkan;
                }
            }
        }

        // CPU con mejor SIMD
        if self.cpu.has_avx512f {
            ComputeBackend::CpuAvx512
        } else if self.cpu.has_avx2 && self.cpu.has_fma {
            ComputeBackend::CpuAvx2
        } else if self.cpu.has_avx {
            ComputeBackend::CpuAvx
        } else {
            ComputeBackend::CpuSse2
        }
    }

    /// Obtiene información del sistema
    pub fn system_info(&self) -> SystemInfo {
        SystemInfo {
            cpu_vendor: self.cpu.vendor.clone(),
            cpu_brand: self.cpu.brand.clone(),
            cpu_cores: self.cpu.cores,
            cpu_threads: self.cpu.threads,
            cpu_simd: self.cpu.best_simd_name().to_string(),
            cpu_simd_width: self.cpu.best_simd_width(),
            gpu_available: self.gpu.is_some(),
            gpu_name: self
                .gpu
                .as_ref()
                .map(|g| g.device_name.clone())
                .unwrap_or_default(),
            gpu_vendor: self
                .gpu
                .as_ref()
                .map(|g| g.vendor.name().to_string())
                .unwrap_or_default(),
            gpu_vram_mb: self.gpu.as_ref().map(|g| g.vram_mb).unwrap_or(0),
            vulkan_available: self
                .gpu
                .as_ref()
                .map(|g| g.vulkan_available)
                .unwrap_or(false),
            cuda_available: self.gpu.as_ref().map(|g| g.cuda_available).unwrap_or(false),
        }
    }

    /// Imprime resumen del sistema
    pub fn print_summary(&self) {
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║              ADead-BIB AUTO-DISPATCHER                      ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!(
            "║ CPU: {:52} ║",
            &self.cpu.brand[..self.cpu.brand.len().min(52)]
        );
        println!(
            "║   Cores: {:3} | Threads: {:3} | SIMD: {:10} ({:3}-bit)  ║",
            self.cpu.cores,
            self.cpu.threads,
            self.cpu.best_simd_name(),
            self.cpu.best_simd_width()
        );
        println!("╠════════════════════════════════════════════════════════════╣");

        if let Some(gpu) = &self.gpu {
            println!(
                "║ GPU: {:52} ║",
                &gpu.device_name[..gpu.device_name.len().min(52)]
            );
            println!(
                "║   VRAM: {:5} MB | Vulkan: {} | CUDA: {}                   ║",
                gpu.vram_mb,
                if gpu.vulkan_available { "✓" } else { "✗" },
                if gpu.cuda_available { "✓" } else { "✗" }
            );
        } else {
            println!("║ GPU: Not available                                         ║");
        }

        println!("╠════════════════════════════════════════════════════════════╣");
        println!(
            "║ GPU Threshold: {:10} elements                          ║",
            self.gpu_threshold
        );
        println!(
            "║ Best CPU Backend: {:40} ║",
            format!("{:?}", ComputeBackend::best_cpu(&self.cpu))
        );
        println!("╚════════════════════════════════════════════════════════════╝");
    }
}

impl Default for AutoDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Información del sistema
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub cpu_vendor: String,
    pub cpu_brand: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub cpu_simd: String,
    pub cpu_simd_width: u32,
    pub gpu_available: bool,
    pub gpu_name: String,
    pub gpu_vendor: String,
    pub gpu_vram_mb: u32,
    pub vulkan_available: bool,
    pub cuda_available: bool,
}

/// Estimador de rendimiento para selección de backend
pub struct PerformanceEstimator;

impl PerformanceEstimator {
    /// Estima GFLOPS para MatMul en CPU
    pub fn estimate_cpu_matmul_gflops(cpu: &CPUFeatures, m: usize, n: usize, k: usize) -> f64 {
        let base_gflops = if cpu.has_avx512f {
            200.0
        } else if cpu.has_avx2 && cpu.has_fma {
            100.0
        } else if cpu.has_avx {
            50.0
        } else {
            10.0
        };

        // Ajustar por tamaño (cache effects)
        let size = m * n * k;
        let cache_factor = if size < 64 * 64 * 64 {
            1.2
        } else if size < 256 * 256 * 256 {
            1.0
        } else {
            0.7
        };

        base_gflops * cache_factor * (cpu.cores as f64)
    }

    /// Estima GFLOPS para MatMul en GPU
    pub fn estimate_gpu_matmul_gflops(gpu: &GPUFeatures, m: usize, n: usize, k: usize) -> f64 {
        let base_gflops = match gpu.vendor {
            GPUVendor::NVIDIA => 5000.0, // RTX 3060 ~12 TFLOPS
            GPUVendor::AMD => 4000.0,
            GPUVendor::Intel => 2000.0,
            GPUVendor::Unknown => 1000.0,
        };

        // Ajustar por tamaño (overhead de transferencia)
        let size = m * n * k;
        let transfer_factor = if size < 1024 * 1024 {
            0.1
        }
        // Muy pequeño, overhead domina
        else if size < 10 * 1024 * 1024 {
            0.5
        } else {
            0.9
        }; // Grande, GPU brilla

        base_gflops * transfer_factor
    }

    /// Decide si usar GPU basado en estimación de rendimiento
    pub fn should_use_gpu(
        cpu: &CPUFeatures,
        gpu: &GPUFeatures,
        m: usize,
        n: usize,
        k: usize,
    ) -> bool {
        let cpu_gflops = Self::estimate_cpu_matmul_gflops(cpu, m, n, k);
        let gpu_gflops = Self::estimate_gpu_matmul_gflops(gpu, m, n, k);

        gpu_gflops > cpu_gflops * 1.5 // GPU debe ser 1.5x mejor para justificar overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_creation() {
        let dispatcher = AutoDispatcher::cpu_only();
        assert!(dispatcher.gpu.is_none());
    }

    #[test]
    fn test_dispatcher_deterministic() {
        let d1 = AutoDispatcher::cpu_only();
        let d2 = AutoDispatcher::cpu_only();

        let backend1 = d1.select("matmul", 1000);
        let backend2 = d2.select("matmul", 1000);

        assert_eq!(backend1, backend2);
    }

    #[test]
    fn test_select_matmul() {
        let dispatcher = AutoDispatcher::cpu_only();
        let backend = dispatcher.select_matmul(1024, 1024, 1024);

        // Debe seleccionar un backend CPU válido
        assert!(matches!(
            backend,
            ComputeBackend::CpuAvx512
                | ComputeBackend::CpuAvx2
                | ComputeBackend::CpuAvx
                | ComputeBackend::CpuSse2
                | ComputeBackend::CpuScalar
        ));
    }
}
