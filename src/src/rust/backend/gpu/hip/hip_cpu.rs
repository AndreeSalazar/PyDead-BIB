// ============================================================
// ADead-BIB - HIP-CPU Backend
// ============================================================
// Ejecuta kernels estilo CUDA/HIP en CPU usando:
// - Paralelismo con threads (rayon-style)
// - SIMD cuando es posible (AVX2/AVX512)
// - Fallback seguro cuando no hay GPU
//
// Esto permite:
// 1. Debugging de kernels sin GPU
// 2. Ejecución en máquinas sin GPU
// 3. Comparación CPU vs GPU para benchmarks
// ============================================================

use std::sync::atomic::{AtomicUsize, Ordering};

/// Wrapper para punteros raw que implementa Send + Sync
/// SAFETY: El usuario debe garantizar que el acceso es seguro
#[derive(Clone, Copy)]
pub struct SendPtr<T>(*mut T);

unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

impl<T> SendPtr<T> {
    pub fn new(ptr: *mut T) -> Self {
        Self(ptr)
    }

    pub fn from_const(ptr: *const T) -> Self {
        Self(ptr as *mut T)
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0
    }

    pub unsafe fn add(&self, offset: usize) -> *mut T {
        self.0.add(offset)
    }

    pub unsafe fn read(&self, offset: usize) -> T
    where
        T: Copy,
    {
        *self.0.add(offset)
    }

    pub unsafe fn write(&self, offset: usize, value: T) {
        *self.0.add(offset) = value;
    }
}

/// Configuración de ejecución HIP-CPU
#[derive(Debug, Clone)]
pub struct HipCpuConfig {
    /// Número de threads a usar (0 = auto-detect)
    pub num_threads: usize,
    /// Habilitar SIMD (AVX2/AVX512)
    pub enable_simd: bool,
    /// Tamaño de bloque simulado
    pub block_size: (u32, u32, u32),
    /// Verbose logging
    pub verbose: bool,
}

impl Default for HipCpuConfig {
    fn default() -> Self {
        Self {
            num_threads: 0, // Auto-detect
            block_size: (256, 1, 1),
            enable_simd: true,
            verbose: false,
        }
    }
}

impl HipCpuConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threads(mut self, n: usize) -> Self {
        self.num_threads = n;
        self
    }

    pub fn with_block_size(mut self, x: u32, y: u32, z: u32) -> Self {
        self.block_size = (x, y, z);
        self
    }

    /// Obtiene el número real de threads a usar
    pub fn effective_threads(&self) -> usize {
        if self.num_threads == 0 {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4)
        } else {
            self.num_threads
        }
    }
}

/// Dimensiones de grid/block (compatible con CUDA/HIP)
#[derive(Debug, Clone, Copy)]
pub struct Dim3 {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Dim3 {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    pub fn linear(n: u32) -> Self {
        Self { x: n, y: 1, z: 1 }
    }

    pub fn total(&self) -> u32 {
        self.x * self.y * self.z
    }
}

impl From<(u32, u32, u32)> for Dim3 {
    fn from((x, y, z): (u32, u32, u32)) -> Self {
        Self { x, y, z }
    }
}

impl From<u32> for Dim3 {
    fn from(x: u32) -> Self {
        Self { x, y: 1, z: 1 }
    }
}

/// Índices de thread (simula threadIdx, blockIdx, etc.)
#[derive(Debug, Clone, Copy)]
pub struct ThreadIdx {
    pub thread_idx: Dim3,
    pub block_idx: Dim3,
    pub block_dim: Dim3,
    pub grid_dim: Dim3,
}

impl ThreadIdx {
    /// Calcula el índice global lineal (como en CUDA)
    pub fn global_idx_x(&self) -> u32 {
        self.block_idx.x * self.block_dim.x + self.thread_idx.x
    }

    pub fn global_idx_y(&self) -> u32 {
        self.block_idx.y * self.block_dim.y + self.thread_idx.y
    }

    pub fn global_idx_z(&self) -> u32 {
        self.block_idx.z * self.block_dim.z + self.thread_idx.z
    }

    /// Índice global lineal 1D
    pub fn global_id(&self) -> u32 {
        let gx = self.global_idx_x();
        let gy = self.global_idx_y();
        let gz = self.global_idx_z();
        let width = self.grid_dim.x * self.block_dim.x;
        let height = self.grid_dim.y * self.block_dim.y;
        gz * width * height + gy * width + gx
    }
}

/// Runtime HIP-CPU para ejecutar kernels en CPU
pub struct HipCpuRuntime {
    config: HipCpuConfig,
    /// Contador de kernels ejecutados
    kernel_count: AtomicUsize,
}

impl HipCpuRuntime {
    pub fn new(config: HipCpuConfig) -> Self {
        Self {
            config,
            kernel_count: AtomicUsize::new(0),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(HipCpuConfig::default())
    }

    /// Ejecuta un kernel paralelo 1D (estilo parallel_for)
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
        let num_threads = self.config.effective_threads();
        let chunk_size = (n + num_threads - 1) / num_threads;

        if self.config.verbose {
            println!(
                "[HIP-CPU] parallel_for: n={}, threads={}, chunk={}",
                n, num_threads, chunk_size
            );
        }

        std::thread::scope(|s| {
            for t in 0..num_threads {
                let start = t * chunk_size;
                let end = std::cmp::min(start + chunk_size, n);
                let kernel_ref = &kernel;

                s.spawn(move || {
                    for i in start..end {
                        kernel_ref(i);
                    }
                });
            }
        });

        self.kernel_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Ejecuta un kernel con grid/block dimensions (estilo CUDA)
    ///
    /// # Ejemplo
    /// ```ignore
    /// runtime.launch_kernel(
    ///     Dim3::new(grid_x, 1, 1),
    ///     Dim3::new(256, 1, 1),
    ///     |idx| {
    ///         let i = idx.global_idx_x() as usize;
    ///         if i < n {
    ///             result[i] = a[i] + b[i];
    ///         }
    ///     }
    /// );
    /// ```
    pub fn launch_kernel<F>(&self, grid_dim: Dim3, block_dim: Dim3, kernel: F)
    where
        F: Fn(ThreadIdx) + Sync + Send,
    {
        let total_blocks = grid_dim.total() as usize;
        let _threads_per_block = block_dim.total() as usize;
        let num_threads = self.config.effective_threads();

        if self.config.verbose {
            println!(
                "[HIP-CPU] launch_kernel: grid=({},{},{}), block=({},{},{})",
                grid_dim.x, grid_dim.y, grid_dim.z, block_dim.x, block_dim.y, block_dim.z
            );
        }

        // Paralelizar por bloques
        let blocks_per_thread = (total_blocks + num_threads - 1) / num_threads;

        std::thread::scope(|s| {
            for t in 0..num_threads {
                let start_block = t * blocks_per_thread;
                let end_block = std::cmp::min(start_block + blocks_per_thread, total_blocks);
                let kernel_ref = &kernel;

                s.spawn(move || {
                    for block_linear in start_block..end_block {
                        // Convertir índice lineal a 3D
                        let bz = block_linear / (grid_dim.x * grid_dim.y) as usize;
                        let by = (block_linear / grid_dim.x as usize) % grid_dim.y as usize;
                        let bx = block_linear % grid_dim.x as usize;

                        let block_idx = Dim3::new(bx as u32, by as u32, bz as u32);

                        // Ejecutar todos los threads del bloque
                        for tz in 0..block_dim.z {
                            for ty in 0..block_dim.y {
                                for tx in 0..block_dim.x {
                                    let idx = ThreadIdx {
                                        thread_idx: Dim3::new(tx, ty, tz),
                                        block_idx,
                                        block_dim,
                                        grid_dim,
                                    };
                                    kernel_ref(idx);
                                }
                            }
                        }
                    }
                });
            }
        });

        self.kernel_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Vector Add (optimizado con SIMD cuando es posible)
    pub fn vector_add(&self, a: &[f32], b: &[f32], c: &mut [f32]) {
        assert_eq!(a.len(), b.len());
        assert_eq!(a.len(), c.len());

        let n = a.len();

        if self.config.enable_simd {
            self.vector_add_simd(a, b, c);
        } else {
            let c_ptr = SendPtr::new(c.as_mut_ptr());
            let a_ptr = SendPtr::from_const(a.as_ptr());
            let b_ptr = SendPtr::from_const(b.as_ptr());

            self.parallel_for(n, |i| {
                // SAFETY: índices validados por parallel_for
                unsafe {
                    c_ptr.write(i, a_ptr.read(i) + b_ptr.read(i));
                }
            });
        }
    }

    /// Vector Add con SIMD (AVX2 cuando disponible)
    #[cfg(target_arch = "x86_64")]
    fn vector_add_simd(&self, a: &[f32], b: &[f32], c: &mut [f32]) {
        let n = a.len();
        let num_threads = self.config.effective_threads();
        let chunk_size = (n + num_threads - 1) / num_threads;

        std::thread::scope(|s| {
            let a_chunks = a.chunks(chunk_size);
            let b_chunks = b.chunks(chunk_size);
            let c_chunks = c.chunks_mut(chunk_size);

            for ((a_chunk, b_chunk), c_chunk) in a_chunks.zip(b_chunks).zip(c_chunks) {
                s.spawn(move || {
                    // Procesar en grupos de 8 (AVX2 = 256 bits = 8 floats)
                    let simd_len = a_chunk.len() / 8 * 8;

                    // Parte SIMD
                    for i in (0..simd_len).step_by(8) {
                        // Sin intrinsics explícitos, el compilador auto-vectoriza
                        for j in 0..8 {
                            c_chunk[i + j] = a_chunk[i + j] + b_chunk[i + j];
                        }
                    }

                    // Resto escalar
                    for i in simd_len..a_chunk.len() {
                        c_chunk[i] = a_chunk[i] + b_chunk[i];
                    }
                });
            }
        });
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn vector_add_simd(&self, a: &[f32], b: &[f32], c: &mut [f32]) {
        // Fallback sin SIMD
        let n = a.len();
        self.parallel_for(n, |i| {
            c[i] = a[i] + b[i];
        });
    }

    /// SAXPY: y = a * x + y
    pub fn saxpy(&self, alpha: f32, x: &[f32], y: &mut [f32]) {
        assert_eq!(x.len(), y.len());
        let n = x.len();

        let x_ptr = SendPtr::from_const(x.as_ptr());
        let y_ptr = SendPtr::new(y.as_mut_ptr());

        self.parallel_for(n, |i| unsafe {
            let xi = x_ptr.read(i);
            let yi = y_ptr.read(i);
            y_ptr.write(i, alpha * xi + yi);
        });
    }

    /// Matrix Multiply (C = A * B) - Naive pero paralelo
    pub fn matmul(&self, a: &[f32], b: &[f32], c: &mut [f32], m: usize, n: usize, k: usize) {
        // A: m x k, B: k x n, C: m x n
        assert_eq!(a.len(), m * k);
        assert_eq!(b.len(), k * n);
        assert_eq!(c.len(), m * n);

        let a_ptr = SendPtr::from_const(a.as_ptr());
        let b_ptr = SendPtr::from_const(b.as_ptr());
        let c_ptr = SendPtr::new(c.as_mut_ptr());

        // Paralelizar por filas de C
        self.parallel_for(m, |row| {
            for col in 0..n {
                let mut sum = 0.0f32;
                for i in 0..k {
                    unsafe {
                        sum += a_ptr.read(row * k + i) * b_ptr.read(i * n + col);
                    }
                }
                unsafe {
                    c_ptr.write(row * n + col, sum);
                }
            }
        });
    }

    /// Matrix Multiply con tiling (mejor cache locality)
    pub fn matmul_tiled(&self, a: &[f32], b: &[f32], c: &mut [f32], m: usize, n: usize, k: usize) {
        const TILE_SIZE: usize = 32;

        // Inicializar C a cero
        for x in c.iter_mut() {
            *x = 0.0;
        }

        let a_ptr = SendPtr::from_const(a.as_ptr());
        let b_ptr = SendPtr::from_const(b.as_ptr());
        let c_ptr = SendPtr::new(c.as_mut_ptr());

        // Paralelizar por tiles de filas
        let num_row_tiles = (m + TILE_SIZE - 1) / TILE_SIZE;

        self.parallel_for(num_row_tiles, |row_tile| {
            let row_start = row_tile * TILE_SIZE;
            let row_end = std::cmp::min(row_start + TILE_SIZE, m);

            for col_tile in 0..((n + TILE_SIZE - 1) / TILE_SIZE) {
                let col_start = col_tile * TILE_SIZE;
                let col_end = std::cmp::min(col_start + TILE_SIZE, n);

                for k_tile in 0..((k + TILE_SIZE - 1) / TILE_SIZE) {
                    let k_start = k_tile * TILE_SIZE;
                    let k_end = std::cmp::min(k_start + TILE_SIZE, k);

                    // Multiplicar tile
                    for row in row_start..row_end {
                        for col in col_start..col_end {
                            unsafe {
                                let mut sum = c_ptr.read(row * n + col);
                                for i in k_start..k_end {
                                    sum += a_ptr.read(row * k + i) * b_ptr.read(i * n + col);
                                }
                                c_ptr.write(row * n + col, sum);
                            }
                        }
                    }
                }
            }
        });
    }

    /// Reduce (suma paralela)
    pub fn reduce_sum(&self, data: &[f32]) -> f32 {
        let num_threads = self.config.effective_threads();
        let chunk_size = (data.len() + num_threads - 1) / num_threads;

        let partial_sums: Vec<f32> = std::thread::scope(|s| {
            data.chunks(chunk_size)
                .map(|chunk| s.spawn(move || chunk.iter().sum::<f32>()))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect()
        });

        partial_sums.iter().sum()
    }

    /// Estadísticas del runtime
    pub fn stats(&self) -> HipCpuStats {
        HipCpuStats {
            kernels_executed: self.kernel_count.load(Ordering::Relaxed),
            num_threads: self.config.effective_threads(),
            simd_enabled: self.config.enable_simd,
        }
    }
}

#[derive(Debug)]
pub struct HipCpuStats {
    pub kernels_executed: usize,
    pub num_threads: usize,
    pub simd_enabled: bool,
}

impl std::fmt::Display for HipCpuStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HIP-CPU Stats: {} kernels, {} threads, SIMD: {}",
            self.kernels_executed,
            self.num_threads,
            if self.simd_enabled { "ON" } else { "OFF" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_for() {
        let runtime = HipCpuRuntime::with_default_config();
        let n = 1000;
        let mut result = vec![0i32; n];

        // Usar SendPtr para thread safety
        let result_ptr = SendPtr::new(result.as_mut_ptr());

        runtime.parallel_for(n, |i| unsafe {
            result_ptr.write(i, i as i32 * 2);
        });

        for i in 0..n {
            assert_eq!(result[i], i as i32 * 2);
        }
    }

    #[test]
    fn test_vector_add() {
        let runtime = HipCpuRuntime::with_default_config();
        let n = 10000;

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
        let runtime = HipCpuRuntime::with_default_config();
        let n = 1000;
        let alpha = 2.5f32;

        let x: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let mut y: Vec<f32> = (0..n).map(|i| i as f32 * 0.5).collect();
        let y_orig = y.clone();

        runtime.saxpy(alpha, &x, &mut y);

        for i in 0..n {
            let expected = alpha * x[i] + y_orig[i];
            assert!((y[i] - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn test_matmul() {
        let runtime = HipCpuRuntime::with_default_config();
        let m = 64;
        let n = 64;
        let k = 64;

        let a = vec![1.0f32; m * k];
        let b = vec![2.0f32; k * n];
        let mut c = vec![0.0f32; m * n];

        runtime.matmul(&a, &b, &mut c, m, n, k);

        // Cada elemento de C debería ser k * 1.0 * 2.0 = 2k
        let expected = (k as f32) * 2.0;
        for val in &c {
            assert!((*val - expected).abs() < 1e-4);
        }
    }

    #[test]
    fn test_reduce_sum() {
        let runtime = HipCpuRuntime::with_default_config();
        let n = 1000; // Smaller n for f32 precision

        let data: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let sum = runtime.reduce_sum(&data);

        let expected: f32 = (0..n).map(|i| i as f32).sum();
        // f32 has limited precision, use relative tolerance
        let tolerance = expected.abs() * 1e-4;
        assert!(
            (sum - expected).abs() < tolerance.max(1.0),
            "sum={}, expected={}, diff={}",
            sum,
            expected,
            (sum - expected).abs()
        );
    }

    #[test]
    fn test_launch_kernel() {
        let runtime = HipCpuRuntime::with_default_config();
        let n = 1024usize;
        let mut result = vec![0i32; n];
        let result_ptr = SendPtr::new(result.as_mut_ptr());

        let grid = Dim3::new(4, 1, 1);
        let block = Dim3::new(256, 1, 1);

        runtime.launch_kernel(grid, block, |idx| {
            let i = idx.global_idx_x() as usize;
            if i < n {
                unsafe {
                    result_ptr.write(i, i as i32);
                }
            }
        });

        for i in 0..n {
            assert_eq!(result[i], i as i32);
        }
    }
}
