// ============================================================
// ADead-BIB - HIP Backend (HIP-CPU + AMD HIP)
// ============================================================
// HIP = Heterogeneous-compute Interface for Portability
//
// Este módulo proporciona:
// 1. HIP-CPU: Fallback que ejecuta kernels en CPU con SIMD
// 2. HIP nativo: Para GPUs AMD (ROCm)
// 3. Traducción CUDA→HIP: Portabilidad NVIDIA↔AMD
//
// Filosofía ADead-BIB:
// - Código único → múltiples backends
// - Sin dependencias externas pesadas
// - Bytes directos cuando es posible
// ============================================================

pub mod cuda_to_hip;
pub mod hip_cpu;
pub mod hip_runtime;

pub use hip_cpu::{Dim3, HipCpuConfig, HipCpuRuntime, HipCpuStats, SendPtr, ThreadIdx};
pub use hip_runtime::{
    detect_hip_backend, get_device_info, print_hip_info, HipBackend, HipCodeGen, HipDeviceInfo,
    HipKernel,
};
