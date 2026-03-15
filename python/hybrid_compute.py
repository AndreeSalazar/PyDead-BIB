"""
ADead-BIB Hybrid Compute - Sistema CPU + GPU
=============================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with love in Peru

Sistema de computo hibrido que detecta hardware y usa GPU si esta disponible.
"""

import os
import sys
import time
import numpy as np
from pathlib import Path
from enum import Enum

sys.path.insert(0, str(Path(__file__).parent))

from gpu_detect import detect_hardware


class ComputeMode(Enum):
    CPU = "cpu"
    GPU = "gpu"
    HYBRID = "hybrid"
    AUTO = "auto"


HAS_TORCH = False
TORCH_CUDA = False
HAS_CUPY = False

try:
    import torch
    HAS_TORCH = True
    TORCH_CUDA = torch.cuda.is_available()
except ImportError:
    pass

try:
    import cupy as cp
    HAS_CUPY = True
except ImportError:
    pass


class CPUBackend:
    def __init__(self):
        self.name = "CPU (NumPy)"
        self.device = "cpu"
    
    def to_device(self, x):
        return np.asarray(x, dtype=np.float32)
    
    def to_numpy(self, x):
        return x
    
    def matmul(self, a, b):
        return np.matmul(a, b)
    
    def sync(self):
        pass


class HybridCompute:
    def __init__(self, mode=ComputeMode.AUTO):
        self.hardware = detect_hardware()
        self.mode = mode
        self.cpu = CPUBackend()
        self.gpu = None
        
        if mode != ComputeMode.CPU:
            self._init_gpu()
        
        self.effective_mode = ComputeMode.GPU if self.gpu else ComputeMode.CPU
        self.stats = {"cpu_ops": 0, "gpu_ops": 0, "cpu_time_ms": 0, "gpu_time_ms": 0}
    
    def _init_gpu(self):
        if HAS_TORCH and TORCH_CUDA:
            try:
                self.gpu = type('TorchBackend', (), {
                    'name': "GPU (PyTorch CUDA)",
                    'device': torch.device("cuda"),
                    'to_device': lambda s, x: torch.from_numpy(x.astype(np.float32)).cuda(),
                    'to_numpy': lambda s, x: x.cpu().numpy(),
                    'matmul': lambda s, a, b: torch.matmul(a, b),
                    'sync': lambda s: torch.cuda.synchronize()
                })()
            except Exception:
                pass
    
    def get_backend(self, prefer_gpu=True):
        if prefer_gpu and self.gpu:
            return self.gpu
        return self.cpu
    
    def matmul(self, a, b, force_cpu=False):
        backend = self.cpu if force_cpu else self.get_backend()
        start = time.perf_counter()
        a_dev = backend.to_device(a)
        b_dev = backend.to_device(b)
        result = backend.matmul(a_dev, b_dev)
        backend.sync()
        result_np = backend.to_numpy(result)
        elapsed = (time.perf_counter() - start) * 1000
        
        if backend == self.gpu:
            self.stats["gpu_ops"] += 1
            self.stats["gpu_time_ms"] += elapsed
        else:
            self.stats["cpu_ops"] += 1
            self.stats["cpu_time_ms"] += elapsed
        return result_np
    
    def print_info(self):
        print("\n" + "=" * 60)
        print("   ADead-BIB Hybrid Compute")
        print("=" * 60)
        print(f"\nModo: {self.effective_mode.value.upper()}")
        print(f"CPU: {self.cpu.name}")
        print(f"GPU: {self.gpu.name if self.gpu else 'No disponible'}")
        print(f"\nHardware:")
        print(f"  CPU: {self.hardware.cpu.name}")
        print(f"  Cores: {self.hardware.cpu.cores_logical}")
        print(f"  RAM: {self.hardware.ram.total_gb:.1f} GB")


def benchmark():
    print("\n" + "=" * 70)
    print("   BENCHMARK: CPU vs GPU")
    print("=" * 70)
    
    hybrid = HybridCompute(ComputeMode.AUTO)
    hybrid.print_info()
    
    sizes = [256, 512, 1024]
    
    print("\nMultiplicacion de Matrices")
    print("-" * 60)
    print(f"{'Tamano':<12} {'CPU (ms)':<15} {'GPU (ms)':<15} {'Speedup':<10}")
    print("-" * 60)
    
    for size in sizes:
        a = np.random.randn(size, size).astype(np.float32)
        b = np.random.randn(size, size).astype(np.float32)
        
        start = time.perf_counter()
        for _ in range(3):
            _ = hybrid.matmul(a, b, force_cpu=True)
        cpu_ms = ((time.perf_counter() - start) / 3) * 1000
        
        if hybrid.gpu:
            start = time.perf_counter()
            for _ in range(3):
                _ = hybrid.matmul(a, b, force_cpu=False)
            gpu_ms = ((time.perf_counter() - start) / 3) * 1000
            speedup = cpu_ms / gpu_ms
        else:
            gpu_ms = None
            speedup = 1.0
        
        gpu_str = f"{gpu_ms:.2f}" if gpu_ms else "N/A"
        print(f"{size}x{size:<6} {cpu_ms:<15.2f} {gpu_str:<15} {speedup:.1f}x")
    
    print("-" * 60)


def demo():
    print("\n" + "=" * 70)
    print("   ADead-BIB Hybrid Compute Demo")
    print("   Author: Eddi Andree Salazar Matos")
    print("=" * 70)
    
    benchmark()
    
    print("\n" + "=" * 70)
    print("   Demo completada")
    print("=" * 70)


if __name__ == "__main__":
    demo()
