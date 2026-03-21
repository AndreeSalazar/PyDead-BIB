// ADead-BIB Runtime - Módulo Principal
// Runtime determinista para exprimir CPU y GPU al máximo
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
//
// Nota: gpu_detect movido a backend/gpu/gpu_detect.rs

pub mod cpu_detect;
pub mod dispatcher;
pub mod gpu_dispatcher;
pub mod gpu_misuse_detector;

pub use cpu_detect::{CPUFeatures, ComputeBackend};
pub use dispatcher::{AutoDispatcher, PerformanceEstimator, SystemInfo};
pub use gpu_dispatcher::{
    DataLocation, DecisionReason, ExecutionTarget, GpuDispatcher, OperationCost,
};
pub use gpu_misuse_detector::{
    GpuMisuseDetector, MisuseReport, MisuseScore, MisuseSeverity, MisuseType,
};

// Re-export GPU detect desde backend
pub use crate::backend::gpu::gpu_detect::{
    detect_cuda_simple, detect_vulkan_simple, GPUFeatures, GPUVendor,
};
