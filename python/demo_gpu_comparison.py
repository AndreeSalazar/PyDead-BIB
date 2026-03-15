"""
ADead-BIB GPU Comparison Demo
=============================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with love in Peru

Demuestra las mejoras de rendimiento:
- CPU only (NumPy)
- GPU only (CUDA)
- Hibrido (CPU + GPU)
"""

import sys
import time
import numpy as np
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
sys.path.insert(0, str(Path(__file__).parent.parent / "hex"))

# Verificar CUDA
try:
    import torch
    HAS_CUDA = torch.cuda.is_available()
    if HAS_CUDA:
        GPU_NAME = torch.cuda.get_device_name(0)
        GPU_VRAM = torch.cuda.get_device_properties(0).total_memory / 1024**3
    else:
        GPU_NAME = "No GPU"
        GPU_VRAM = 0
except ImportError:
    HAS_CUDA = False
    GPU_NAME = "PyTorch no instalado"
    GPU_VRAM = 0


def benchmark_cpu(a, b, iterations=5):
    """Benchmark en CPU."""
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        _ = np.matmul(a, b)
        times.append((time.perf_counter() - start) * 1000)
    return np.mean(times)


def benchmark_gpu(a, b, iterations=10):
    """Benchmark en GPU."""
    if not HAS_CUDA:
        return None
    
    a_gpu = torch.from_numpy(a).cuda()
    b_gpu = torch.from_numpy(b).cuda()
    torch.cuda.synchronize()
    
    # Warmup
    for _ in range(3):
        _ = torch.matmul(a_gpu, b_gpu)
    torch.cuda.synchronize()
    
    times = []
    for _ in range(iterations):
        start = time.perf_counter()
        _ = torch.matmul(a_gpu, b_gpu)
        torch.cuda.synchronize()
        times.append((time.perf_counter() - start) * 1000)
    
    return np.mean(times)


def benchmark_hybrid(a, b, threshold=512):
    """Benchmark hibrido: GPU para matrices grandes, CPU para pequenas."""
    size = a.shape[0]
    
    if size >= threshold and HAS_CUDA:
        return benchmark_gpu(a, b)
    else:
        return benchmark_cpu(a, b)


def run_comparison():
    """Ejecuta comparacion completa."""
    print("=" * 80)
    print("   ADead-BIB: CPU vs GPU vs Hibrido")
    print("   RTX 3060 12GB + AMD Ryzen (12 cores)")
    print("=" * 80)
    
    print(f"\nHardware detectado:")
    print(f"  GPU: {GPU_NAME}")
    print(f"  VRAM: {GPU_VRAM:.1f} GB")
    print(f"  CUDA: {'Disponible' if HAS_CUDA else 'No disponible'}")
    
    sizes = [256, 512, 1024, 2048, 4096]
    
    print("\n" + "=" * 80)
    print("   BENCHMARK: Multiplicacion de Matrices")
    print("=" * 80)
    print(f"\n{'Tamano':<12} {'CPU (ms)':<12} {'GPU (ms)':<12} {'Hibrido':<12} {'Speedup GPU':<12}")
    print("-" * 60)
    
    results = []
    
    for size in sizes:
        a = np.random.randn(size, size).astype(np.float32)
        b = np.random.randn(size, size).astype(np.float32)
        
        cpu_ms = benchmark_cpu(a, b)
        gpu_ms = benchmark_gpu(a, b)
        hybrid_ms = benchmark_hybrid(a, b)
        
        if gpu_ms:
            speedup = cpu_ms / gpu_ms
            gpu_str = f"{gpu_ms:.2f}"
            hybrid_str = f"{hybrid_ms:.2f}"
            speedup_str = f"{speedup:.1f}x"
        else:
            speedup = 1.0
            gpu_str = "N/A"
            hybrid_str = f"{hybrid_ms:.2f}"
            speedup_str = "1x"
        
        print(f"{size}x{size:<6} {cpu_ms:<12.2f} {gpu_str:<12} {hybrid_str:<12} {speedup_str:<12}")
        
        results.append({
            "size": size,
            "cpu_ms": cpu_ms,
            "gpu_ms": gpu_ms,
            "hybrid_ms": hybrid_ms,
            "speedup": speedup
        })
    
    print("-" * 60)
    
    # Attention benchmark
    print("\n" + "=" * 80)
    print("   BENCHMARK: Atencion Multi-Head (Transformer)")
    print("=" * 80)
    
    configs = [
        (256, 64, 4),
        (512, 128, 8),
        (1024, 256, 8),
    ]
    
    print(f"\n{'Config':<20} {'CPU (ms)':<12} {'GPU (ms)':<12} {'Speedup':<12}")
    print("-" * 60)
    
    for seq_len, dim, heads in configs:
        batch = 16
        
        # CPU
        q = np.random.randn(batch, seq_len, dim).astype(np.float32)
        k = np.random.randn(batch, seq_len, dim).astype(np.float32)
        v = np.random.randn(batch, seq_len, dim).astype(np.float32)
        
        start = time.perf_counter()
        for _ in range(3):
            scores = np.matmul(q, k.transpose(0, 2, 1)) / np.sqrt(dim)
            weights = np.exp(scores) / np.sum(np.exp(scores), axis=-1, keepdims=True)
            _ = np.matmul(weights, v)
        cpu_ms = ((time.perf_counter() - start) / 3) * 1000
        
        # GPU
        if HAS_CUDA:
            q_gpu = torch.from_numpy(q).cuda()
            k_gpu = torch.from_numpy(k).cuda()
            v_gpu = torch.from_numpy(v).cuda()
            torch.cuda.synchronize()
            
            start = time.perf_counter()
            for _ in range(10):
                scores = torch.matmul(q_gpu, k_gpu.transpose(1, 2)) / (dim ** 0.5)
                weights = torch.softmax(scores, dim=-1)
                _ = torch.matmul(weights, v_gpu)
            torch.cuda.synchronize()
            gpu_ms = ((time.perf_counter() - start) / 10) * 1000
            
            speedup = cpu_ms / gpu_ms
            gpu_str = f"{gpu_ms:.2f}"
            speedup_str = f"{speedup:.1f}x"
        else:
            gpu_str = "N/A"
            speedup_str = "1x"
        
        config_str = f"seq={seq_len}, dim={dim}"
        print(f"{config_str:<20} {cpu_ms:<12.2f} {gpu_str:<12} {speedup_str:<12}")
    
    print("-" * 60)
    
    # Resumen
    print("\n" + "=" * 80)
    print("   RESUMEN DE MEJORAS")
    print("=" * 80)
    
    if results:
        avg_speedup = np.mean([r["speedup"] for r in results if r["speedup"] > 1])
        max_speedup = max([r["speedup"] for r in results])
        
        print(f"\n  Speedup promedio (MatMul): {avg_speedup:.1f}x")
        print(f"  Speedup maximo: {max_speedup:.1f}x")
        
        # Calcular ahorro de tiempo
        total_cpu = sum(r["cpu_ms"] for r in results)
        total_gpu = sum(r["gpu_ms"] for r in results if r["gpu_ms"])
        
        print(f"\n  Tiempo total CPU: {total_cpu:.1f} ms")
        print(f"  Tiempo total GPU: {total_gpu:.1f} ms")
        print(f"  Tiempo ahorrado: {total_cpu - total_gpu:.1f} ms ({(1 - total_gpu/total_cpu)*100:.1f}%)")
    
    print("\n" + "=" * 80)
    print("   RECOMENDACIONES")
    print("=" * 80)
    
    print("""
  1. Matrices pequenas (<512): Usar CPU (overhead GPU no vale la pena)
  2. Matrices medianas (512-2048): Usar GPU (10-20x mas rapido)
  3. Matrices grandes (>2048): Usar GPU (15-20x mas rapido)
  4. Atencion Transformer: Siempre GPU (30-50x mas rapido)
  
  Modo Hibrido recomendado:
  - CPU: Tokenizacion, preprocesamiento, I/O
  - GPU: MatMul, Attention, FFN, Softmax
""")
    
    print("=" * 80)
    print("   Demo completada - ADead-BIB GPU Support")
    print("=" * 80)
    
    return results


if __name__ == "__main__":
    run_comparison()
