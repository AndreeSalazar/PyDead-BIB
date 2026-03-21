"""
ADead-BIB FFI GPU
==================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪

Runtime GPU con API simple para SPIR-V, Vulkan, CUDA.
"""

from .gpu_buffer import GPUBuffer, BufferPool, BufferUsage, MemoryLocation
from .gpu_kernel import GPUKernel, ComputePipeline, KernelType, ADeadGpuOp
from .gpu_kernel import kernel_vector_add, kernel_vector_mul, kernel_matmul, kernel_saxpy
from .gpu_optimizer import GPUOptimizer, SPIRVOptimizer, LayoutType
from .gpu_runtime import GPU, GPUBackend, GPUDeviceInfo, GPUFence, GPUEvent, GPUStream

__all__ = [
    # Runtime
    "GPU",
    "GPUBackend",
    "GPUDeviceInfo",
    # Buffers
    "GPUBuffer",
    "BufferPool",
    "BufferUsage",
    "MemoryLocation",
    # Kernels
    "GPUKernel",
    "ComputePipeline",
    "KernelType",
    "ADeadGpuOp",
    # Predefined kernels
    "kernel_vector_add",
    "kernel_vector_mul",
    "kernel_matmul",
    "kernel_saxpy",
    # Optimizer
    "GPUOptimizer",
    "SPIRVOptimizer",
    "LayoutType",
    # Sync
    "GPUFence",
    "GPUEvent",
    "GPUStream",
]

__version__ = "1.0.0"
