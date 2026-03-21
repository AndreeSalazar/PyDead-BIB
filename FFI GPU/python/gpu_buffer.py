"""
GPU Buffer - Gestión de Memoria GPU
====================================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪

Gestión de buffers GPU con transferencias CPU ↔ GPU.
"""

import numpy as np
from typing import Optional, Union, Tuple
from dataclasses import dataclass
from enum import Enum


class BufferUsage(Enum):
    """Uso del buffer."""
    STORAGE = "storage"      # Lectura/escritura en shader
    UNIFORM = "uniform"      # Solo lectura, pequeño
    VERTEX = "vertex"        # Datos de vértices
    INDEX = "index"          # Índices
    STAGING = "staging"      # Transferencia CPU ↔ GPU


class MemoryLocation(Enum):
    """Ubicación de memoria."""
    DEVICE = "device"        # Solo GPU (más rápido)
    HOST = "host"            # CPU visible
    SHARED = "shared"        # Compartida CPU/GPU


@dataclass
class BufferInfo:
    """Información del buffer."""
    size: int
    usage: BufferUsage
    location: MemoryLocation
    alignment: int
    is_mapped: bool


class GPUBuffer:
    """
    Buffer de memoria GPU.
    Maneja transferencias CPU ↔ GPU y alineación.
    """
    
    def __init__(self, 
                 data: Optional[np.ndarray] = None,
                 size: Optional[int] = None,
                 dtype: np.dtype = np.float32,
                 usage: BufferUsage = BufferUsage.STORAGE,
                 location: MemoryLocation = MemoryLocation.DEVICE):
        """
        Crea un buffer GPU.
        
        Args:
            data: Datos iniciales (opcional)
            size: Tamaño en elementos (si no hay data)
            dtype: Tipo de datos
            usage: Uso del buffer
            location: Ubicación de memoria
        """
        self.dtype = dtype
        self.usage = usage
        self.location = location
        self._is_freed = False
        
        # Determinar tamaño
        if data is not None:
            self._data = np.ascontiguousarray(data, dtype=dtype)
            self.size = self._data.nbytes
            self.shape = self._data.shape
            self.element_count = self._data.size
        elif size is not None:
            self.element_count = size
            self.size = size * np.dtype(dtype).itemsize
            self.shape = (size,)
            self._data = np.zeros(size, dtype=dtype)
        else:
            raise ValueError("Debe proporcionar data o size")
        
        # Alineación (128 bytes para GPU)
        self.alignment = 128
        self.aligned_size = ((self.size + self.alignment - 1) // self.alignment) * self.alignment
        
        # Simular handle GPU (en implementación real sería vk::Buffer o similar)
        self._gpu_handle = id(self)
        self._is_on_gpu = False
        
    def write(self, data: np.ndarray, offset: int = 0):
        """
        Escribe datos al buffer (CPU → GPU).
        
        Args:
            data: Datos a escribir
            offset: Offset en elementos
        """
        if self._is_freed:
            raise RuntimeError("Buffer ya liberado")
        
        data = np.ascontiguousarray(data, dtype=self.dtype)
        end = offset + data.size
        
        if end > self.element_count:
            raise ValueError(f"Datos exceden tamaño del buffer: {end} > {self.element_count}")
        
        self._data.flat[offset:end] = data.flat
        self._is_on_gpu = True
    
    def read(self, size: Optional[int] = None, offset: int = 0) -> np.ndarray:
        """
        Lee datos del buffer (GPU → CPU).
        
        Args:
            size: Número de elementos a leer (None = todo)
            offset: Offset en elementos
            
        Returns:
            Datos leídos
        """
        if self._is_freed:
            raise RuntimeError("Buffer ya liberado")
        
        if size is None:
            return self._data.copy()
        
        end = offset + size
        if end > self.element_count:
            raise ValueError(f"Lectura excede tamaño: {end} > {self.element_count}")
        
        return self._data.flat[offset:end].copy()
    
    def fill(self, value: float):
        """Llena el buffer con un valor."""
        if self._is_freed:
            raise RuntimeError("Buffer ya liberado")
        self._data.fill(value)
        self._is_on_gpu = True
    
    def zero(self):
        """Llena el buffer con ceros."""
        self.fill(0)
    
    def copy_to(self, other: 'GPUBuffer'):
        """Copia este buffer a otro."""
        if self._is_freed or other._is_freed:
            raise RuntimeError("Buffer ya liberado")
        if self.size != other.size:
            raise ValueError("Tamaños incompatibles")
        other._data[:] = self._data
        other._is_on_gpu = True
    
    def free(self):
        """Libera el buffer."""
        if not self._is_freed:
            self._data = None
            self._is_freed = True
            self._is_on_gpu = False
    
    def info(self) -> BufferInfo:
        """Retorna información del buffer."""
        return BufferInfo(
            size=self.size,
            usage=self.usage,
            location=self.location,
            alignment=self.alignment,
            is_mapped=not self._is_freed
        )
    
    def __len__(self) -> int:
        return self.element_count
    
    def __del__(self):
        self.free()
    
    def __repr__(self) -> str:
        status = "freed" if self._is_freed else "active"
        return f"GPUBuffer({self.shape}, dtype={self.dtype}, {self.size} bytes, {status})"


class BufferPool:
    """
    Pool de buffers para reutilización.
    Evita allocaciones frecuentes.
    """
    
    def __init__(self, max_buffers: int = 100):
        self.max_buffers = max_buffers
        self._pool: dict = {}  # size -> list of buffers
        self._allocated = 0
        self._reused = 0
    
    def acquire(self, size: int, dtype: np.dtype = np.float32) -> GPUBuffer:
        """Obtiene un buffer del pool o crea uno nuevo."""
        key = (size, dtype)
        
        if key in self._pool and self._pool[key]:
            self._reused += 1
            buf = self._pool[key].pop()
            buf.zero()
            return buf
        
        self._allocated += 1
        return GPUBuffer(size=size, dtype=dtype)
    
    def release(self, buffer: GPUBuffer):
        """Devuelve un buffer al pool."""
        if buffer._is_freed:
            return
        
        key = (buffer.element_count, buffer.dtype)
        
        if key not in self._pool:
            self._pool[key] = []
        
        if len(self._pool[key]) < self.max_buffers:
            self._pool[key].append(buffer)
        else:
            buffer.free()
    
    def clear(self):
        """Libera todos los buffers del pool."""
        for buffers in self._pool.values():
            for buf in buffers:
                buf.free()
        self._pool.clear()
    
    def stats(self) -> dict:
        """Estadísticas del pool."""
        total_pooled = sum(len(b) for b in self._pool.values())
        return {
            "allocated": self._allocated,
            "reused": self._reused,
            "pooled": total_pooled,
            "reuse_rate": self._reused / max(1, self._allocated + self._reused)
        }


# Funciones de utilidad para alineación
def align_size(size: int, alignment: int = 128) -> int:
    """Alinea tamaño a múltiplo de alignment."""
    return ((size + alignment - 1) // alignment) * alignment


def is_aligned(ptr: int, alignment: int = 128) -> bool:
    """Verifica si un puntero está alineado."""
    return ptr % alignment == 0


def optimal_alignment(dtype: np.dtype) -> int:
    """Retorna alineación óptima para un tipo de datos."""
    itemsize = np.dtype(dtype).itemsize
    # GPU prefiere 128 bytes, pero mínimo el tamaño del elemento
    return max(128, itemsize)
