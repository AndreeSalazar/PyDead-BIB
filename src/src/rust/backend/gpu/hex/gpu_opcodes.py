"""
ADead-BIB GPU Opcodes - Generador de Opcodes GPU
=================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with love in Peru

Genera opcodes hexadecimales para operaciones GPU que ADead-BIB
puede ejecutar directamente en la RTX 3060.
"""

import struct
from typing import List, Tuple, Optional
from dataclasses import dataclass
from enum import IntEnum


class GPUOpcode(IntEnum):
    """Opcodes GPU para ADead-BIB."""
    # Inicializacion
    GPU_INIT = 0xC0DA0001
    GPU_SHUTDOWN = 0xC0DA0002
    
    # Memoria
    GPU_ALLOC = 0xC0DA0010
    GPU_FREE = 0xC0DA0011
    GPU_COPY_H2D = 0xC0DA0012  # Host to Device
    GPU_COPY_D2H = 0xC0DA0013  # Device to Host
    GPU_MEMSET = 0xC0DA0014
    
    # Operaciones matematicas
    GPU_MATMUL = 0xC0DA0020
    GPU_ADD = 0xC0DA0021
    GPU_SUB = 0xC0DA0022
    GPU_MUL = 0xC0DA0023
    GPU_DIV = 0xC0DA0024
    GPU_SCALE = 0xC0DA0025
    
    # Activaciones
    GPU_RELU = 0xC0DA0030
    GPU_SIGMOID = 0xC0DA0031
    GPU_TANH = 0xC0DA0032
    GPU_SOFTMAX = 0xC0DA0033
    GPU_GELU = 0xC0DA0034
    
    # Transformer
    GPU_ATTENTION = 0xC0DA0040
    GPU_MULTIHEAD_ATTN = 0xC0DA0041
    GPU_FFN = 0xC0DA0042
    GPU_LAYERNORM = 0xC0DA0043
    GPU_EMBEDDING = 0xC0DA0044
    
    # Sincronizacion
    GPU_SYNC = 0xC0DA00F0
    GPU_BARRIER = 0xC0DA00F1
    
    # Control
    GPU_NOP = 0xC0DA0000
    GPU_END = 0xC0DAFFFF


@dataclass
class GPUInstruction:
    """Instruccion GPU con opcode y operandos."""
    opcode: GPUOpcode
    operands: List[int]
    comment: str = ""
    
    def to_bytes(self) -> bytes:
        """Convierte la instruccion a bytes."""
        # Formato: [opcode:4][num_operands:1][operands:4*n]
        data = struct.pack('<I', self.opcode)
        data += struct.pack('<B', len(self.operands))
        for op in self.operands:
            data += struct.pack('<I', op)
        return data
    
    def to_hex(self) -> str:
        """Convierte a string hexadecimal."""
        return self.to_bytes().hex().upper()
    
    def __str__(self) -> str:
        ops = ", ".join(f"0x{op:08X}" for op in self.operands)
        return f"{self.opcode.name:<20} [{ops}]  ; {self.comment}"


class GPUProgram:
    """Programa GPU compuesto de instrucciones."""
    
    def __init__(self, name: str = "gpu_program"):
        self.name = name
        self.instructions: List[GPUInstruction] = []
        self.data_section: bytes = b""
        self.symbols: dict = {}
    
    def add(self, opcode: GPUOpcode, operands: List[int] = None, comment: str = ""):
        """Agrega una instruccion."""
        self.instructions.append(GPUInstruction(
            opcode=opcode,
            operands=operands or [],
            comment=comment
        ))
        return self
    
    def init(self):
        """Inicializa GPU."""
        return self.add(GPUOpcode.GPU_INIT, [], "Inicializar contexto CUDA")
    
    def alloc(self, size: int, reg: int = 0):
        """Reserva memoria GPU."""
        return self.add(GPUOpcode.GPU_ALLOC, [size, reg], f"Reservar {size} bytes en reg{reg}")
    
    def copy_to_gpu(self, src: int, dst: int, size: int):
        """Copia datos Host -> Device."""
        return self.add(GPUOpcode.GPU_COPY_H2D, [src, dst, size], "Copiar a GPU")
    
    def copy_from_gpu(self, src: int, dst: int, size: int):
        """Copia datos Device -> Host."""
        return self.add(GPUOpcode.GPU_COPY_D2H, [src, dst, size], "Copiar de GPU")
    
    def matmul(self, a: int, b: int, c: int, m: int, n: int, k: int):
        """Multiplicacion de matrices C = A @ B."""
        return self.add(GPUOpcode.GPU_MATMUL, [a, b, c, m, n, k], f"MatMul {m}x{k} @ {k}x{n}")
    
    def relu(self, src: int, dst: int, size: int):
        """Activacion ReLU."""
        return self.add(GPUOpcode.GPU_RELU, [src, dst, size], "ReLU")
    
    def softmax(self, src: int, dst: int, rows: int, cols: int):
        """Softmax por filas."""
        return self.add(GPUOpcode.GPU_SOFTMAX, [src, dst, rows, cols], "Softmax")
    
    def attention(self, q: int, k: int, v: int, out: int, seq_len: int, dim: int):
        """Atencion scaled dot-product."""
        return self.add(GPUOpcode.GPU_ATTENTION, [q, k, v, out, seq_len, dim], "Attention")
    
    def sync(self):
        """Sincroniza GPU."""
        return self.add(GPUOpcode.GPU_SYNC, [], "Sincronizar")
    
    def free(self, reg: int):
        """Libera memoria."""
        return self.add(GPUOpcode.GPU_FREE, [reg], f"Liberar reg{reg}")
    
    def end(self):
        """Termina programa."""
        return self.add(GPUOpcode.GPU_END, [], "Fin del programa")
    
    def to_bytes(self) -> bytes:
        """Genera binario completo."""
        # Header: magic + version + num_instructions
        header = struct.pack('<4sHH', b'AGPU', 1, len(self.instructions))
        
        # Instructions
        code = b"".join(inst.to_bytes() for inst in self.instructions)
        
        return header + code
    
    def to_hex_dump(self) -> str:
        """Genera dump hexadecimal legible."""
        lines = []
        lines.append(f"; ADead-BIB GPU Program: {self.name}")
        lines.append(f"; Instructions: {len(self.instructions)}")
        lines.append("; " + "=" * 60)
        lines.append("")
        
        offset = 0
        for inst in self.instructions:
            hex_str = inst.to_hex()
            lines.append(f"{offset:04X}: {hex_str:<40} ; {inst.comment}")
            offset += len(inst.to_bytes())
        
        lines.append("")
        lines.append("; " + "=" * 60)
        lines.append(f"; Total size: {offset} bytes")
        
        return "\n".join(lines)
    
    def save(self, filename: str):
        """Guarda programa a archivo."""
        with open(filename, 'wb') as f:
            f.write(self.to_bytes())
    
    def save_hex(self, filename: str):
        """Guarda dump hexadecimal."""
        with open(filename, 'w') as f:
            f.write(self.to_hex_dump())


def create_matmul_program(m: int, n: int, k: int) -> GPUProgram:
    """Crea programa de multiplicacion de matrices."""
    prog = GPUProgram("matmul")
    
    size_a = m * k * 4  # float32
    size_b = k * n * 4
    size_c = m * n * 4
    
    prog.init()
    prog.alloc(size_a, 0)  # reg0 = A
    prog.alloc(size_b, 1)  # reg1 = B
    prog.alloc(size_c, 2)  # reg2 = C
    prog.copy_to_gpu(0x1000, 0, size_a)  # Copiar A
    prog.copy_to_gpu(0x2000, 1, size_b)  # Copiar B
    prog.matmul(0, 1, 2, m, n, k)  # C = A @ B
    prog.sync()
    prog.copy_from_gpu(2, 0x3000, size_c)  # Copiar C
    prog.free(0)
    prog.free(1)
    prog.free(2)
    prog.end()
    
    return prog


def create_attention_program(seq_len: int, dim: int) -> GPUProgram:
    """Crea programa de atencion."""
    prog = GPUProgram("attention")
    
    size = seq_len * dim * 4
    
    prog.init()
    prog.alloc(size, 0)  # Q
    prog.alloc(size, 1)  # K
    prog.alloc(size, 2)  # V
    prog.alloc(size, 3)  # Output
    prog.copy_to_gpu(0x1000, 0, size)
    prog.copy_to_gpu(0x2000, 1, size)
    prog.copy_to_gpu(0x3000, 2, size)
    prog.attention(0, 1, 2, 3, seq_len, dim)
    prog.sync()
    prog.copy_from_gpu(3, 0x4000, size)
    prog.free(0)
    prog.free(1)
    prog.free(2)
    prog.free(3)
    prog.end()
    
    return prog


def create_transformer_layer_program(seq_len: int, dim: int, hidden: int) -> GPUProgram:
    """Crea programa de capa transformer completa."""
    prog = GPUProgram("transformer_layer")
    
    size_qkv = seq_len * dim * 4
    size_hidden = seq_len * hidden * 4
    
    prog.init()
    
    # Atencion
    prog.alloc(size_qkv, 0)  # Q
    prog.alloc(size_qkv, 1)  # K
    prog.alloc(size_qkv, 2)  # V
    prog.alloc(size_qkv, 3)  # Attn output
    prog.copy_to_gpu(0x1000, 0, size_qkv)
    prog.copy_to_gpu(0x2000, 1, size_qkv)
    prog.copy_to_gpu(0x3000, 2, size_qkv)
    prog.attention(0, 1, 2, 3, seq_len, dim)
    
    # FFN
    prog.alloc(size_hidden, 4)  # Hidden
    prog.alloc(size_qkv, 5)     # FFN output
    prog.add(GPUOpcode.GPU_FFN, [3, 4, 5, seq_len, dim, hidden], "FFN")
    prog.relu(4, 4, size_hidden)
    
    prog.sync()
    prog.copy_from_gpu(5, 0x4000, size_qkv)
    
    for i in range(6):
        prog.free(i)
    
    prog.end()
    
    return prog


def demo():
    """Demo de generacion de opcodes GPU."""
    print("=" * 70)
    print("   ADead-BIB GPU Opcodes Generator")
    print("   Author: Eddi Andree Salazar Matos")
    print("=" * 70)
    
    # MatMul 1024x1024
    print("\n1. Programa MatMul 1024x1024:")
    print("-" * 50)
    prog = create_matmul_program(1024, 1024, 1024)
    print(prog.to_hex_dump())
    
    # Guardar
    prog.save_hex("matmul_1024.hex")
    print(f"\nGuardado: matmul_1024.hex")
    
    # Attention
    print("\n2. Programa Attention 512x256:")
    print("-" * 50)
    prog = create_attention_program(512, 256)
    print(prog.to_hex_dump())
    
    # Transformer
    print("\n3. Programa Transformer Layer:")
    print("-" * 50)
    prog = create_transformer_layer_program(512, 256, 1024)
    print(prog.to_hex_dump())
    
    print("\n" + "=" * 70)
    print("   Opcodes generados correctamente")
    print("=" * 70)


if __name__ == "__main__":
    demo()
