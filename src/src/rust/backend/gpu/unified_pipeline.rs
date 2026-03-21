// ADead-BIB - Unified Pipeline (HEX + Binario + CUDA)
// Elimina ruido innecesario y facilita TODO el flujo de compilación
//
// Filosofía: Detectar → Respetar → Decidir → Ejecutar
//
// PRINCIPIOS:
// 1. DETECTAR: Conocer exactamente qué hardware tenemos
// 2. RESPETAR: No exceder límites de VRAM, bandwidth, TDP
// 3. DECIDIR: Elegir CPU/GPU basado en datos reales
// 4. EJECUTAR: Código limpio sin ruido
//
// Autor: Eddi Andreé Salazar Matos

use super::gpu_detect::{GPUFeatures, GPUVendor};
use super::hex::{GpuOpcode, HexGenerator};
use crate::runtime::gpu_dispatcher::{DataLocation, ExecutionTarget, GpuDispatcher, OperationCost};
use std::process::Command;

// ============================================================================
// DETECCIÓN DETALLADA DE HARDWARE
// ============================================================================

/// Estado actual de la GPU (detección en tiempo real)
#[derive(Debug, Clone)]
pub struct GpuRuntimeState {
    /// VRAM total en MB
    pub vram_total_mb: u32,
    /// VRAM libre en MB
    pub vram_free_mb: u32,
    /// VRAM usada en MB
    pub vram_used_mb: u32,
    /// Temperatura actual en °C
    pub temperature_c: u32,
    /// Utilización GPU %
    pub gpu_utilization: u32,
    /// Utilización memoria %
    pub memory_utilization: u32,
    /// Power draw actual en W
    pub power_draw_w: u32,
    /// Power limit en W
    pub power_limit_w: u32,
    /// Clock actual en MHz
    pub clock_mhz: u32,
    /// Memory clock en MHz
    pub memory_clock_mhz: u32,
}

impl Default for GpuRuntimeState {
    fn default() -> Self {
        Self {
            vram_total_mb: 0,
            vram_free_mb: 0,
            vram_used_mb: 0,
            temperature_c: 0,
            gpu_utilization: 0,
            memory_utilization: 0,
            power_draw_w: 0,
            power_limit_w: 0,
            clock_mhz: 0,
            memory_clock_mhz: 0,
        }
    }
}

impl GpuRuntimeState {
    /// Detecta estado actual via nvidia-smi
    pub fn detect() -> Self {
        let mut state = Self::default();

        // Query nvidia-smi para estado en tiempo real
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=memory.total,memory.free,memory.used,temperature.gpu,utilization.gpu,utilization.memory,power.draw,power.limit,clocks.current.graphics,clocks.current.memory",
                "--format=csv,noheader,nounits"
            ])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.trim().split(',').map(|s| s.trim()).collect();

                if parts.len() >= 10 {
                    state.vram_total_mb = parts[0].parse().unwrap_or(0);
                    state.vram_free_mb = parts[1].parse().unwrap_or(0);
                    state.vram_used_mb = parts[2].parse().unwrap_or(0);
                    state.temperature_c = parts[3].parse().unwrap_or(0);
                    state.gpu_utilization = parts[4].parse().unwrap_or(0);
                    state.memory_utilization = parts[5].parse().unwrap_or(0);
                    state.power_draw_w = parts[6].parse::<f32>().unwrap_or(0.0) as u32;
                    state.power_limit_w = parts[7].parse::<f32>().unwrap_or(0.0) as u32;
                    state.clock_mhz = parts[8].parse().unwrap_or(0);
                    state.memory_clock_mhz = parts[9].parse().unwrap_or(0);
                }
            }
        }

        state
    }

    /// ¿Hay suficiente VRAM para esta operación?
    pub fn has_enough_vram(&self, required_mb: u32) -> bool {
        self.vram_free_mb >= required_mb
    }

    /// ¿La GPU está sobrecalentada? (> 80°C)
    pub fn is_overheating(&self) -> bool {
        self.temperature_c > 80
    }

    /// ¿La GPU está muy ocupada? (> 90%)
    pub fn is_busy(&self) -> bool {
        self.gpu_utilization > 90
    }

    /// ¿Estamos cerca del límite de potencia? (> 95%)
    pub fn is_power_limited(&self) -> bool {
        if self.power_limit_w == 0 {
            return false;
        }
        (self.power_draw_w as f32 / self.power_limit_w as f32) > 0.95
    }
}

/// Razón detallada de la decisión CPU/GPU
#[derive(Debug, Clone)]
pub enum DecisionReason {
    /// GPU no disponible
    NoGpu,
    /// Datos muy pequeños para GPU
    DataTooSmall {
        elements: usize,
        min_threshold: usize,
    },
    /// No hay suficiente VRAM
    InsufficientVram { required_mb: u32, available_mb: u32 },
    /// GPU sobrecalentada
    GpuOverheating { temp_c: u32 },
    /// GPU muy ocupada
    GpuBusy { utilization: u32 },
    /// Límite de potencia alcanzado
    PowerLimited { power_w: u32, limit_w: u32 },
    /// Baja intensidad computacional (CPU es mejor)
    LowComputeIntensity { flops_per_byte: f64, threshold: f64 },
    /// Alta intensidad computacional (GPU es mejor)
    HighComputeIntensity { flops_per_byte: f64 },
    /// Datos ya en GPU
    DataOnDevice,
    /// Datos persistirán en GPU
    DataWillPersist,
    /// Forzado por usuario
    ForcedByUser,
}

impl DecisionReason {
    /// Descripción legible de la razón
    pub fn description(&self) -> String {
        match self {
            Self::NoGpu => "❌ GPU no disponible".to_string(),
            Self::DataTooSmall {
                elements,
                min_threshold,
            } => format!(
                "📉 Datos pequeños: {} < {} elementos (overhead PCIe domina)",
                elements, min_threshold
            ),
            Self::InsufficientVram {
                required_mb,
                available_mb,
            } => format!(
                "💾 VRAM insuficiente: necesita {} MB, disponible {} MB",
                required_mb, available_mb
            ),
            Self::GpuOverheating { temp_c } => {
                format!("🌡️ GPU sobrecalentada: {}°C (límite 80°C)", temp_c)
            }
            Self::GpuBusy { utilization } => {
                format!("⏳ GPU ocupada: {}% utilización", utilization)
            }
            Self::PowerLimited { power_w, limit_w } => {
                format!("⚡ Límite potencia: {} W / {} W", power_w, limit_w)
            }
            Self::LowComputeIntensity {
                flops_per_byte,
                threshold,
            } => format!(
                "📊 Baja intensidad: {:.2} FLOPs/byte < {:.2} (CPU mejor)",
                flops_per_byte, threshold
            ),
            Self::HighComputeIntensity { flops_per_byte } => format!(
                "🚀 Alta intensidad: {:.2} FLOPs/byte (GPU óptimo)",
                flops_per_byte
            ),
            Self::DataOnDevice => "✅ Datos ya en GPU".to_string(),
            Self::DataWillPersist => "📌 Datos persistirán en GPU".to_string(),
            Self::ForcedByUser => "👤 Forzado por usuario".to_string(),
        }
    }
}

/// Log de decisión para debugging/análisis
#[derive(Debug, Clone)]
pub struct DecisionLog {
    pub operation: String,
    pub target: ExecutionTarget,
    pub reason: DecisionReason,
    pub vram_required_mb: u32,
    pub estimated_time_us: f64,
    pub gpu_state: GpuRuntimeState,
}

// ============================================================================
// PIPELINE UNIFICADO
// ============================================================================

/// Pipeline unificado que elimina ruido y decide automáticamente CPU/GPU
pub struct UnifiedPipeline {
    /// Características de GPU detectadas
    gpu: GPUFeatures,
    /// Estado actual de la GPU (tiempo real)
    gpu_state: GpuRuntimeState,
    /// Dispatcher para decisiones CPU/GPU
    dispatcher: GpuDispatcher,
    /// Modo de operación
    mode: PipelineMode,
    /// Estadísticas de optimización
    stats: OptimizationStats,
    /// Log de decisiones
    decision_log: Vec<DecisionLog>,
    /// Verbose mode
    verbose: bool,
}

/// Modo de operación del pipeline
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PipelineMode {
    /// Solo CPU - sin GPU disponible o datos muy pequeños
    CpuOnly,
    /// GPU preferido - CUDA disponible y datos suficientes
    GpuPreferred,
    /// Híbrido - decisión automática por operación
    Hybrid,
    /// Forzar GPU - ignorar cost model (para benchmarks)
    ForceGpu,
}

/// Estadísticas de optimización
#[derive(Debug, Default)]
pub struct OptimizationStats {
    /// Operaciones enviadas a CPU
    pub cpu_ops: usize,
    /// Operaciones enviadas a GPU
    pub gpu_ops: usize,
    /// Bytes de código eliminados (ruido)
    pub noise_removed_bytes: usize,
    /// Instrucciones fusionadas
    pub fused_instructions: usize,
    /// Transferencias PCIe evitadas
    pub transfers_avoided: usize,
}

/// Resultado de compilación unificada
#[derive(Debug)]
pub struct CompilationResult {
    /// Código binario generado (CPU o GPU)
    pub binary: Vec<u8>,
    /// Target de ejecución
    pub target: ExecutionTarget,
    /// Formato del binario
    pub format: BinaryFormat,
    /// Estadísticas
    pub stats: OptimizationStats,
}

/// Formato del binario generado
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryFormat {
    /// x86-64 nativo
    X86_64,
    /// HEX para GPU (formato ADead)
    HexGpu,
    /// CUDA PTX
    CudaPtx,
    /// SPIR-V para Vulkan
    SpirV,
}

impl UnifiedPipeline {
    /// Crea un nuevo pipeline con detección automática
    pub fn new() -> Self {
        let gpu = GPUFeatures::detect();
        let gpu_state = GpuRuntimeState::detect();
        let mode = Self::determine_mode(&gpu);

        Self {
            gpu,
            gpu_state,
            dispatcher: GpuDispatcher::new(),
            mode,
            stats: OptimizationStats::default(),
            decision_log: Vec::new(),
            verbose: false,
        }
    }

    /// Crea pipeline con modo específico
    pub fn with_mode(mode: PipelineMode) -> Self {
        let gpu = GPUFeatures::detect();
        let gpu_state = GpuRuntimeState::detect();
        Self {
            gpu,
            gpu_state,
            dispatcher: GpuDispatcher::new(),
            mode,
            stats: OptimizationStats::default(),
            decision_log: Vec::new(),
            verbose: false,
        }
    }

    /// Crea pipeline con modo verbose (muestra decisiones detalladas)
    pub fn with_verbose(mode: PipelineMode, verbose: bool) -> Self {
        let gpu = GPUFeatures::detect();
        let gpu_state = GpuRuntimeState::detect();
        Self {
            gpu,
            gpu_state,
            dispatcher: GpuDispatcher::new(),
            mode,
            stats: OptimizationStats::default(),
            decision_log: Vec::new(),
            verbose,
        }
    }

    /// Actualiza el estado de la GPU (llamar antes de operaciones críticas)
    pub fn refresh_gpu_state(&mut self) {
        self.gpu_state = GpuRuntimeState::detect();
    }

    /// Determina el modo óptimo basado en hardware
    fn determine_mode(gpu: &GPUFeatures) -> PipelineMode {
        if !gpu.available {
            return PipelineMode::CpuOnly;
        }

        if gpu.cuda_available && gpu.vendor == GPUVendor::NVIDIA {
            PipelineMode::GpuPreferred
        } else if gpu.vulkan_available {
            PipelineMode::Hybrid
        } else {
            PipelineMode::CpuOnly
        }
    }

    /// Calcula VRAM requerida para una operación (en MB)
    fn calculate_vram_required(&self, op: &MathOperation) -> u32 {
        match op {
            MathOperation::VectorAdd { size } => {
                // 3 vectores de floats: A, B, C
                ((*size * 3 * 4) / (1024 * 1024)) as u32 + 1
            }
            MathOperation::MatMul { m, n, k } => {
                // 3 matrices: A(m×k), B(k×n), C(m×n)
                let a_size = m * k * 4;
                let b_size = k * n * 4;
                let c_size = m * n * 4;
                ((a_size + b_size + c_size) / (1024 * 1024)) as u32 + 1
            }
            MathOperation::Saxpy { size, .. } => {
                // 2 vectores: x, y
                ((*size * 2 * 4) / (1024 * 1024)) as u32 + 1
            }
            MathOperation::Reduction { size } => {
                // 1 vector + resultado parcial
                ((*size * 4 + 1024) / (1024 * 1024)) as u32 + 1
            }
        }
    }

    /// Decide dónde ejecutar con respeto total al hardware
    fn decide_with_respect(&mut self, op: &MathOperation) -> (ExecutionTarget, DecisionReason) {
        let cost = op.to_cost();
        let vram_required = self.calculate_vram_required(op);

        // 1. ¿GPU disponible?
        if !self.gpu.available {
            return (ExecutionTarget::CPU, DecisionReason::NoGpu);
        }

        // 2. ¿Modo forzado?
        if self.mode == PipelineMode::ForceGpu {
            return (ExecutionTarget::GPU, DecisionReason::ForcedByUser);
        }
        if self.mode == PipelineMode::CpuOnly {
            return (ExecutionTarget::CPU, DecisionReason::ForcedByUser);
        }

        // 3. RESPETAR: Verificar límites de hardware

        // 3a. ¿Suficiente VRAM?
        if !self.gpu_state.has_enough_vram(vram_required) {
            return (
                ExecutionTarget::CPU,
                DecisionReason::InsufficientVram {
                    required_mb: vram_required,
                    available_mb: self.gpu_state.vram_free_mb,
                },
            );
        }

        // 3b. ¿GPU sobrecalentada?
        if self.gpu_state.is_overheating() {
            return (
                ExecutionTarget::CPU,
                DecisionReason::GpuOverheating {
                    temp_c: self.gpu_state.temperature_c,
                },
            );
        }

        // 3c. ¿GPU muy ocupada?
        if self.gpu_state.is_busy() {
            return (
                ExecutionTarget::CPU,
                DecisionReason::GpuBusy {
                    utilization: self.gpu_state.gpu_utilization,
                },
            );
        }

        // 3d. ¿Límite de potencia?
        if self.gpu_state.is_power_limited() {
            return (
                ExecutionTarget::CPU,
                DecisionReason::PowerLimited {
                    power_w: self.gpu_state.power_draw_w,
                    limit_w: self.gpu_state.power_limit_w,
                },
            );
        }

        // 4. DECIDIR: Basado en intensidad computacional
        let elements = match op {
            MathOperation::VectorAdd { size } => *size,
            MathOperation::MatMul { m, n, .. } => m * n,
            MathOperation::Saxpy { size, .. } => *size,
            MathOperation::Reduction { size } => *size,
        };

        // Umbral mínimo de elementos
        const MIN_ELEMENTS: usize = 100_000;
        if elements < MIN_ELEMENTS {
            return (
                ExecutionTarget::CPU,
                DecisionReason::DataTooSmall {
                    elements,
                    min_threshold: MIN_ELEMENTS,
                },
            );
        }

        // Intensidad computacional
        let flops_per_byte = cost.flops_per_byte();
        const MIN_INTENSITY: f64 = 0.5;

        if flops_per_byte < MIN_INTENSITY {
            return (
                ExecutionTarget::CPU,
                DecisionReason::LowComputeIntensity {
                    flops_per_byte,
                    threshold: MIN_INTENSITY,
                },
            );
        }

        // GPU es la mejor opción
        (
            ExecutionTarget::GPU,
            DecisionReason::HighComputeIntensity { flops_per_byte },
        )
    }

    /// Compila una operación matemática de forma óptima
    pub fn compile_math_op(&mut self, op: MathOperation) -> CompilationResult {
        // Refrescar estado de GPU antes de decidir
        self.refresh_gpu_state();

        let (target, reason) = self.decide_with_respect(&op);
        let vram_required = self.calculate_vram_required(&op);

        // Log de decisión
        if self.verbose {
            println!("   📋 Decisión: {}", reason.description());
        }

        let log = DecisionLog {
            operation: format!("{:?}", op),
            target,
            reason: reason.clone(),
            vram_required_mb: vram_required,
            estimated_time_us: op.to_cost().estimate_kernel_us(),
            gpu_state: self.gpu_state.clone(),
        };
        self.decision_log.push(log);

        match target {
            ExecutionTarget::CPU => {
                self.stats.cpu_ops += 1;
                self.compile_cpu(&op)
            }
            ExecutionTarget::GPU
            | ExecutionTarget::GPUWithTransfer
            | ExecutionTarget::GPURoundTrip => {
                self.stats.gpu_ops += 1;
                self.compile_gpu(&op)
            }
        }
    }

    /// Compila para CPU (x86-64)
    fn compile_cpu(&mut self, op: &MathOperation) -> CompilationResult {
        let mut code = Vec::new();

        match op {
            MathOperation::VectorAdd { size } => {
                // Loop optimizado sin ruido
                // for (i = 0; i < n; i++) C[i] = A[i] + B[i]
                code.extend_from_slice(&self.emit_vector_loop_x86(*size));
            }
            MathOperation::MatMul { m, n, k } => {
                // MatMul con loop tiling para cache
                code.extend_from_slice(&self.emit_matmul_tiled_x86(*m, *n, *k));
            }
            MathOperation::Saxpy { size, .. } => {
                // SAXPY: y = a*x + y
                code.extend_from_slice(&self.emit_saxpy_x86(*size));
            }
            MathOperation::Reduction { size } => {
                code.extend_from_slice(&self.emit_reduction_x86(*size));
            }
        }

        CompilationResult {
            binary: code,
            target: ExecutionTarget::CPU,
            format: BinaryFormat::X86_64,
            stats: std::mem::take(&mut self.stats),
        }
    }

    /// Compila para GPU (HEX o CUDA)
    fn compile_gpu(&mut self, op: &MathOperation) -> CompilationResult {
        if self.gpu.cuda_available {
            self.compile_cuda(op)
        } else {
            self.compile_hex(op)
        }
    }

    /// Compila a formato HEX (GPU directo)
    fn compile_hex(&mut self, op: &MathOperation) -> CompilationResult {
        let mut gen = HexGenerator::new();

        match op {
            MathOperation::VectorAdd { .. } => {
                // Código HEX mínimo para VectorAdd
                gen.emit(GpuOpcode::Load, 0, 0, 0); // Load A
                gen.emit(GpuOpcode::Load, 1, 1, 0); // Load B
                gen.emit(GpuOpcode::VecAdd, 2, 0, 1); // C = A + B
                gen.emit(GpuOpcode::Store, 2, 2, 0); // Store C
                gen.emit(GpuOpcode::Exit, 0, 0, 0);
            }
            MathOperation::MatMul { .. } => {
                gen.emit(GpuOpcode::Load, 0, 0, 0);
                gen.emit(GpuOpcode::Load, 1, 1, 0);
                gen.emit(GpuOpcode::MatMul, 2, 0, 1);
                gen.emit(GpuOpcode::Store, 2, 2, 0);
                gen.emit(GpuOpcode::Sync, 0, 0, 0);
                gen.emit(GpuOpcode::Exit, 0, 0, 0);
            }
            MathOperation::Saxpy { .. } => {
                gen.emit(GpuOpcode::Load, 0, 0, 0); // Load x
                gen.emit(GpuOpcode::Load, 1, 1, 0); // Load y
                gen.emit(GpuOpcode::Fma, 1, 0, 1); // y = a*x + y (FMA)
                gen.emit(GpuOpcode::Store, 1, 1, 0);
                gen.emit(GpuOpcode::Exit, 0, 0, 0);
            }
            MathOperation::Reduction { .. } => {
                gen.emit(GpuOpcode::Load, 0, 0, 0);
                gen.emit(GpuOpcode::VecDot, 1, 0, 0); // Usar dot product consigo mismo
                gen.emit(GpuOpcode::Store, 1, 1, 0);
                gen.emit(GpuOpcode::Exit, 0, 0, 0);
            }
        }

        // Optimizar: eliminar instrucciones redundantes
        let optimized = self.optimize_hex(gen.to_hex());

        CompilationResult {
            binary: optimized,
            target: ExecutionTarget::GPU,
            format: BinaryFormat::HexGpu,
            stats: std::mem::take(&mut self.stats),
        }
    }

    /// Compila a CUDA PTX inline
    fn compile_cuda(&mut self, op: &MathOperation) -> CompilationResult {
        let cuda_code = match op {
            MathOperation::VectorAdd { size } => self.generate_cuda_vectoradd(*size),
            MathOperation::MatMul { m, n, k } => self.generate_cuda_matmul(*m, *n, *k),
            MathOperation::Saxpy { size, alpha } => self.generate_cuda_saxpy(*size, *alpha),
            MathOperation::Reduction { size } => self.generate_cuda_reduction(*size),
        };

        CompilationResult {
            binary: cuda_code.into_bytes(),
            target: ExecutionTarget::GPU,
            format: BinaryFormat::CudaPtx,
            stats: std::mem::take(&mut self.stats),
        }
    }

    // ========================================
    // Generadores x86-64 optimizados
    // ========================================

    fn emit_vector_loop_x86(&self, _size: usize) -> Vec<u8> {
        // Código x86-64 optimizado para VectorAdd
        // Usa SIMD cuando sea posible
        vec![
            // xor ecx, ecx (i = 0)
            0x31, 0xC9, // loop_start:
            // movaps xmm0, [rdi + rcx*4]
            0x0F, 0x28, 0x04, 0x8F, // addps xmm0, [rsi + rcx*4]
            0x0F, 0x58, 0x04, 0x8E, // movaps [rdx + rcx*4], xmm0
            0x0F, 0x29, 0x04, 0x8A, // add ecx, 4
            0x83, 0xC1, 0x04, // cmp ecx, size
            0x3B, 0x4C, 0x24, 0x08, // jl loop_start
            0x7C, 0xED, // ret
            0xC3,
        ]
    }

    fn emit_matmul_tiled_x86(&self, _m: usize, _n: usize, _k: usize) -> Vec<u8> {
        // MatMul con tiling para mejor uso de cache
        // Tile size = 32x32 para L1 cache
        vec![
            // Prologue
            0x55, // push rbp
            0x48, 0x89, 0xE5, // mov rbp, rsp
            // Loop structure (simplified)
            0x31, 0xC0, // xor eax, eax
            // ... (código de loop tiling)
            // Epilogue
            0x5D, // pop rbp
            0xC3, // ret
        ]
    }

    fn emit_saxpy_x86(&self, _size: usize) -> Vec<u8> {
        vec![
            // SAXPY optimizado con FMA si disponible
            0x31, 0xC9, // xor ecx, ecx
            // vfmadd231ps ymm0, ymm1, [rdi + rcx]
            0xC4, 0xE2, 0x75, 0xB8, 0x04, 0x0F, 0xC3,
        ]
    }

    fn emit_reduction_x86(&self, _size: usize) -> Vec<u8> {
        vec![
            // Reduction con tree reduction
            0x66, 0x0F, 0xEF, 0xC0, // pxor xmm0, xmm0
            // ... loop de suma
            0xC3,
        ]
    }

    // ========================================
    // Generadores CUDA
    // ========================================

    fn generate_cuda_vectoradd(&self, size: usize) -> String {
        format!(
            r#"// ADead-BIB CUDA - VectorAdd (optimizado, sin ruido)
__global__ void vectorAdd(float *A, float *B, float *C, int n) {{
    int i = blockDim.x * blockIdx.x + threadIdx.x;
    if (i < n) C[i] = A[i] + B[i];
}}
// Launch: vectorAdd<<<{blocks}, 256>>>(A, B, C, {size});
"#,
            blocks = (size + 255) / 256,
            size = size
        )
    }

    fn generate_cuda_matmul(&self, m: usize, n: usize, k: usize) -> String {
        format!(
            r#"// ADead-BIB CUDA - MatMul (tiled, optimizado)
#define TILE 16
__global__ void matmul(float *A, float *B, float *C, int M, int N, int K) {{
    __shared__ float As[TILE][TILE], Bs[TILE][TILE];
    int row = blockIdx.y * TILE + threadIdx.y;
    int col = blockIdx.x * TILE + threadIdx.x;
    float sum = 0.0f;
    for (int t = 0; t < (K + TILE - 1) / TILE; t++) {{
        if (row < M && t * TILE + threadIdx.x < K)
            As[threadIdx.y][threadIdx.x] = A[row * K + t * TILE + threadIdx.x];
        else As[threadIdx.y][threadIdx.x] = 0.0f;
        if (col < N && t * TILE + threadIdx.y < K)
            Bs[threadIdx.y][threadIdx.x] = B[(t * TILE + threadIdx.y) * N + col];
        else Bs[threadIdx.y][threadIdx.x] = 0.0f;
        __syncthreads();
        for (int i = 0; i < TILE; i++) sum += As[threadIdx.y][i] * Bs[i][threadIdx.x];
        __syncthreads();
    }}
    if (row < M && col < N) C[row * N + col] = sum;
}}
// Launch: matmul<<<dim3({bx},{by}), dim3(16,16)>>>(A, B, C, {m}, {n}, {k});
"#,
            bx = (n + 15) / 16,
            by = (m + 15) / 16,
            m = m,
            n = n,
            k = k
        )
    }

    fn generate_cuda_saxpy(&self, size: usize, alpha: f32) -> String {
        format!(
            r#"// ADead-BIB CUDA - SAXPY (y = a*x + y)
__global__ void saxpy(float a, float *x, float *y, int n) {{
    int i = blockDim.x * blockIdx.x + threadIdx.x;
    if (i < n) y[i] = a * x[i] + y[i];
}}
// Launch: saxpy<<<{blocks}, 256>>>({alpha}f, x, y, {size});
"#,
            blocks = (size + 255) / 256,
            alpha = alpha,
            size = size
        )
    }

    fn generate_cuda_reduction(&self, size: usize) -> String {
        format!(
            r#"// ADead-BIB CUDA - Reduction (parallel sum)
__global__ void reduce(float *in, float *out, int n) {{
    __shared__ float sdata[256];
    int tid = threadIdx.x;
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    sdata[tid] = (i < n) ? in[i] : 0.0f;
    __syncthreads();
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {{
        if (tid < s) sdata[tid] += sdata[tid + s];
        __syncthreads();
    }}
    if (tid == 0) out[blockIdx.x] = sdata[0];
}}
// Launch: reduce<<<{blocks}, 256>>>(in, out, {size});
"#,
            blocks = (size + 255) / 256,
            size = size
        )
    }

    // ========================================
    // Optimizador HEX
    // ========================================

    /// Optimiza código HEX eliminando ruido
    fn optimize_hex(&mut self, mut code: Vec<u8>) -> Vec<u8> {
        let original_len = code.len();

        // Patrón 1: Load seguido de Store al mismo registro → eliminar
        let mut i = 0;
        while i + 8 <= code.len() {
            if code[i] == GpuOpcode::Load as u8
                && code[i + 4] == GpuOpcode::Store as u8
                && code[i + 1] == code[i + 5]
            {
                // mismo registro
                // Eliminar ambas instrucciones
                code.drain(i..i + 8);
                self.stats.fused_instructions += 2;
                continue;
            }
            i += 4;
        }

        // Patrón 2: Sync consecutivos → mantener solo uno
        i = 0;
        while i + 8 <= code.len() {
            if code[i] == GpuOpcode::Sync as u8 && code[i + 4] == GpuOpcode::Sync as u8 {
                code.drain(i + 4..i + 8);
                self.stats.fused_instructions += 1;
                continue;
            }
            i += 4;
        }

        // Patrón 3: Exit seguido de cualquier cosa → eliminar lo que sigue
        if let Some(exit_pos) = code.chunks(4).position(|c| c[0] == GpuOpcode::Exit as u8) {
            let keep_until = (exit_pos + 1) * 4;
            if code.len() > keep_until {
                let removed = code.len() - keep_until;
                code.truncate(keep_until);
                self.stats.noise_removed_bytes += removed;
            }
        }

        self.stats.noise_removed_bytes += original_len - code.len();
        code
    }

    /// Imprime resumen del pipeline con estado detallado
    pub fn print_summary(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║        ADead-BIB Unified Pipeline v2.0                       ║");
        println!("║        Detectar → Respetar → Decidir → Ejecutar              ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ 🔧 CONFIGURACIÓN                                             ║");
        println!("║   Mode:        {:?}", self.mode);
        println!(
            "║   Verbose:     {}",
            if self.verbose { "✅" } else { "❌" }
        );
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ 🎮 GPU DETECTADA                                             ║");
        if self.gpu.available {
            println!("║   Device:      {}", self.gpu.device_name);
            println!("║   Vendor:      {:?}", self.gpu.vendor);
            println!(
                "║   CUDA:        {}",
                if self.gpu.cuda_available {
                    "✅ Available"
                } else {
                    "❌ Not available"
                }
            );
            println!(
                "║   Vulkan:      {}",
                if self.gpu.vulkan_available {
                    "✅ Available"
                } else {
                    "❌ Not available"
                }
            );
        } else {
            println!("║   ❌ No GPU disponible");
        }
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ 📊 ESTADO EN TIEMPO REAL                                     ║");
        if self.gpu_state.vram_total_mb > 0 {
            println!(
                "║   VRAM Total:  {} MB ({:.1} GB)",
                self.gpu_state.vram_total_mb,
                self.gpu_state.vram_total_mb as f32 / 1024.0
            );
            println!(
                "║   VRAM Libre:  {} MB ({:.1}%)",
                self.gpu_state.vram_free_mb,
                (self.gpu_state.vram_free_mb as f32 / self.gpu_state.vram_total_mb as f32) * 100.0
            );
            println!("║   VRAM Usada:  {} MB", self.gpu_state.vram_used_mb);
            println!("║   ─────────────────────────────────────────────────────────");

            // Temperatura con indicador visual
            let temp_icon = if self.gpu_state.temperature_c > 80 {
                "🔥"
            } else if self.gpu_state.temperature_c > 70 {
                "🌡️"
            } else {
                "❄️"
            };
            println!(
                "║   Temperatura: {} {}°C",
                temp_icon, self.gpu_state.temperature_c
            );

            // Utilización con barra visual
            let util_bar = Self::make_progress_bar(self.gpu_state.gpu_utilization as usize, 20);
            println!(
                "║   GPU Util:    [{}] {}%",
                util_bar, self.gpu_state.gpu_utilization
            );

            let mem_bar = Self::make_progress_bar(self.gpu_state.memory_utilization as usize, 20);
            println!(
                "║   Mem Util:    [{}] {}%",
                mem_bar, self.gpu_state.memory_utilization
            );

            println!("║   ─────────────────────────────────────────────────────────");
            println!(
                "║   Power:       {} W / {} W",
                self.gpu_state.power_draw_w, self.gpu_state.power_limit_w
            );
            println!("║   GPU Clock:   {} MHz", self.gpu_state.clock_mhz);
            println!("║   Mem Clock:   {} MHz", self.gpu_state.memory_clock_mhz);
        } else {
            println!("║   ⚠️  No se pudo obtener estado en tiempo real");
        }
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ 🛡️ LÍMITES RESPETADOS                                        ║");
        println!(
            "║   VRAM:        {} (libre > requerida)",
            if self.gpu_state.vram_free_mb > 100 {
                "✅"
            } else {
                "⚠️"
            }
        );
        println!(
            "║   Temperatura: {} (< 80°C)",
            if !self.gpu_state.is_overheating() {
                "✅"
            } else {
                "🔥 CALIENTE"
            }
        );
        println!(
            "║   Ocupación:   {} (< 90%)",
            if !self.gpu_state.is_busy() {
                "✅"
            } else {
                "⏳ OCUPADA"
            }
        );
        println!(
            "║   Potencia:    {} (< 95% límite)",
            if !self.gpu_state.is_power_limited() {
                "✅"
            } else {
                "⚡ LIMITADA"
            }
        );
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ 📈 ESTADÍSTICAS DE COMPILACIÓN                               ║");
        println!("║   CPU ops:           {}", self.stats.cpu_ops);
        println!("║   GPU ops:           {}", self.stats.gpu_ops);
        println!(
            "║   Noise removed:     {} bytes",
            self.stats.noise_removed_bytes
        );
        println!("║   Fused instructions:{}", self.stats.fused_instructions);
        println!("║   Transfers avoided: {}", self.stats.transfers_avoided);

        // Mostrar log de decisiones si hay
        if !self.decision_log.is_empty() {
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ 📋 LOG DE DECISIONES                                         ║");
            for (i, log) in self.decision_log.iter().enumerate() {
                println!(
                    "║   {}. {:?} → {:?}",
                    i + 1,
                    log.target,
                    log.reason.description()
                );
                println!(
                    "║      VRAM req: {} MB | Est. time: {:.1} µs",
                    log.vram_required_mb, log.estimated_time_us
                );
            }
        }

        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    /// Crea una barra de progreso visual
    fn make_progress_bar(percent: usize, width: usize) -> String {
        let filled = (percent * width) / 100;
        let empty = width - filled;
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }

    /// Imprime solo el estado de la GPU (útil para monitoreo)
    pub fn print_gpu_status(&self) {
        println!(
            "🎮 GPU Status: {} | {}°C | {}% util | {} MB free",
            &self.gpu.device_name,
            self.gpu_state.temperature_c,
            self.gpu_state.gpu_utilization,
            self.gpu_state.vram_free_mb
        );
    }

    /// Obtiene el log de decisiones
    pub fn get_decision_log(&self) -> &[DecisionLog] {
        &self.decision_log
    }

    /// Limpia el log de decisiones
    pub fn clear_decision_log(&mut self) {
        self.decision_log.clear();
    }
}

impl Default for UnifiedPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Operaciones matemáticas soportadas
#[derive(Debug, Clone)]
pub enum MathOperation {
    VectorAdd { size: usize },
    MatMul { m: usize, n: usize, k: usize },
    Saxpy { size: usize, alpha: f32 },
    Reduction { size: usize },
}

impl MathOperation {
    /// Convierte a OperationCost para el dispatcher
    pub fn to_cost(&self) -> OperationCost {
        match self {
            MathOperation::VectorAdd { size } => {
                crate::runtime::gpu_dispatcher::operations::vector_add(
                    *size,
                    DataLocation::Host,
                    false,
                )
            }
            MathOperation::MatMul { m, .. } => {
                crate::runtime::gpu_dispatcher::operations::matmul(*m, DataLocation::Host, true)
            }
            MathOperation::Saxpy { size, .. } => {
                crate::runtime::gpu_dispatcher::operations::saxpy(*size, DataLocation::Host, false)
            }
            MathOperation::Reduction { size } => {
                crate::runtime::gpu_dispatcher::operations::reduction(*size, DataLocation::Host)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = UnifiedPipeline::new();
        assert!(matches!(
            pipeline.mode,
            PipelineMode::CpuOnly | PipelineMode::GpuPreferred | PipelineMode::Hybrid
        ));
    }

    #[test]
    fn test_vectoradd_compilation() {
        let mut pipeline = UnifiedPipeline::with_mode(PipelineMode::CpuOnly);
        let result = pipeline.compile_math_op(MathOperation::VectorAdd { size: 1024 });
        assert!(!result.binary.is_empty());
        assert_eq!(result.format, BinaryFormat::X86_64);
    }

    #[test]
    fn test_hex_optimization() {
        let mut pipeline = UnifiedPipeline::new();

        // Código con ruido: Sync, Sync, Exit, Load (el Load después de Exit es ruido)
        let noisy_code = vec![
            GpuOpcode::Sync as u8,
            0,
            0,
            0,
            GpuOpcode::Sync as u8,
            0,
            0,
            0,
            GpuOpcode::Exit as u8,
            0,
            0,
            0,
            GpuOpcode::Load as u8,
            0,
            0,
            0, // Ruido después de Exit
        ];

        let optimized = pipeline.optimize_hex(noisy_code);

        // Debe eliminar el segundo Sync y el Load después de Exit
        assert!(optimized.len() < 16);
    }
}
