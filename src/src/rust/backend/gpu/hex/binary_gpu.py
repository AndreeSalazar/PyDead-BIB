"""
ADead-BIB Binary GPU - Generador de Binarios Hibridos CPU+GPU
=============================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with love in Peru

Genera binarios ejecutables que combinan codigo CPU (x86-64) con
llamadas GPU (CUDA) para maximo rendimiento.
"""

import struct
import numpy as np
from typing import List, Dict, Optional
from dataclasses import dataclass
from enum import IntEnum
from pathlib import Path

# Importar opcodes GPU
from gpu_opcodes import GPUOpcode, GPUProgram, GPUInstruction


class BinarySection(IntEnum):
    """Secciones del binario hibrido."""
    HEADER = 0x00
    CPU_CODE = 0x01
    GPU_CODE = 0x02
    DATA = 0x03
    SYMBOLS = 0x04


@dataclass
class HybridBinary:
    """Binario hibrido CPU + GPU."""
    name: str
    cpu_code: bytes
    gpu_code: bytes
    data: bytes
    symbols: Dict[str, int]
    
    def to_bytes(self) -> bytes:
        """Genera binario completo."""
        # Header
        header = struct.pack(
            '<4sHHIIII',
            b'AHYB',  # Magic: ADead Hybrid
            1,        # Version
            0,        # Flags
            len(self.cpu_code),
            len(self.gpu_code),
            len(self.data),
            len(self.symbols)
        )
        
        # Sections
        binary = header
        binary += self.cpu_code
        binary += self.gpu_code
        binary += self.data
        
        # Symbols
        for name, addr in self.symbols.items():
            name_bytes = name.encode('utf-8')[:32].ljust(32, b'\x00')
            binary += name_bytes + struct.pack('<I', addr)
        
        return binary
    
    def save(self, filename: str):
        """Guarda binario a archivo."""
        with open(filename, 'wb') as f:
            f.write(self.to_bytes())
        print(f"Guardado: {filename} ({len(self.to_bytes())} bytes)")


class HybridCodegen:
    """Generador de codigo hibrido CPU + GPU."""
    
    def __init__(self):
        self.cpu_code = bytearray()
        self.gpu_program = GPUProgram("hybrid")
        self.data = bytearray()
        self.symbols: Dict[str, int] = {}
        self.current_offset = 0
    
    # =========================================================================
    # CPU Code Generation (x86-64)
    # =========================================================================
    
    def emit_cpu(self, *opcodes: int):
        """Emite opcodes CPU."""
        for op in opcodes:
            self.cpu_code.append(op & 0xFF)
        self.current_offset += len(opcodes)
    
    def emit_cpu_bytes(self, data: bytes):
        """Emite bytes CPU."""
        self.cpu_code.extend(data)
        self.current_offset += len(data)
    
    def cpu_push_rbp(self):
        """push rbp"""
        self.emit_cpu(0x55)
    
    def cpu_mov_rbp_rsp(self):
        """mov rbp, rsp"""
        self.emit_cpu(0x48, 0x89, 0xE5)
    
    def cpu_pop_rbp(self):
        """pop rbp"""
        self.emit_cpu(0x5D)
    
    def cpu_ret(self):
        """ret"""
        self.emit_cpu(0xC3)
    
    def cpu_mov_rax_imm64(self, value: int):
        """mov rax, imm64"""
        self.emit_cpu(0x48, 0xB8)
        self.emit_cpu_bytes(struct.pack('<Q', value))
    
    def cpu_call_rax(self):
        """call rax"""
        self.emit_cpu(0xFF, 0xD0)
    
    def cpu_function_prologue(self):
        """Prologo de funcion."""
        self.cpu_push_rbp()
        self.cpu_mov_rbp_rsp()
    
    def cpu_function_epilogue(self):
        """Epilogo de funcion."""
        self.cpu_pop_rbp()
        self.cpu_ret()
    
    # =========================================================================
    # GPU Code Generation
    # =========================================================================
    
    def gpu_init(self):
        """Inicializa GPU."""
        self.gpu_program.init()
    
    def gpu_alloc(self, size: int, reg: int):
        """Reserva memoria GPU."""
        self.gpu_program.alloc(size, reg)
    
    def gpu_matmul(self, a: int, b: int, c: int, m: int, n: int, k: int):
        """MatMul en GPU."""
        self.gpu_program.matmul(a, b, c, m, n, k)
    
    def gpu_attention(self, q: int, k: int, v: int, out: int, seq: int, dim: int):
        """Atencion en GPU."""
        self.gpu_program.attention(q, k, v, out, seq, dim)
    
    def gpu_sync(self):
        """Sincroniza GPU."""
        self.gpu_program.sync()
    
    def gpu_end(self):
        """Termina programa GPU."""
        self.gpu_program.end()
    
    # =========================================================================
    # Hybrid Operations
    # =========================================================================
    
    def add_symbol(self, name: str, address: int):
        """Agrega simbolo."""
        self.symbols[name] = address
    
    def add_data(self, data: bytes) -> int:
        """Agrega datos y retorna offset."""
        offset = len(self.data)
        self.data.extend(data)
        return offset
    
    def add_float_array(self, arr: np.ndarray) -> int:
        """Agrega array de floats."""
        return self.add_data(arr.astype(np.float32).tobytes())
    
    def generate_matmul_hybrid(self, m: int, n: int, k: int) -> HybridBinary:
        """Genera binario hibrido para MatMul."""
        # CPU: Preparar datos y llamar GPU
        self.cpu_function_prologue()
        
        # Simbolo para entrada
        self.add_symbol("matmul_entry", 0)
        
        # GPU: Ejecutar MatMul
        size_a = m * k * 4
        size_b = k * n * 4
        size_c = m * n * 4
        
        self.gpu_init()
        self.gpu_alloc(size_a, 0)
        self.gpu_alloc(size_b, 1)
        self.gpu_alloc(size_c, 2)
        self.gpu_program.copy_to_gpu(0x1000, 0, size_a)
        self.gpu_program.copy_to_gpu(0x2000, 1, size_b)
        self.gpu_matmul(0, 1, 2, m, n, k)
        self.gpu_sync()
        self.gpu_program.copy_from_gpu(2, 0x3000, size_c)
        self.gpu_program.free(0)
        self.gpu_program.free(1)
        self.gpu_program.free(2)
        self.gpu_end()
        
        # CPU: Retornar
        self.cpu_function_epilogue()
        
        self.add_symbol("matmul_exit", len(self.cpu_code))
        
        return HybridBinary(
            name=f"matmul_{m}x{n}x{k}",
            cpu_code=bytes(self.cpu_code),
            gpu_code=self.gpu_program.to_bytes(),
            data=bytes(self.data),
            symbols=self.symbols
        )
    
    def generate_attention_hybrid(self, seq_len: int, dim: int) -> HybridBinary:
        """Genera binario hibrido para Attention."""
        self.cpu_function_prologue()
        self.add_symbol("attention_entry", 0)
        
        size = seq_len * dim * 4
        
        self.gpu_init()
        self.gpu_alloc(size, 0)  # Q
        self.gpu_alloc(size, 1)  # K
        self.gpu_alloc(size, 2)  # V
        self.gpu_alloc(size, 3)  # Output
        self.gpu_program.copy_to_gpu(0x1000, 0, size)
        self.gpu_program.copy_to_gpu(0x2000, 1, size)
        self.gpu_program.copy_to_gpu(0x3000, 2, size)
        self.gpu_attention(0, 1, 2, 3, seq_len, dim)
        self.gpu_sync()
        self.gpu_program.copy_from_gpu(3, 0x4000, size)
        self.gpu_program.free(0)
        self.gpu_program.free(1)
        self.gpu_program.free(2)
        self.gpu_program.free(3)
        self.gpu_end()
        
        self.cpu_function_epilogue()
        self.add_symbol("attention_exit", len(self.cpu_code))
        
        return HybridBinary(
            name=f"attention_{seq_len}x{dim}",
            cpu_code=bytes(self.cpu_code),
            gpu_code=self.gpu_program.to_bytes(),
            data=bytes(self.data),
            symbols=self.symbols
        )


def generate_all_binaries():
    """Genera todos los binarios hibridos."""
    print("=" * 70)
    print("   ADead-BIB Hybrid Binary Generator")
    print("   Author: Eddi Andree Salazar Matos")
    print("=" * 70)
    
    output_dir = Path(__file__).parent / "binaries"
    output_dir.mkdir(exist_ok=True)
    
    # MatMul binaries
    print("\n1. Generando binarios MatMul:")
    for size in [256, 512, 1024, 2048]:
        codegen = HybridCodegen()
        binary = codegen.generate_matmul_hybrid(size, size, size)
        binary.save(str(output_dir / f"matmul_{size}.ahyb"))
    
    # Attention binaries
    print("\n2. Generando binarios Attention:")
    for seq, dim in [(256, 64), (512, 128), (1024, 256)]:
        codegen = HybridCodegen()
        binary = codegen.generate_attention_hybrid(seq, dim)
        binary.save(str(output_dir / f"attention_{seq}x{dim}.ahyb"))
    
    print("\n" + "=" * 70)
    print(f"   Binarios generados en: {output_dir}")
    print("=" * 70)


def demo():
    """Demo de generacion de binarios."""
    print("=" * 70)
    print("   ADead-BIB Binary GPU Demo")
    print("=" * 70)
    
    # Generar un binario de ejemplo
    codegen = HybridCodegen()
    binary = codegen.generate_matmul_hybrid(1024, 1024, 1024)
    
    print(f"\nBinario: {binary.name}")
    print(f"CPU code: {len(binary.cpu_code)} bytes")
    print(f"GPU code: {len(binary.gpu_code)} bytes")
    print(f"Data: {len(binary.data)} bytes")
    print(f"Symbols: {len(binary.symbols)}")
    
    # Mostrar hex dump
    print("\nCPU Code (hex):")
    print(" ".join(f"{b:02X}" for b in binary.cpu_code[:32]))
    
    print("\nGPU Code (hex):")
    print(" ".join(f"{b:02X}" for b in binary.gpu_code[:32]))
    
    print("\nSymbols:")
    for name, addr in binary.symbols.items():
        print(f"  {name}: 0x{addr:08X}")
    
    # Guardar
    binary.save("matmul_1024.ahyb")
    
    print("\n" + "=" * 70)
    print("   Demo completada")
    print("=" * 70)


if __name__ == "__main__":
    demo()
