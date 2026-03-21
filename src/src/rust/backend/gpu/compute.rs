// ============================================================
// ADead-BIB - Unified Compute API
// ============================================================
// API unificada para computación paralela que abstrae:
// - CUDA (NVIDIA) - Tu RTX 3060
// - HIP-CPU (Fallback CPU con SIMD)
// - Vulkan Compute (Portable)
//
// Filosofía: Escribes una vez, corre en cualquier backend
// ============================================================

use super::hip::{detect_hip_backend, get_device_info, HipBackend, HipDeviceInfo};
use super::hip::{Dim3, HipCpuConfig, HipCpuRuntime, SendPtr, ThreadIdx};

/// Backend de compute seleccionado
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeBackend {
    /// CUDA nativo (NVIDIA)
    Cuda,
    /// HIP-CPU (fallback paralelo en CPU)
    HipCpu,
    /// Vulkan Compute
    Vulkan,
    /// CPU secuencial (último fallback)
    CpuSequential,
}

impl ComputeBackend {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cuda => "CUDA",
            Self::HipCpu => "HIP-CPU",
            Self::Vulkan => "Vulkan",
            Self::CpuSequential => "CPU (Sequential)",
        }
    }

    /// Detecta el mejor backend disponible
    pub fn detect_best() -> Self {
        let hip_backend = detect_hip_backend();

        match hip_backend {
            HipBackend::Cuda => Self::Cuda,
            HipBackend::Rocm => Self::Cuda, // ROCm usa misma API
            HipBackend::Cpu => Self::HipCpu,
            HipBackend::None => Self::CpuSequential,
        }
    }
}

/// Configuración del runtime de compute
#[derive(Debug, Clone)]
pub struct ComputeConfig {
    /// Backend preferido (None = auto-detect)
    pub preferred_backend: Option<ComputeBackend>,
    /// Número de threads para HIP-CPU
    pub cpu_threads: usize,
    /// Habilitar SIMD en CPU
    pub enable_simd: bool,
    /// Verbose logging
    pub verbose: bool,
    /// Tamaño de bloque por defecto
    pub default_block_size: (u32, u32, u32),
}

impl Default for ComputeConfig {
    fn default() -> Self {
        Self {
            preferred_backend: None,
            cpu_threads: 0, // Auto
            enable_simd: true,
            verbose: false,
            default_block_size: (256, 1, 1),
        }
    }
}

/// Runtime unificado de compute
pub struct ComputeRuntime {
    backend: ComputeBackend,
    config: ComputeConfig,
    hip_cpu: HipCpuRuntime,
    device_info: HipDeviceInfo,
}

impl ComputeRuntime {
    /// Crea un nuevo runtime con auto-detección
    pub fn new() -> Self {
        Self::with_config(ComputeConfig::default())
    }

    /// Crea un runtime con configuración específica
    pub fn with_config(config: ComputeConfig) -> Self {
        let backend = config
            .preferred_backend
            .unwrap_or_else(ComputeBackend::detect_best);

        let hip_config = HipCpuConfig {
            num_threads: config.cpu_threads,
            enable_simd: config.enable_simd,
            block_size: config.default_block_size,
            verbose: config.verbose,
        };

        let hip_cpu = HipCpuRuntime::new(hip_config);
        let device_info = get_device_info();

        if config.verbose {
            println!("[Compute] Backend: {}", backend.name());
            println!("[Compute] Device: {}", device_info.device_name);
        }

        Self {
            backend,
            config,
            hip_cpu,
            device_info,
        }
    }

    /// Fuerza un backend específico
    pub fn with_backend(backend: ComputeBackend) -> Self {
        let mut config = ComputeConfig::default();
        config.preferred_backend = Some(backend);
        Self::with_config(config)
    }

    /// Obtiene el backend actual
    pub fn backend(&self) -> ComputeBackend {
        self.backend
    }

    /// Obtiene información del dispositivo
    pub fn device_info(&self) -> &HipDeviceInfo {
        &self.device_info
    }

    // ========================================
    // API de Alto Nivel
    // ========================================

    /// Ejecuta una operación paralela sobre un rango
    ///
    /// # Ejemplo
    /// ```ignore
    /// runtime.parallel_for(1000, |i| {
    ///     result[i] = a[i] + b[i];
    /// });
    /// ```
    pub fn parallel_for<F>(&self, n: usize, kernel: F)
    where
        F: Fn(usize) + Sync + Send,
    {
        match self.backend {
            ComputeBackend::Cuda => {
                // Para CUDA real, generaríamos código y lo ejecutaríamos
                // Por ahora, fallback a HIP-CPU
                self.hip_cpu.parallel_for(n, kernel);
            }
            ComputeBackend::HipCpu | ComputeBackend::CpuSequential => {
                self.hip_cpu.parallel_for(n, kernel);
            }
            ComputeBackend::Vulkan => {
                // Vulkan compute requiere más setup
                self.hip_cpu.parallel_for(n, kernel);
            }
        }
    }

    /// Lanza un kernel con dimensiones grid/block
    pub fn launch<F>(&self, grid: impl Into<Dim3>, block: impl Into<Dim3>, kernel: F)
    where
        F: Fn(ThreadIdx) + Sync + Send,
    {
        let grid = grid.into();
        let block = block.into();

        match self.backend {
            ComputeBackend::Cuda => {
                // CUDA real usaría nvcc
                self.hip_cpu.launch_kernel(grid, block, kernel);
            }
            _ => {
                self.hip_cpu.launch_kernel(grid, block, kernel);
            }
        }
    }

    // ========================================
    // Operaciones Vectoriales
    // ========================================

    /// Vector Add: C = A + B
    pub fn vector_add(&self, a: &[f32], b: &[f32], c: &mut [f32]) {
        assert_eq!(a.len(), b.len());
        assert_eq!(a.len(), c.len());

        match self.backend {
            ComputeBackend::Cuda => {
                // TODO: CUDA nativo
                self.hip_cpu.vector_add(a, b, c);
            }
            _ => {
                self.hip_cpu.vector_add(a, b, c);
            }
        }
    }

    /// SAXPY: y = alpha * x + y
    pub fn saxpy(&self, alpha: f32, x: &[f32], y: &mut [f32]) {
        assert_eq!(x.len(), y.len());

        match self.backend {
            ComputeBackend::Cuda => {
                self.hip_cpu.saxpy(alpha, x, y);
            }
            _ => {
                self.hip_cpu.saxpy(alpha, x, y);
            }
        }
    }

    /// Vector Scale: y = alpha * x
    pub fn vector_scale(&self, alpha: f32, x: &[f32], y: &mut [f32]) {
        assert_eq!(x.len(), y.len());
        let n = x.len();

        self.parallel_for(n, |i| unsafe {
            *y.as_ptr().add(i).cast_mut() = alpha * *x.get_unchecked(i);
        });
    }

    /// Dot Product: result = sum(a[i] * b[i])
    pub fn dot_product(&self, a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        let n = a.len();

        // Producto elemento a elemento
        let mut products = vec![0.0f32; n];
        let products_ptr = SendPtr::new(products.as_mut_ptr());
        let a_ptr = SendPtr::from_const(a.as_ptr());
        let b_ptr = SendPtr::from_const(b.as_ptr());

        self.parallel_for(n, |i| unsafe {
            products_ptr.write(i, a_ptr.read(i) * b_ptr.read(i));
        });

        // Reducción
        self.hip_cpu.reduce_sum(&products)
    }

    // ========================================
    // Operaciones de Matrices
    // ========================================

    /// Matrix Multiply: C = A * B
    /// A: m x k, B: k x n, C: m x n
    pub fn matmul(&self, a: &[f32], b: &[f32], c: &mut [f32], m: usize, n: usize, k: usize) {
        assert_eq!(a.len(), m * k);
        assert_eq!(b.len(), k * n);
        assert_eq!(c.len(), m * n);

        match self.backend {
            ComputeBackend::Cuda => {
                // TODO: cuBLAS
                self.hip_cpu.matmul_tiled(a, b, c, m, n, k);
            }
            _ => {
                self.hip_cpu.matmul_tiled(a, b, c, m, n, k);
            }
        }
    }

    /// Matrix Transpose: B = A^T
    pub fn transpose(&self, a: &[f32], b: &mut [f32], rows: usize, cols: usize) {
        assert_eq!(a.len(), rows * cols);
        assert_eq!(b.len(), rows * cols);

        let a_ptr = SendPtr::from_const(a.as_ptr());
        let b_ptr = SendPtr::new(b.as_mut_ptr());

        self.parallel_for(rows, |row| {
            for col in 0..cols {
                unsafe {
                    b_ptr.write(col * rows + row, a_ptr.read(row * cols + col));
                }
            }
        });
    }

    // ========================================
    // Reducciones
    // ========================================

    /// Reduce Sum
    pub fn reduce_sum(&self, data: &[f32]) -> f32 {
        self.hip_cpu.reduce_sum(data)
    }

    /// Reduce Max
    pub fn reduce_max(&self, data: &[f32]) -> f32 {
        if data.is_empty() {
            return f32::NEG_INFINITY;
        }

        let num_threads = self.config.cpu_threads.max(1);
        let chunk_size = (data.len() + num_threads - 1) / num_threads;

        let partial_maxs: Vec<f32> = std::thread::scope(|s| {
            data.chunks(chunk_size)
                .map(|chunk| {
                    s.spawn(move || chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max))
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect()
        });

        partial_maxs
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Reduce Min
    pub fn reduce_min(&self, data: &[f32]) -> f32 {
        if data.is_empty() {
            return f32::INFINITY;
        }

        let num_threads = self.config.cpu_threads.max(1);
        let chunk_size = (data.len() + num_threads - 1) / num_threads;

        let partial_mins: Vec<f32> = std::thread::scope(|s| {
            data.chunks(chunk_size)
                .map(|chunk| s.spawn(move || chunk.iter().cloned().fold(f32::INFINITY, f32::min)))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect()
        });

        partial_mins.iter().cloned().fold(f32::INFINITY, f32::min)
    }

    // ========================================
    // Utilidades
    // ========================================

    /// Imprime información del runtime
    pub fn print_info(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║              ADead-BIB Compute Runtime                        ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Backend:     {:<48} ║", self.backend.name());
        println!(
            "║ Device:      {:<48} ║",
            if self.device_info.device_name.len() > 48 {
                &self.device_info.device_name[..48]
            } else {
                &self.device_info.device_name
            }
        );
        println!(
            "║ Memory:      {} MB                                         ║",
            self.device_info.total_memory_mb
        );
        println!(
            "║ Compute:     {}.{}                                             ║",
            self.device_info.compute_capability.0, self.device_info.compute_capability.1
        );
        println!(
            "║ SIMD:        {}                                              ║",
            if self.config.enable_simd { "ON" } else { "OFF" }
        );
        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    /// Benchmark simple
    pub fn benchmark(&self) -> BenchmarkResults {
        use std::time::Instant;

        let n = 1_000_000;
        let a: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..n).map(|i| (i * 2) as f32).collect();
        let mut c = vec![0.0f32; n];

        // Vector Add
        let start = Instant::now();
        for _ in 0..10 {
            self.vector_add(&a, &b, &mut c);
        }
        let vector_add_time = start.elapsed().as_secs_f64() / 10.0;

        // SAXPY
        let mut y = b.clone();
        let start = Instant::now();
        for _ in 0..10 {
            self.saxpy(2.5, &a, &mut y);
        }
        let saxpy_time = start.elapsed().as_secs_f64() / 10.0;

        // Reduce
        let start = Instant::now();
        let mut sum = 0.0f32;
        for _ in 0..10 {
            sum = self.reduce_sum(&a);
        }
        let reduce_time = start.elapsed().as_secs_f64() / 10.0;

        // MatMul (smaller)
        let m = 256;
        let mat_a = vec![1.0f32; m * m];
        let mat_b = vec![2.0f32; m * m];
        let mut mat_c = vec![0.0f32; m * m];

        let start = Instant::now();
        self.matmul(&mat_a, &mat_b, &mut mat_c, m, m, m);
        let matmul_time = start.elapsed().as_secs_f64();

        BenchmarkResults {
            backend: self.backend,
            vector_add_ms: vector_add_time * 1000.0,
            saxpy_ms: saxpy_time * 1000.0,
            reduce_ms: reduce_time * 1000.0,
            matmul_256_ms: matmul_time * 1000.0,
            elements: n,
        }
    }
}

impl Default for ComputeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Resultados de benchmark
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub backend: ComputeBackend,
    pub vector_add_ms: f64,
    pub saxpy_ms: f64,
    pub reduce_ms: f64,
    pub matmul_256_ms: f64,
    pub elements: usize,
}

impl std::fmt::Display for BenchmarkResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "╔══════════════════════════════════════════════════════════════╗"
        )?;
        writeln!(
            f,
            "║              ADead-BIB Compute Benchmark                      ║"
        )?;
        writeln!(
            f,
            "╠══════════════════════════════════════════════════════════════╣"
        )?;
        writeln!(f, "║ Backend:      {:<47} ║", self.backend.name())?;
        writeln!(f, "║ Elements:     {:<47} ║", self.elements)?;
        writeln!(
            f,
            "╠══════════════════════════════════════════════════════════════╣"
        )?;
        writeln!(
            f,
            "║ Vector Add:   {:>10.3} ms                                  ║",
            self.vector_add_ms
        )?;
        writeln!(
            f,
            "║ SAXPY:        {:>10.3} ms                                  ║",
            self.saxpy_ms
        )?;
        writeln!(
            f,
            "║ Reduce Sum:   {:>10.3} ms                                  ║",
            self.reduce_ms
        )?;
        writeln!(
            f,
            "║ MatMul 256²:  {:>10.3} ms                                  ║",
            self.matmul_256_ms
        )?;
        writeln!(
            f,
            "╚══════════════════════════════════════════════════════════════╝"
        )?;
        Ok(())
    }
}

// ========================================
// Funciones de conveniencia (API global)
// ========================================

/// Crea un runtime con auto-detección
pub fn create_runtime() -> ComputeRuntime {
    ComputeRuntime::new()
}

/// Crea un runtime forzando HIP-CPU
pub fn create_cpu_runtime() -> ComputeRuntime {
    ComputeRuntime::with_backend(ComputeBackend::HipCpu)
}

/// Detecta el mejor backend disponible
pub fn detect_backend() -> ComputeBackend {
    ComputeBackend::detect_best()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_runtime() {
        let runtime = ComputeRuntime::new();
        assert!(
            runtime.backend() != ComputeBackend::CpuSequential
                || runtime.backend() == ComputeBackend::HipCpu
        );
    }

    #[test]
    fn test_vector_add() {
        let runtime = ComputeRuntime::new();
        let n = 1000;

        let a: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..n).map(|i| i as f32 * 2.0).collect();
        let mut c = vec![0.0f32; n];

        runtime.vector_add(&a, &b, &mut c);

        for i in 0..n {
            assert!((c[i] - (a[i] + b[i])).abs() < 1e-6);
        }
    }

    #[test]
    fn test_saxpy() {
        let runtime = ComputeRuntime::new();
        let n = 1000;
        let alpha = 2.5f32;

        let x: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let mut y: Vec<f32> = (0..n).map(|i| i as f32 * 0.5).collect();
        let y_orig = y.clone();

        runtime.saxpy(alpha, &x, &mut y);

        for i in 0..n {
            let expected = alpha * x[i] + y_orig[i];
            assert!((y[i] - expected).abs() < 1e-5);
        }
    }

    #[test]
    fn test_dot_product() {
        let runtime = ComputeRuntime::new();

        let a = vec![1.0f32, 2.0, 3.0, 4.0];
        let b = vec![1.0f32, 1.0, 1.0, 1.0];

        let result = runtime.dot_product(&a, &b);
        assert!((result - 10.0).abs() < 1e-5);
    }

    #[test]
    fn test_reduce() {
        let runtime = ComputeRuntime::new();

        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();

        let sum = runtime.reduce_sum(&data);
        let expected: f32 = (0..100).map(|i| i as f32).sum();
        assert!((sum - expected).abs() < 1e-2);

        let max = runtime.reduce_max(&data);
        assert!((max - 99.0).abs() < 1e-5);

        let min = runtime.reduce_min(&data);
        assert!((min - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_matmul() {
        let runtime = ComputeRuntime::new();
        let m = 32;

        let a = vec![1.0f32; m * m];
        let b = vec![2.0f32; m * m];
        let mut c = vec![0.0f32; m * m];

        runtime.matmul(&a, &b, &mut c, m, m, m);

        // Cada elemento debería ser m * 1.0 * 2.0 = 2m
        let expected = (m as f32) * 2.0;
        for val in &c {
            assert!((*val - expected).abs() < 1e-3);
        }
    }

    #[test]
    fn test_transpose() {
        let runtime = ComputeRuntime::new();

        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut b = vec![0.0f32; 6];

        runtime.transpose(&a, &mut b, 2, 3);

        // Transpuesta de 2x3 es 3x2
        assert!((b[0] - 1.0).abs() < 1e-5); // (0,0)
        assert!((b[1] - 4.0).abs() < 1e-5); // (1,0)
        assert!((b[2] - 2.0).abs() < 1e-5); // (0,1)
        assert!((b[3] - 5.0).abs() < 1e-5); // (1,1)
        assert!((b[4] - 3.0).abs() < 1e-5); // (0,2)
        assert!((b[5] - 6.0).abs() < 1e-5); // (1,2)
    }
}
