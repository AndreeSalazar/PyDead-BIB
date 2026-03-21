"""
GPU Binary Layout Optimizer
============================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪

Optimizador de layout de datos para GPU.
Similar a Binary Layout Optimizer de CPU pero para GPU.
"""

import numpy as np
from typing import Tuple, Optional, List
from dataclasses import dataclass
from enum import Enum


class LayoutType(Enum):
    """Tipos de layout de memoria."""
    ROW_MAJOR = "row_major"      # C-style (default)
    COL_MAJOR = "col_major"      # Fortran-style
    TILED = "tiled"              # Bloques para cache
    MORTON = "morton"            # Z-order curve
    HILBERT = "hilbert"          # Hilbert curve


@dataclass
class OptimizationResult:
    """Resultado de optimización."""
    original_size: int
    optimized_size: int
    layout: LayoutType
    tile_size: Tuple[int, int]
    alignment: int
    padding: int
    speedup_estimate: float


class GPUOptimizer:
    """
    Optimizador de layout de datos para GPU.
    Optimiza acceso a memoria para coalescing y cache.
    """
    
    # Tamaños típicos de cache line GPU
    CACHE_LINE_SIZE = 128  # bytes
    WARP_SIZE = 32         # threads por warp (NVIDIA)
    
    def __init__(self, 
                 cache_line: int = 128,
                 warp_size: int = 32,
                 tile_size: int = 32):
        self.cache_line = cache_line
        self.warp_size = warp_size
        self.default_tile_size = tile_size
    
    def optimize_matrix(self, 
                       data: np.ndarray,
                       tile_size: Optional[int] = None,
                       layout: LayoutType = LayoutType.TILED) -> np.ndarray:
        """
        Optimiza layout de matriz para GPU.
        
        Args:
            data: Matriz a optimizar
            tile_size: Tamaño de tile (default: 32)
            layout: Tipo de layout
            
        Returns:
            Matriz con layout optimizado
        """
        tile_size = tile_size or self.default_tile_size
        
        if layout == LayoutType.TILED:
            return self._tile_matrix(data, tile_size)
        elif layout == LayoutType.MORTON:
            return self._morton_order(data)
        elif layout == LayoutType.COL_MAJOR:
            return np.asfortranarray(data)
        else:
            return np.ascontiguousarray(data)
    
    def _tile_matrix(self, data: np.ndarray, tile_size: int) -> np.ndarray:
        """Reorganiza matriz en tiles para mejor cache."""
        if data.ndim != 2:
            return data
        
        rows, cols = data.shape
        
        # Pad a múltiplo de tile_size
        pad_rows = (tile_size - rows % tile_size) % tile_size
        pad_cols = (tile_size - cols % tile_size) % tile_size
        
        if pad_rows > 0 or pad_cols > 0:
            data = np.pad(data, ((0, pad_rows), (0, pad_cols)), mode='constant')
        
        new_rows, new_cols = data.shape
        
        # Reorganizar en tiles
        tiled = data.reshape(
            new_rows // tile_size, tile_size,
            new_cols // tile_size, tile_size
        ).transpose(0, 2, 1, 3).reshape(-1)
        
        return tiled.reshape(new_rows, new_cols)
    
    def _morton_order(self, data: np.ndarray) -> np.ndarray:
        """Reorganiza en orden Z (Morton curve)."""
        if data.ndim != 2:
            return data
        
        rows, cols = data.shape
        
        # Simplificado: solo funciona para potencias de 2
        size = max(rows, cols)
        size = 1 << (size - 1).bit_length()  # Siguiente potencia de 2
        
        # Pad
        padded = np.zeros((size, size), dtype=data.dtype)
        padded[:rows, :cols] = data
        
        # Morton reorder (simplificado)
        result = np.zeros_like(padded)
        for i in range(size):
            for j in range(size):
                morton_idx = self._interleave_bits(i, j)
                flat_idx = morton_idx % (size * size)
                result.flat[flat_idx] = padded[i, j]
        
        return result[:rows, :cols]
    
    @staticmethod
    def _interleave_bits(x: int, y: int) -> int:
        """Intercala bits de x e y para índice Morton."""
        result = 0
        for i in range(16):
            result |= ((x >> i) & 1) << (2 * i)
            result |= ((y >> i) & 1) << (2 * i + 1)
        return result
    
    def align(self, data: np.ndarray, alignment: int = 128) -> np.ndarray:
        """
        Alinea datos a boundary especificado.
        
        Args:
            data: Datos a alinear
            alignment: Alineación en bytes
            
        Returns:
            Datos alineados (con padding si necesario)
        """
        data = np.ascontiguousarray(data)
        
        # Calcular padding necesario
        current_size = data.nbytes
        aligned_size = ((current_size + alignment - 1) // alignment) * alignment
        padding = aligned_size - current_size
        
        if padding == 0:
            return data
        
        # Agregar padding
        flat = data.flatten()
        pad_elements = padding // data.itemsize
        if pad_elements > 0:
            padded = np.concatenate([flat, np.zeros(pad_elements, dtype=data.dtype)])
            return padded.reshape(-1)
        
        return data
    
    def coalesce(self, data: np.ndarray, stride: int = 32) -> np.ndarray:
        """
        Reorganiza datos para acceso coalescente.
        
        En GPU, threads consecutivos deben acceder a memoria consecutiva.
        
        Args:
            data: Datos a reorganizar
            stride: Stride de acceso (típicamente warp_size)
            
        Returns:
            Datos reorganizados para coalescing
        """
        if data.ndim != 2:
            return data
        
        rows, cols = data.shape
        
        # Asegurar que cols es múltiplo de stride
        if cols % stride != 0:
            pad_cols = stride - (cols % stride)
            data = np.pad(data, ((0, 0), (0, pad_cols)), mode='constant')
            cols = data.shape[1]
        
        # Reorganizar para acceso coalescente
        # Cada grupo de 'stride' elementos consecutivos en memoria
        return np.ascontiguousarray(data)
    
    def optimize_for_matmul(self, 
                           A: np.ndarray, 
                           B: np.ndarray,
                           tile_size: int = 32) -> Tuple[np.ndarray, np.ndarray]:
        """
        Optimiza matrices A y B para multiplicación.
        
        Args:
            A: Matriz izquierda (M x K)
            B: Matriz derecha (K x N)
            tile_size: Tamaño de tile
            
        Returns:
            (A_opt, B_opt) matrices optimizadas
        """
        # A: tile en row-major
        A_opt = self._tile_matrix(A, tile_size)
        
        # B: transponer y tile para mejor acceso
        B_opt = self._tile_matrix(B.T, tile_size).T
        
        return A_opt, B_opt
    
    def analyze(self, data: np.ndarray) -> OptimizationResult:
        """
        Analiza datos y sugiere optimizaciones.
        
        Args:
            data: Datos a analizar
            
        Returns:
            Resultado con sugerencias
        """
        original_size = data.nbytes
        
        # Determinar mejor tile size
        if data.ndim == 2:
            rows, cols = data.shape
            tile_size = min(32, rows, cols)
        else:
            tile_size = 32
        
        # Calcular padding necesario
        aligned_size = ((original_size + self.cache_line - 1) // self.cache_line) * self.cache_line
        padding = aligned_size - original_size
        
        # Estimar speedup
        # Acceso no coalescente: ~10x más lento
        # Tiling: ~2-4x mejora en cache
        speedup = 1.0
        if data.ndim == 2:
            speedup = 2.5  # Estimación conservadora
        
        return OptimizationResult(
            original_size=original_size,
            optimized_size=aligned_size,
            layout=LayoutType.TILED if data.ndim == 2 else LayoutType.ROW_MAJOR,
            tile_size=(tile_size, tile_size),
            alignment=self.cache_line,
            padding=padding,
            speedup_estimate=speedup
        )
    
    def pack_structs(self, 
                    fields: List[Tuple[str, np.dtype, int]],
                    alignment: int = 16) -> Tuple[int, List[int]]:
        """
        Calcula layout óptimo para struct de GPU.
        
        Args:
            fields: Lista de (nombre, dtype, count)
            alignment: Alineación de struct
            
        Returns:
            (total_size, offsets) tamaño total y offsets de cada campo
        """
        offsets = []
        current_offset = 0
        
        for name, dtype, count in fields:
            field_size = np.dtype(dtype).itemsize * count
            field_align = min(alignment, np.dtype(dtype).itemsize)
            
            # Alinear offset
            if current_offset % field_align != 0:
                current_offset += field_align - (current_offset % field_align)
            
            offsets.append(current_offset)
            current_offset += field_size
        
        # Alinear tamaño total
        if current_offset % alignment != 0:
            current_offset += alignment - (current_offset % alignment)
        
        return current_offset, offsets


class SPIRVOptimizer:
    """
    Optimizador de bytecode SPIR-V.
    Minimiza y optimiza shaders para GPU.
    """
    
    SPIRV_MAGIC = 0x07230203
    
    def __init__(self):
        self.stats = {
            "instructions_removed": 0,
            "size_reduction": 0,
        }
    
    def optimize(self, spirv: bytes) -> bytes:
        """
        Optimiza bytecode SPIR-V.
        
        Args:
            spirv: Bytecode SPIR-V
            
        Returns:
            Bytecode optimizado
        """
        if len(spirv) < 20:
            return spirv
        
        # Verificar magic
        import struct
        magic = struct.unpack('<I', spirv[:4])[0]
        if magic != self.SPIRV_MAGIC:
            return spirv
        
        # Por ahora, solo strip de decoraciones innecesarias
        # En implementación completa: DCE, constant folding, etc.
        
        original_size = len(spirv)
        optimized = self._strip_debug_info(spirv)
        
        self.stats["size_reduction"] = original_size - len(optimized)
        
        return optimized
    
    def _strip_debug_info(self, spirv: bytes) -> bytes:
        """Elimina información de debug del SPIR-V."""
        # Simplificado: en realidad necesita parsear instrucciones
        # OpName, OpMemberName, OpLine, etc.
        return spirv
    
    def analyze(self, spirv: bytes) -> dict:
        """Analiza bytecode SPIR-V."""
        if len(spirv) < 20:
            return {"error": "SPIR-V demasiado corto"}
        
        import struct
        header = struct.unpack('<5I', spirv[:20])
        
        return {
            "magic": hex(header[0]),
            "version": f"{(header[1] >> 16) & 0xFF}.{(header[1] >> 8) & 0xFF}",
            "generator": header[2],
            "bound": header[3],
            "size_bytes": len(spirv),
            "instruction_words": (len(spirv) - 20) // 4,
        }
