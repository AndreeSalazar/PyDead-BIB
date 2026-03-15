"""
Metal_Dead Integrations - Integraciones externas
"""

from .gpu_compute import GPUCompute, GPUTransformer, MetalDeadGPU
from .gpu_advanced import GPUAdvanced, GPUConfig, MetalDeadGPUMax
from .adead_accelerator import ADeadAccelerator, MetalDeadADead
from .metal_dead_smart_gpu import MetalDeadSmartGPU

__all__ = [
    "GPUCompute",
    "GPUTransformer",
    "MetalDeadGPU",
    "GPUAdvanced",
    "GPUConfig",
    "MetalDeadGPUMax",
    "ADeadAccelerator",
    "MetalDeadADead",
    "MetalDeadSmartGPU",
]
