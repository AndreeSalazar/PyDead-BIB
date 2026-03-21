// ============================================================
// ADead-BIB - HIP Runtime (AMD ROCm compatible)
// ============================================================
// Runtime para GPUs AMD usando HIP nativo
// También soporta traducción CUDA→HIP para portabilidad
//
// En tu caso (RTX 3060), esto sirve como:
// 1. Capa de abstracción portable
// 2. Fallback a HIP-CPU cuando no hay GPU
// 3. Preparación para soporte AMD futuro
// ============================================================

use std::path::Path;
use std::process::Command;

/// Estado del runtime HIP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HipBackend {
    /// CUDA nativo (NVIDIA)
    Cuda,
    /// ROCm nativo (AMD)
    Rocm,
    /// HIP-CPU (fallback)
    Cpu,
    /// No disponible
    None,
}

impl HipBackend {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cuda => "CUDA (via HIP)",
            Self::Rocm => "ROCm",
            Self::Cpu => "HIP-CPU",
            Self::None => "None",
        }
    }
}

/// Información del dispositivo HIP
#[derive(Debug, Clone)]
pub struct HipDeviceInfo {
    pub backend: HipBackend,
    pub device_name: String,
    pub compute_capability: (u32, u32),
    pub total_memory_mb: u32,
    pub multiprocessor_count: u32,
    pub warp_size: u32,
    pub max_threads_per_block: u32,
    pub max_shared_memory_per_block: u32,
}

impl Default for HipDeviceInfo {
    fn default() -> Self {
        Self {
            backend: HipBackend::None,
            device_name: String::new(),
            compute_capability: (0, 0),
            total_memory_mb: 0,
            multiprocessor_count: 0,
            warp_size: 32,
            max_threads_per_block: 1024,
            max_shared_memory_per_block: 49152,
        }
    }
}

/// Detecta el backend HIP disponible
pub fn detect_hip_backend() -> HipBackend {
    // 1. Verificar CUDA (NVIDIA)
    if detect_cuda_available() {
        return HipBackend::Cuda;
    }

    // 2. Verificar ROCm (AMD)
    if detect_rocm_available() {
        return HipBackend::Rocm;
    }

    // 3. Fallback a HIP-CPU
    HipBackend::Cpu
}

/// Detecta si CUDA está disponible
fn detect_cuda_available() -> bool {
    #[cfg(windows)]
    {
        let system32 = std::env::var("SystemRoot")
            .map(|r| format!("{}\\System32\\nvcuda.dll", r))
            .unwrap_or_default();
        Path::new(&system32).exists()
    }
    #[cfg(not(windows))]
    {
        Path::new("/usr/lib/libcuda.so").exists()
            || Path::new("/usr/lib/x86_64-linux-gnu/libcuda.so").exists()
    }
}

/// Detecta si ROCm está disponible
fn detect_rocm_available() -> bool {
    #[cfg(windows)]
    {
        // ROCm en Windows es limitado
        Path::new("C:\\Program Files\\AMD\\ROCm").exists()
    }
    #[cfg(not(windows))]
    {
        Path::new("/opt/rocm").exists() || Path::new("/usr/lib/libamdhip64.so").exists()
    }
}

/// Obtiene información del dispositivo
pub fn get_device_info() -> HipDeviceInfo {
    let backend = detect_hip_backend();

    match backend {
        HipBackend::Cuda => get_cuda_device_info(),
        HipBackend::Rocm => get_rocm_device_info(),
        HipBackend::Cpu => get_cpu_device_info(),
        HipBackend::None => HipDeviceInfo::default(),
    }
}

fn get_cuda_device_info() -> HipDeviceInfo {
    // Usar nvidia-smi para obtener info
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let line = stdout.lines().next().unwrap_or("");
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

            let device_name = parts.first().unwrap_or(&"Unknown NVIDIA GPU").to_string();
            let memory_mb: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

            // Detectar compute capability basado en nombre
            let (cc_major, cc_minor) = detect_compute_capability(&device_name);
            let sm_count = detect_sm_count(&device_name);

            HipDeviceInfo {
                backend: HipBackend::Cuda,
                device_name,
                compute_capability: (cc_major, cc_minor),
                total_memory_mb: memory_mb,
                multiprocessor_count: sm_count,
                warp_size: 32,
                max_threads_per_block: 1024,
                max_shared_memory_per_block: if cc_major >= 8 { 163840 } else { 49152 },
            }
        }
        _ => HipDeviceInfo {
            backend: HipBackend::Cuda,
            device_name: "NVIDIA GPU (unknown)".to_string(),
            ..Default::default()
        },
    }
}

fn get_rocm_device_info() -> HipDeviceInfo {
    // Usar rocm-smi para obtener info
    let output = Command::new("rocm-smi")
        .args(["--showproductname"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let device_name = stdout
                .lines()
                .find(|l| l.contains("GPU"))
                .unwrap_or("AMD GPU")
                .to_string();

            HipDeviceInfo {
                backend: HipBackend::Rocm,
                device_name,
                compute_capability: (9, 0), // gfx9xx
                total_memory_mb: 0,         // TODO: parse from rocm-smi
                multiprocessor_count: 0,
                warp_size: 64, // AMD wavefront
                max_threads_per_block: 1024,
                max_shared_memory_per_block: 65536,
            }
        }
        _ => HipDeviceInfo {
            backend: HipBackend::Rocm,
            device_name: "AMD GPU (unknown)".to_string(),
            ..Default::default()
        },
    }
}

fn get_cpu_device_info() -> HipDeviceInfo {
    let num_cpus = std::thread::available_parallelism()
        .map(|p| p.get() as u32)
        .unwrap_or(4);

    HipDeviceInfo {
        backend: HipBackend::Cpu,
        device_name: format!("CPU ({} threads)", num_cpus),
        compute_capability: (0, 0),
        total_memory_mb: 0, // System RAM
        multiprocessor_count: num_cpus,
        warp_size: 1, // No SIMD grouping
        max_threads_per_block: num_cpus * 256,
        max_shared_memory_per_block: 1024 * 1024, // 1MB
    }
}

fn detect_compute_capability(name: &str) -> (u32, u32) {
    let name_lower = name.to_lowercase();

    if name_lower.contains("4090")
        || name_lower.contains("4080")
        || name_lower.contains("4070")
        || name_lower.contains("4060")
    {
        (8, 9) // Ada Lovelace
    } else if name_lower.contains("3090")
        || name_lower.contains("3080")
        || name_lower.contains("3070")
        || name_lower.contains("3060")
        || name_lower.contains("3050")
    {
        (8, 6) // Ampere
    } else if name_lower.contains("2080")
        || name_lower.contains("2070")
        || name_lower.contains("2060")
    {
        (7, 5) // Turing
    } else if name_lower.contains("1080")
        || name_lower.contains("1070")
        || name_lower.contains("1060")
    {
        (6, 1) // Pascal
    } else {
        (5, 0) // Default
    }
}

fn detect_sm_count(name: &str) -> u32 {
    let name_lower = name.to_lowercase();

    // RTX 30 Series
    if name_lower.contains("3090") {
        82
    } else if name_lower.contains("3080") {
        68
    } else if name_lower.contains("3070") {
        46
    } else if name_lower.contains("3060") {
        28
    }
    // RTX 40 Series
    else if name_lower.contains("4090") {
        128
    } else if name_lower.contains("4080") {
        76
    } else if name_lower.contains("4070") {
        46
    } else {
        0
    }
}

/// Generador de código HIP portable
pub struct HipCodeGen {
    target: HipBackend,
    kernels: Vec<HipKernel>,
}

#[derive(Debug, Clone)]
pub struct HipKernel {
    pub name: String,
    pub params: Vec<(String, String)>, // (name, type)
    pub body: String,
    pub block_size: (u32, u32, u32),
}

impl HipCodeGen {
    pub fn new(target: HipBackend) -> Self {
        Self {
            target,
            kernels: Vec::new(),
        }
    }

    pub fn auto_detect() -> Self {
        Self::new(detect_hip_backend())
    }

    /// Añade un kernel
    pub fn add_kernel(&mut self, kernel: HipKernel) {
        self.kernels.push(kernel);
    }

    /// Genera código HIP/CUDA
    pub fn generate(&self) -> String {
        let mut code = String::new();

        // Header
        code.push_str("// Generated by ADead-BIB HIP Backend\n");
        code.push_str(&format!("// Target: {}\n\n", self.target.name()));

        // Includes
        match self.target {
            HipBackend::Cuda => {
                code.push_str("#include <cuda_runtime.h>\n");
                code.push_str("#include <stdio.h>\n\n");
            }
            HipBackend::Rocm => {
                code.push_str("#include <hip/hip_runtime.h>\n");
                code.push_str("#include <stdio.h>\n\n");
            }
            _ => {}
        }

        // Macros de compatibilidad
        if self.target == HipBackend::Cuda {
            code.push_str("// CUDA compatibility macros\n");
            code.push_str("#define hipMalloc cudaMalloc\n");
            code.push_str("#define hipFree cudaFree\n");
            code.push_str("#define hipMemcpy cudaMemcpy\n");
            code.push_str("#define hipMemcpyHostToDevice cudaMemcpyHostToDevice\n");
            code.push_str("#define hipMemcpyDeviceToHost cudaMemcpyDeviceToHost\n");
            code.push_str("#define hipDeviceSynchronize cudaDeviceSynchronize\n");
            code.push_str(
                "#define hipLaunchKernelGGL(kernel, grid, block, shMem, stream, ...) \\\n",
            );
            code.push_str("    kernel<<<grid, block, shMem, stream>>>(__VA_ARGS__)\n\n");
        }

        // Kernels
        for kernel in &self.kernels {
            code.push_str(&self.generate_kernel(kernel));
            code.push_str("\n");
        }

        code
    }

    fn generate_kernel(&self, kernel: &HipKernel) -> String {
        let mut code = String::new();

        // Signature
        code.push_str("__global__ void ");
        code.push_str(&kernel.name);
        code.push_str("(");

        let params: Vec<String> = kernel
            .params
            .iter()
            .map(|(name, ty)| format!("{} {}", ty, name))
            .collect();
        code.push_str(&params.join(", "));

        code.push_str(") {\n");
        code.push_str(&kernel.body);
        code.push_str("}\n");

        code
    }

    /// Genera un kernel de vector add portable
    pub fn add_vector_add(&mut self) {
        self.add_kernel(HipKernel {
            name: "vectorAdd".to_string(),
            params: vec![
                ("A".to_string(), "const float*".to_string()),
                ("B".to_string(), "const float*".to_string()),
                ("C".to_string(), "float*".to_string()),
                ("n".to_string(), "int".to_string()),
            ],
            body: r#"    int i = blockDim.x * blockIdx.x + threadIdx.x;
    if (i < n) {
        C[i] = A[i] + B[i];
    }
"#
            .to_string(),
            block_size: (256, 1, 1),
        });
    }

    /// Genera un kernel de SAXPY portable
    pub fn add_saxpy(&mut self) {
        self.add_kernel(HipKernel {
            name: "saxpy".to_string(),
            params: vec![
                ("alpha".to_string(), "float".to_string()),
                ("x".to_string(), "const float*".to_string()),
                ("y".to_string(), "float*".to_string()),
                ("n".to_string(), "int".to_string()),
            ],
            body: r#"    int i = blockDim.x * blockIdx.x + threadIdx.x;
    if (i < n) {
        y[i] = alpha * x[i] + y[i];
    }
"#
            .to_string(),
            block_size: (256, 1, 1),
        });
    }

    /// Genera un kernel de MatMul portable
    pub fn add_matmul(&mut self) {
        self.add_kernel(HipKernel {
            name: "matrixMul".to_string(),
            params: vec![
                ("A".to_string(), "const float*".to_string()),
                ("B".to_string(), "const float*".to_string()),
                ("C".to_string(), "float*".to_string()),
                ("N".to_string(), "int".to_string()),
            ],
            body: r#"    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;
    
    if (row < N && col < N) {
        float sum = 0.0f;
        for (int k = 0; k < N; k++) {
            sum += A[row * N + k] * B[k * N + col];
        }
        C[row * N + col] = sum;
    }
"#
            .to_string(),
            block_size: (16, 16, 1),
        });
    }

    /// Genera un kernel de MatMul con shared memory (optimizado)
    pub fn add_matmul_shared(&mut self) {
        self.add_kernel(HipKernel {
            name: "matrixMulShared".to_string(),
            params: vec![
                ("A".to_string(), "const float*".to_string()),
                ("B".to_string(), "const float*".to_string()),
                ("C".to_string(), "float*".to_string()),
                ("N".to_string(), "int".to_string()),
            ],
            body: r#"    const int TILE_SIZE = 16;
    __shared__ float As[16][16];
    __shared__ float Bs[16][16];
    
    int row = blockIdx.y * TILE_SIZE + threadIdx.y;
    int col = blockIdx.x * TILE_SIZE + threadIdx.x;
    
    float sum = 0.0f;
    
    for (int t = 0; t < (N + TILE_SIZE - 1) / TILE_SIZE; t++) {
        // Load tiles into shared memory
        if (row < N && t * TILE_SIZE + threadIdx.x < N)
            As[threadIdx.y][threadIdx.x] = A[row * N + t * TILE_SIZE + threadIdx.x];
        else
            As[threadIdx.y][threadIdx.x] = 0.0f;
            
        if (col < N && t * TILE_SIZE + threadIdx.y < N)
            Bs[threadIdx.y][threadIdx.x] = B[(t * TILE_SIZE + threadIdx.y) * N + col];
        else
            Bs[threadIdx.y][threadIdx.x] = 0.0f;
            
        __syncthreads();
        
        // Compute partial sum
        for (int k = 0; k < TILE_SIZE; k++) {
            sum += As[threadIdx.y][k] * Bs[k][threadIdx.x];
        }
        
        __syncthreads();
    }
    
    if (row < N && col < N) {
        C[row * N + col] = sum;
    }
"#
            .to_string(),
            block_size: (16, 16, 1),
        });
    }

    /// Genera un kernel de reducción (suma)
    pub fn add_reduce_sum(&mut self) {
        self.add_kernel(HipKernel {
            name: "reduceSum".to_string(),
            params: vec![
                ("input".to_string(), "const float*".to_string()),
                ("output".to_string(), "float*".to_string()),
                ("n".to_string(), "int".to_string()),
            ],
            body: r#"    extern __shared__ float sdata[];
    
    unsigned int tid = threadIdx.x;
    unsigned int i = blockIdx.x * blockDim.x * 2 + threadIdx.x;
    
    // Load and first add
    float sum = 0.0f;
    if (i < n) sum = input[i];
    if (i + blockDim.x < n) sum += input[i + blockDim.x];
    sdata[tid] = sum;
    __syncthreads();
    
    // Reduction in shared memory
    for (unsigned int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (tid < s) {
            sdata[tid] += sdata[tid + s];
        }
        __syncthreads();
    }
    
    // Write result
    if (tid == 0) {
        output[blockIdx.x] = sdata[0];
    }
"#
            .to_string(),
            block_size: (256, 1, 1),
        });
    }
}

/// Imprime información del sistema HIP
pub fn print_hip_info() {
    let info = get_device_info();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    ADead-BIB HIP Runtime                      ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Backend:     {:<48} ║", info.backend.name());
    println!(
        "║ Device:      {:<48} ║",
        if info.device_name.len() > 48 {
            &info.device_name[..48]
        } else {
            &info.device_name
        }
    );
    println!(
        "║ Compute:     {}.{}                                             ║",
        info.compute_capability.0, info.compute_capability.1
    );
    println!(
        "║ Memory:      {} MB                                         ║",
        info.total_memory_mb
    );
    println!(
        "║ SMs/CUs:     {}                                              ║",
        info.multiprocessor_count
    );
    println!(
        "║ Warp/Wave:   {}                                              ║",
        info.warp_size
    );
    println!("╚══════════════════════════════════════════════════════════════╝");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_backend() {
        let backend = detect_hip_backend();
        // Debería detectar algo (al menos CPU)
        assert!(backend != HipBackend::None || backend == HipBackend::Cpu);
    }

    #[test]
    fn test_codegen_cuda() {
        let mut codegen = HipCodeGen::new(HipBackend::Cuda);
        codegen.add_vector_add();
        codegen.add_saxpy();

        let code = codegen.generate();
        assert!(code.contains("__global__"));
        assert!(code.contains("vectorAdd"));
        assert!(code.contains("saxpy"));
        assert!(code.contains("cudaMalloc")); // Macro de compatibilidad
    }

    #[test]
    fn test_codegen_rocm() {
        let mut codegen = HipCodeGen::new(HipBackend::Rocm);
        codegen.add_matmul();

        let code = codegen.generate();
        assert!(code.contains("hip/hip_runtime.h"));
        assert!(code.contains("matrixMul"));
    }

    #[test]
    fn test_matmul_shared() {
        let mut codegen = HipCodeGen::new(HipBackend::Cuda);
        codegen.add_matmul_shared();

        let code = codegen.generate();
        assert!(code.contains("__shared__"));
        assert!(code.contains("__syncthreads"));
    }
}
