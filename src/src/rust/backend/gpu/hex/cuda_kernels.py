"""
ADead-BIB CUDA Kernels - Kernels Pre-compilados
================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with love in Peru

Kernels CUDA optimizados para RTX 3060 que ADead-BIB puede llamar directamente.
"""

import torch
import numpy as np
import time
from typing import Tuple, Optional
from dataclasses import dataclass


@dataclass
class KernelStats:
    """Estadisticas de ejecucion de kernel."""
    name: str
    time_ms: float
    gflops: float
    memory_mb: float


class CUDAKernels:
    """Kernels CUDA optimizados para ADead-BIB."""
    
    def __init__(self):
        if not torch.cuda.is_available():
            raise RuntimeError("CUDA no disponible")
        
        self.device = torch.device("cuda")
        self.stream = torch.cuda.Stream()
        
        # Info GPU
        props = torch.cuda.get_device_properties(0)
        self.gpu_name = props.name
        self.vram_gb = props.total_memory / 1024**3
        self.sm_count = props.multi_processor_count
        
        # Stats
        self.stats = []
        
        # Warmup
        self._warmup()
    
    def _warmup(self):
        """Calienta la GPU."""
        a = torch.randn(1000, 1000, device=self.device)
        for _ in range(10):
            _ = torch.matmul(a, a)
        torch.cuda.synchronize()
    
    def matmul(self, a: np.ndarray, b: np.ndarray) -> Tuple[np.ndarray, KernelStats]:
        """Multiplicacion de matrices en GPU."""
        m, k = a.shape
        k2, n = b.shape
        assert k == k2, "Dimensiones incompatibles"
        
        # Transferir a GPU
        a_gpu = torch.from_numpy(a.astype(np.float32)).to(self.device)
        b_gpu = torch.from_numpy(b.astype(np.float32)).to(self.device)
        torch.cuda.synchronize()
        
        # Ejecutar
        start = time.perf_counter()
        c_gpu = torch.matmul(a_gpu, b_gpu)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) * 1000
        
        # Calcular GFLOPS
        flops = 2 * m * n * k
        gflops = flops / (elapsed / 1000) / 1e9
        
        # Transferir resultado
        c = c_gpu.cpu().numpy()
        
        stats = KernelStats(
            name="matmul",
            time_ms=elapsed,
            gflops=gflops,
            memory_mb=(a.nbytes + b.nbytes + c.nbytes) / 1024**2
        )
        self.stats.append(stats)
        
        return c, stats
    
    def batch_matmul(self, a: np.ndarray, b: np.ndarray) -> Tuple[np.ndarray, KernelStats]:
        """Multiplicacion de matrices en batch."""
        batch, m, k = a.shape
        batch2, k2, n = b.shape
        assert batch == batch2 and k == k2
        
        a_gpu = torch.from_numpy(a.astype(np.float32)).to(self.device)
        b_gpu = torch.from_numpy(b.astype(np.float32)).to(self.device)
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        c_gpu = torch.bmm(a_gpu, b_gpu)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) * 1000
        
        flops = 2 * batch * m * n * k
        gflops = flops / (elapsed / 1000) / 1e9
        
        c = c_gpu.cpu().numpy()
        
        stats = KernelStats(
            name="batch_matmul",
            time_ms=elapsed,
            gflops=gflops,
            memory_mb=(a.nbytes + b.nbytes + c.nbytes) / 1024**2
        )
        self.stats.append(stats)
        
        return c, stats
    
    def attention(self, q: np.ndarray, k: np.ndarray, v: np.ndarray, 
                  scale: Optional[float] = None) -> Tuple[np.ndarray, KernelStats]:
        """Atencion scaled dot-product."""
        seq_len, dim = q.shape
        if scale is None:
            scale = 1.0 / np.sqrt(dim)
        
        q_gpu = torch.from_numpy(q.astype(np.float32)).to(self.device)
        k_gpu = torch.from_numpy(k.astype(np.float32)).to(self.device)
        v_gpu = torch.from_numpy(v.astype(np.float32)).to(self.device)
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        scores = torch.matmul(q_gpu, k_gpu.T) * scale
        weights = torch.softmax(scores, dim=-1)
        output = torch.matmul(weights, v_gpu)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) * 1000
        
        # FLOPS: 2 matmuls + softmax
        flops = 2 * seq_len * seq_len * dim + 2 * seq_len * seq_len * dim
        gflops = flops / (elapsed / 1000) / 1e9
        
        result = output.cpu().numpy()
        
        stats = KernelStats(
            name="attention",
            time_ms=elapsed,
            gflops=gflops,
            memory_mb=(q.nbytes + k.nbytes + v.nbytes + result.nbytes) / 1024**2
        )
        self.stats.append(stats)
        
        return result, stats
    
    def multihead_attention(self, q: np.ndarray, k: np.ndarray, v: np.ndarray,
                            num_heads: int) -> Tuple[np.ndarray, KernelStats]:
        """Atencion multi-head."""
        batch, seq_len, dim = q.shape
        head_dim = dim // num_heads
        
        q_gpu = torch.from_numpy(q.astype(np.float32)).to(self.device)
        k_gpu = torch.from_numpy(k.astype(np.float32)).to(self.device)
        v_gpu = torch.from_numpy(v.astype(np.float32)).to(self.device)
        
        # Reshape para multi-head
        q_gpu = q_gpu.view(batch, seq_len, num_heads, head_dim).transpose(1, 2)
        k_gpu = k_gpu.view(batch, seq_len, num_heads, head_dim).transpose(1, 2)
        v_gpu = v_gpu.view(batch, seq_len, num_heads, head_dim).transpose(1, 2)
        
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        scale = 1.0 / (head_dim ** 0.5)
        scores = torch.matmul(q_gpu, k_gpu.transpose(-2, -1)) * scale
        weights = torch.softmax(scores, dim=-1)
        output = torch.matmul(weights, v_gpu)
        output = output.transpose(1, 2).contiguous().view(batch, seq_len, dim)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) * 1000
        
        flops = 4 * batch * num_heads * seq_len * seq_len * head_dim
        gflops = flops / (elapsed / 1000) / 1e9
        
        result = output.cpu().numpy()
        
        stats = KernelStats(
            name="multihead_attention",
            time_ms=elapsed,
            gflops=gflops,
            memory_mb=(q.nbytes + k.nbytes + v.nbytes + result.nbytes) / 1024**2
        )
        self.stats.append(stats)
        
        return result, stats
    
    def relu(self, x: np.ndarray) -> Tuple[np.ndarray, KernelStats]:
        """Activacion ReLU."""
        x_gpu = torch.from_numpy(x.astype(np.float32)).to(self.device)
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        y_gpu = torch.relu(x_gpu)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) * 1000
        
        result = y_gpu.cpu().numpy()
        
        stats = KernelStats(
            name="relu",
            time_ms=elapsed,
            gflops=x.size / (elapsed / 1000) / 1e9,
            memory_mb=x.nbytes * 2 / 1024**2
        )
        self.stats.append(stats)
        
        return result, stats
    
    def softmax(self, x: np.ndarray, axis: int = -1) -> Tuple[np.ndarray, KernelStats]:
        """Softmax."""
        x_gpu = torch.from_numpy(x.astype(np.float32)).to(self.device)
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        y_gpu = torch.softmax(x_gpu, dim=axis)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) * 1000
        
        result = y_gpu.cpu().numpy()
        
        stats = KernelStats(
            name="softmax",
            time_ms=elapsed,
            gflops=x.size * 5 / (elapsed / 1000) / 1e9,
            memory_mb=x.nbytes * 2 / 1024**2
        )
        self.stats.append(stats)
        
        return result, stats
    
    def layer_norm(self, x: np.ndarray, eps: float = 1e-5) -> Tuple[np.ndarray, KernelStats]:
        """Layer normalization."""
        x_gpu = torch.from_numpy(x.astype(np.float32)).to(self.device)
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        y_gpu = torch.nn.functional.layer_norm(x_gpu, x_gpu.shape[-1:], eps=eps)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) * 1000
        
        result = y_gpu.cpu().numpy()
        
        stats = KernelStats(
            name="layer_norm",
            time_ms=elapsed,
            gflops=x.size * 5 / (elapsed / 1000) / 1e9,
            memory_mb=x.nbytes * 2 / 1024**2
        )
        self.stats.append(stats)
        
        return result, stats
    
    def ffn(self, x: np.ndarray, w1: np.ndarray, w2: np.ndarray) -> Tuple[np.ndarray, KernelStats]:
        """Feed-forward network: ReLU(x @ W1) @ W2."""
        x_gpu = torch.from_numpy(x.astype(np.float32)).to(self.device)
        w1_gpu = torch.from_numpy(w1.astype(np.float32)).to(self.device)
        w2_gpu = torch.from_numpy(w2.astype(np.float32)).to(self.device)
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        hidden = torch.relu(torch.matmul(x_gpu, w1_gpu))
        output = torch.matmul(hidden, w2_gpu)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) * 1000
        
        # FLOPS
        m, k = x.shape
        k, h = w1.shape
        h, n = w2.shape
        flops = 2 * m * k * h + 2 * m * h * n + m * h
        gflops = flops / (elapsed / 1000) / 1e9
        
        result = output.cpu().numpy()
        
        stats = KernelStats(
            name="ffn",
            time_ms=elapsed,
            gflops=gflops,
            memory_mb=(x.nbytes + w1.nbytes + w2.nbytes + result.nbytes) / 1024**2
        )
        self.stats.append(stats)
        
        return result, stats
    
    def get_stats_summary(self) -> dict:
        """Resumen de estadisticas."""
        if not self.stats:
            return {}
        
        total_time = sum(s.time_ms for s in self.stats)
        avg_gflops = np.mean([s.gflops for s in self.stats])
        
        return {
            "gpu": self.gpu_name,
            "vram_gb": self.vram_gb,
            "total_kernels": len(self.stats),
            "total_time_ms": total_time,
            "avg_gflops": avg_gflops,
        }
    
    def print_stats(self):
        """Imprime estadisticas."""
        print(f"\nGPU: {self.gpu_name} ({self.vram_gb:.1f} GB)")
        print(f"SMs: {self.sm_count}")
        print(f"\nKernels ejecutados: {len(self.stats)}")
        print("-" * 60)
        print(f"{'Kernel':<20} {'Tiempo (ms)':<15} {'GFLOPS':<15}")
        print("-" * 60)
        for s in self.stats:
            print(f"{s.name:<20} {s.time_ms:<15.3f} {s.gflops:<15.1f}")
        print("-" * 60)


def demo():
    """Demo de kernels CUDA."""
    print("=" * 70)
    print("   ADead-BIB CUDA Kernels Demo")
    print("   Author: Eddi Andree Salazar Matos")
    print("=" * 70)
    
    kernels = CUDAKernels()
    
    # MatMul
    print("\n1. MatMul 2048x2048:")
    a = np.random.randn(2048, 2048).astype(np.float32)
    b = np.random.randn(2048, 2048).astype(np.float32)
    c, stats = kernels.matmul(a, b)
    print(f"   Tiempo: {stats.time_ms:.2f} ms, GFLOPS: {stats.gflops:.1f}")
    
    # Attention
    print("\n2. Attention 512x256:")
    q = np.random.randn(512, 256).astype(np.float32)
    k = np.random.randn(512, 256).astype(np.float32)
    v = np.random.randn(512, 256).astype(np.float32)
    out, stats = kernels.attention(q, k, v)
    print(f"   Tiempo: {stats.time_ms:.2f} ms, GFLOPS: {stats.gflops:.1f}")
    
    # Multi-head Attention
    print("\n3. Multi-Head Attention (8 heads):")
    q = np.random.randn(32, 512, 256).astype(np.float32)
    k = np.random.randn(32, 512, 256).astype(np.float32)
    v = np.random.randn(32, 512, 256).astype(np.float32)
    out, stats = kernels.multihead_attention(q, k, v, num_heads=8)
    print(f"   Tiempo: {stats.time_ms:.2f} ms, GFLOPS: {stats.gflops:.1f}")
    
    # FFN
    print("\n4. FFN (256 -> 1024 -> 256):")
    x = np.random.randn(512, 256).astype(np.float32)
    w1 = np.random.randn(256, 1024).astype(np.float32)
    w2 = np.random.randn(1024, 256).astype(np.float32)
    out, stats = kernels.ffn(x, w1, w2)
    print(f"   Tiempo: {stats.time_ms:.2f} ms, GFLOPS: {stats.gflops:.1f}")
    
    # Resumen
    kernels.print_stats()
    
    print("\n" + "=" * 70)
    print("   Demo completada")
    print("=" * 70)


if __name__ == "__main__":
    demo()
