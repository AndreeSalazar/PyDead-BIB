"""
GPU Kernel - Carga y Ejecución de Kernels SPIR-V
=================================================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪

Sistema de carga y ejecución de kernels SPIR-V/ADead.
"""

import struct
from pathlib import Path
from typing import List, Optional, Tuple, Dict, Any
from dataclasses import dataclass
from enum import Enum

import numpy as np

from .gpu_buffer import GPUBuffer


class KernelType(Enum):
    """Tipo de kernel."""
    SPIRV = "spirv"          # SPIR-V bytecode
    ADEAD = "adead"          # ADead-BIB bytecode
    WGSL = "wgsl"            # WebGPU Shading Language
    GLSL = "glsl"            # GLSL compute shader


@dataclass
class KernelInfo:
    """Información del kernel."""
    name: str
    type: KernelType
    entry_point: str
    workgroup_size: Tuple[int, int, int]
    num_bindings: int
    bytecode_size: int


class GPUKernel:
    """
    Kernel GPU cargado y listo para ejecución.
    """
    
    SPIRV_MAGIC = 0x07230203
    
    def __init__(self, bytecode: bytes, 
                 kernel_type: KernelType = KernelType.SPIRV,
                 entry_point: str = "main",
                 workgroup_size: Tuple[int, int, int] = (256, 1, 1)):
        """
        Crea un kernel desde bytecode.
        
        Args:
            bytecode: Bytecode del kernel
            kernel_type: Tipo de kernel
            entry_point: Punto de entrada
            workgroup_size: Tamaño de workgroup
        """
        self.bytecode = bytecode
        self.kernel_type = kernel_type
        self.entry_point = entry_point
        self.workgroup_size = workgroup_size
        self._is_compiled = False
        self._pipeline_handle = None
        
        # Validar bytecode
        if kernel_type == KernelType.SPIRV:
            self._validate_spirv()
        
        # Extraer metadata
        self._extract_metadata()
    
    def _validate_spirv(self):
        """Valida que el bytecode sea SPIR-V válido."""
        if len(self.bytecode) < 20:
            raise ValueError("Bytecode SPIR-V demasiado corto")
        
        magic = struct.unpack('<I', self.bytecode[:4])[0]
        if magic != self.SPIRV_MAGIC:
            raise ValueError(f"Magic number inválido: {hex(magic)}, esperado: {hex(self.SPIRV_MAGIC)}")
    
    def _extract_metadata(self):
        """Extrae metadata del kernel."""
        self.num_bindings = 0
        self.local_size = self.workgroup_size
        
        if self.kernel_type == KernelType.SPIRV and len(self.bytecode) >= 20:
            # Parsear header SPIR-V
            header = struct.unpack('<5I', self.bytecode[:20])
            self._spirv_version = header[1]
            self._spirv_generator = header[2]
            self._spirv_bound = header[3]
    
    @classmethod
    def from_file(cls, path: str, **kwargs) -> 'GPUKernel':
        """Carga kernel desde archivo."""
        path = Path(path)
        
        if not path.exists():
            raise FileNotFoundError(f"Kernel no encontrado: {path}")
        
        # Detectar tipo por extensión
        ext = path.suffix.lower()
        kernel_type = {
            '.spv': KernelType.SPIRV,
            '.spirv': KernelType.SPIRV,
            '.adb': KernelType.ADEAD,
            '.wgsl': KernelType.WGSL,
            '.glsl': KernelType.GLSL,
            '.comp': KernelType.GLSL,
        }.get(ext, KernelType.SPIRV)
        
        with open(path, 'rb') as f:
            bytecode = f.read()
        
        return cls(bytecode, kernel_type=kernel_type, **kwargs)
    
    @classmethod
    def from_adead(cls, ops: List[Tuple[int, int]], 
                   workgroup_size: Tuple[int, int, int] = (256, 1, 1)) -> 'GPUKernel':
        """
        Crea kernel desde opcodes ADead-BIB.
        
        Args:
            ops: Lista de (opcode, operand)
            workgroup_size: Tamaño de workgroup
        """
        # Codificar opcodes ADead (4 bits opcode + 4 bits operand)
        bytecode = bytes([(op << 4) | (operand & 0x0F) for op, operand in ops])
        return cls(bytecode, kernel_type=KernelType.ADEAD, workgroup_size=workgroup_size)
    
    def info(self) -> KernelInfo:
        """Retorna información del kernel."""
        return KernelInfo(
            name=self.entry_point,
            type=self.kernel_type,
            entry_point=self.entry_point,
            workgroup_size=self.workgroup_size,
            num_bindings=self.num_bindings,
            bytecode_size=len(self.bytecode)
        )
    
    def __repr__(self) -> str:
        return f"GPUKernel({self.entry_point}, {self.kernel_type.value}, {len(self.bytecode)} bytes)"


class ComputePipeline:
    """
    Pipeline de compute para ejecución de kernels.
    Encapsula: descriptor sets, pipeline state, command buffers.
    """
    
    def __init__(self, kernel: GPUKernel):
        self.kernel = kernel
        self._bindings: List[GPUBuffer] = []
        self._is_bound = False
        self._dispatch_count = 0
    
    def bind(self, *buffers: GPUBuffer):
        """
        Vincula buffers al pipeline.
        
        Args:
            *buffers: Buffers a vincular (en orden de binding)
        """
        self._bindings = list(buffers)
        self._is_bound = True
    
    def dispatch(self, groups: Tuple[int, int, int]) -> 'DispatchInfo':
        """
        Ejecuta el kernel.
        
        Args:
            groups: Número de workgroups (x, y, z)
            
        Returns:
            Información del dispatch
        """
        if not self._is_bound:
            raise RuntimeError("Pipeline no tiene buffers vinculados")
        
        # Calcular número total de invocaciones
        wg = self.kernel.workgroup_size
        total_invocations = (
            groups[0] * groups[1] * groups[2] *
            wg[0] * wg[1] * wg[2]
        )
        
        self._dispatch_count += 1
        
        return DispatchInfo(
            groups=groups,
            workgroup_size=wg,
            total_invocations=total_invocations,
            dispatch_id=self._dispatch_count
        )
    
    def __repr__(self) -> str:
        return f"ComputePipeline({self.kernel.entry_point}, bindings={len(self._bindings)})"


@dataclass
class DispatchInfo:
    """Información de un dispatch."""
    groups: Tuple[int, int, int]
    workgroup_size: Tuple[int, int, int]
    total_invocations: int
    dispatch_id: int


# ============================================
# KERNELS PREDEFINIDOS (ADead-BIB opcodes)
# ============================================

class ADeadGpuOp:
    """Opcodes ADead para GPU."""
    EXIT = 0x0
    LOAD = 0x1
    STORE = 0x2
    LOAD_IMM = 0x3
    ADD = 0x4
    SUB = 0x5
    MUL = 0x6
    DIV = 0x7
    VEC_ADD = 0x8
    VEC_MUL = 0x9
    DOT = 0xA
    MATMUL = 0xB
    SYNC = 0xC
    NOP = 0xD


def kernel_vector_add() -> GPUKernel:
    """Kernel: C[i] = A[i] + B[i]"""
    return GPUKernel.from_adead([
        (ADeadGpuOp.LOAD, 0),      # acc = A[gid]
        (ADeadGpuOp.ADD, 1),       # acc += B[gid]
        (ADeadGpuOp.STORE, 2),     # C[gid] = acc (buffer 2)
        (ADeadGpuOp.EXIT, 0),
    ])


def kernel_vector_mul() -> GPUKernel:
    """Kernel: C[i] = A[i] * B[i]"""
    return GPUKernel.from_adead([
        (ADeadGpuOp.LOAD, 0),
        (ADeadGpuOp.MUL, 1),
        (ADeadGpuOp.STORE, 2),     # C[gid] = acc (buffer 2)
        (ADeadGpuOp.EXIT, 0),
    ])


def kernel_saxpy(alpha: int = 2) -> GPUKernel:
    """Kernel: Y[i] = alpha * X[i] + Y[i]"""
    return GPUKernel.from_adead([
        (ADeadGpuOp.LOAD, 0),          # acc = X[gid]
        (ADeadGpuOp.LOAD_IMM, alpha),  # acc = alpha
        (ADeadGpuOp.MUL, 0),           # acc *= X[gid]
        (ADeadGpuOp.ADD, 1),           # acc += Y[gid]
        (ADeadGpuOp.STORE, 1),         # Y[gid] = acc
        (ADeadGpuOp.EXIT, 0),
    ])


def kernel_matmul() -> GPUKernel:
    """Kernel: C = A * B (matrix multiplication)"""
    return GPUKernel.from_adead([
        (ADeadGpuOp.LOAD, 0),      # Load A
        (ADeadGpuOp.MATMUL, 1),    # MatMul with B
        (ADeadGpuOp.STORE, 2),     # Store to C
        (ADeadGpuOp.EXIT, 0),
    ], workgroup_size=(16, 16, 1))


def kernel_reduce_sum() -> GPUKernel:
    """Kernel: sum = reduce_add(A)"""
    return GPUKernel.from_adead([
        (ADeadGpuOp.LOAD, 0),      # acc = A[gid]
        (ADeadGpuOp.SYNC, 0),      # barrier
        (ADeadGpuOp.ADD, 0),       # reduction
        (ADeadGpuOp.STORE, 1),     # output
        (ADeadGpuOp.EXIT, 0),
    ])
