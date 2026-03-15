"""
Metal_Dead Core - Módulos principales
CPU-First con integración ADead-BIB FFI
"""

from .metal_dead import MetalDead, MetalDeadConfig
from .memory import PersistentMemory, MemoryItem
from .context import PersonalContext, UserProfile
from .tokenizer import SmartTokenizer
from .model import LightTransformer
from .intelligence import IntelligenceEngine, CriticalThinking, KnowledgeBase
from .metal_dead_smart import MetalDeadSmart

# CPU-First modules (v1.0)
from .cpu_compute import CPUCompute, CPUTransformer, ComputeBackend
from .metal_dead_cpu import MetalDeadCPU, MetalDeadCPUConfig

__all__ = [
    # Original
    "MetalDead",
    "MetalDeadConfig",
    "MetalDeadSmart",
    "PersistentMemory",
    "MemoryItem",
    "PersonalContext",
    "UserProfile",
    "SmartTokenizer",
    "LightTransformer",
    "IntelligenceEngine",
    "CriticalThinking",
    "KnowledgeBase",
    # CPU-First
    "CPUCompute",
    "CPUTransformer",
    "ComputeBackend",
    "MetalDeadCPU",
    "MetalDeadCPUConfig",
]
