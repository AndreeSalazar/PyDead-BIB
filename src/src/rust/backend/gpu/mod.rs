// ============================================================
// ADead-BIB - GPU Backend
// ============================================================
// BINARY IS BINARY - Emitimos bytes GPU DIRECTAMENTE
// Sin GLSL. Sin HLSL. Código → Opcodes HEX → Backend → GPU
//
// Arquitectura de dos niveles:
// ┌─────────────────────────────────────────────────────────┐
// │ Nivel 1: Opcodes ADead-BIB (0xC0DA...)                  │
// │   - Tu contrato                                         │
// │   - Tu formato                                          │
// │   - Portable                                            │
// │   - Documentado                                         │
// ├─────────────────────────────────────────────────────────┤
// │ Nivel 2: Backend por target                             │
// │   - spirv/   → Vulkan/OpenCL (TODAS las GPUs)           │
// │   - cuda/    → NVIDIA (PTX directo)                     │
// │   - vulkan/  → Runtime Vulkan                           │
// └─────────────────────────────────────────────────────────┘
//
// Estructura:
// - hex/           : 🔥 CORE - Opcodes GPU directos (0xC0DA...)
// - spirv/         : Backend SPIR-V (Vulkan/OpenCL)
// - cuda/          : Backend CUDA (NVIDIA PTX)
// - vulkan/        : Runtime Vulkan
// - detect.rs      : Detección de GPU
// - scheduler.rs   : Scheduler CPU↔GPU
// - memory.rs      : Memoria explícita (buffers)
// - metrics.rs     : Métricas reales
//
// Filosofía: "Bytes directos a la GPU. Sin shaders textuales."
// ============================================================

// === CORE: Opcodes HEX directos ===
pub mod hex;

// === Backends por target ===
pub mod cuda; // CUDA/PTX - Solo NVIDIA
pub mod hip;
pub mod spirv; // SPIR-V (Vulkan/OpenCL) - Todas las GPUs
pub mod vulkan; // Runtime Vulkan // HIP (AMD ROCm + HIP-CPU fallback)

// === API Unificada ===
pub mod compute; // API unificada: compute::parallel_for, compute::matmul, etc.

// === Legacy (mantener compatibilidad) ===
pub mod vulkan_runtime; // TODO: migrar a vulkan/

// === Infraestructura ===
pub mod gpu_detect;
pub mod memory;
pub mod metrics;
pub mod scheduler;
pub mod unified_pipeline;

// Re-exports principales
pub use gpu_detect::*;
pub use memory::{BufferUsage, GpuAllocator, MemoryType};
pub use metrics::{GpuMetrics, GpuProfiler, PerformanceEstimator};
pub use scheduler::{CommandBuffer, Dispatch, GpuScheduler};
pub use spirv::bytecode::{ADeadGpuOp, BytecodeToSpirV};

// Re-exports HIP + Compute API
pub use compute::{BenchmarkResults, ComputeBackend, ComputeConfig, ComputeRuntime};
pub use hip::cuda_to_hip::{translate_cuda_file, CudaToHipTranslator};
pub use hip::{detect_hip_backend, get_device_info, HipBackend, HipDeviceInfo};
pub use hip::{print_hip_info, HipCodeGen, HipKernel};
pub use hip::{Dim3, HipCpuConfig, HipCpuRuntime, SendPtr, ThreadIdx};
