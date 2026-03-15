"""
ADead-BIB GPU Benchmark - RTX 3060 12GB
=======================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with love in Peru
"""

import torch
import numpy as np
import time

def benchmark():
    print("=" * 70)
    print("   BENCHMARK REAL: CPU vs GPU (RTX 3060 12GB)")
    print("=" * 70)
    
    if not torch.cuda.is_available():
        print("ERROR: CUDA no disponible")
        return
    
    print(f"\nGPU: {torch.cuda.get_device_name(0)}")
    print(f"VRAM: {torch.cuda.get_device_properties(0).total_memory / 1024**3:.1f} GB")
    
    # Warmup GPU
    print("\nCalentando GPU...")
    a = torch.randn(1000, 1000, device='cuda')
    b = torch.randn(1000, 1000, device='cuda')
    for _ in range(20):
        _ = torch.matmul(a, b)
    torch.cuda.synchronize()
    
    sizes = [512, 1024, 2048, 4096]
    
    print("\n" + "-" * 60)
    print(f"{'Tamano':<12} {'CPU (ms)':<12} {'GPU (ms)':<12} {'Speedup':<10}")
    print("-" * 60)
    
    results = []
    
    for size in sizes:
        # CPU benchmark
        a_cpu = np.random.randn(size, size).astype(np.float32)
        b_cpu = np.random.randn(size, size).astype(np.float32)
        
        start = time.perf_counter()
        for _ in range(3):
            _ = np.matmul(a_cpu, b_cpu)
        cpu_ms = ((time.perf_counter() - start) / 3) * 1000
        
        # GPU benchmark
        a_gpu = torch.from_numpy(a_cpu).cuda()
        b_gpu = torch.from_numpy(b_cpu).cuda()
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        for _ in range(10):
            _ = torch.matmul(a_gpu, b_gpu)
        torch.cuda.synchronize()
        gpu_ms = ((time.perf_counter() - start) / 10) * 1000
        
        speedup = cpu_ms / gpu_ms
        print(f"{size}x{size:<6} {cpu_ms:<12.2f} {gpu_ms:<12.2f} {speedup:.1f}x")
        
        results.append({
            "size": size,
            "cpu_ms": cpu_ms,
            "gpu_ms": gpu_ms,
            "speedup": speedup
        })
    
    print("-" * 60)
    
    # Resumen
    avg_speedup = np.mean([r["speedup"] for r in results])
    max_speedup = max([r["speedup"] for r in results])
    
    print(f"\nSpeedup promedio: {avg_speedup:.1f}x")
    print(f"Speedup maximo: {max_speedup:.1f}x")
    
    # Benchmark de atencion (transformer)
    print("\n" + "=" * 60)
    print("   BENCHMARK: Atencion Multi-Head (Transformer)")
    print("=" * 60)
    
    seq_len = 512
    embed_dim = 256
    num_heads = 8
    batch_size = 32
    
    print(f"\nConfig: seq={seq_len}, dim={embed_dim}, heads={num_heads}, batch={batch_size}")
    
    # CPU
    q_cpu = np.random.randn(batch_size, seq_len, embed_dim).astype(np.float32)
    k_cpu = np.random.randn(batch_size, seq_len, embed_dim).astype(np.float32)
    v_cpu = np.random.randn(batch_size, seq_len, embed_dim).astype(np.float32)
    
    start = time.perf_counter()
    for _ in range(3):
        scores = np.matmul(q_cpu, k_cpu.transpose(0, 2, 1)) / np.sqrt(embed_dim)
        weights = np.exp(scores) / np.sum(np.exp(scores), axis=-1, keepdims=True)
        _ = np.matmul(weights, v_cpu)
    cpu_attn_ms = ((time.perf_counter() - start) / 3) * 1000
    
    # GPU
    q_gpu = torch.from_numpy(q_cpu).cuda()
    k_gpu = torch.from_numpy(k_cpu).cuda()
    v_gpu = torch.from_numpy(v_cpu).cuda()
    torch.cuda.synchronize()
    
    start = time.perf_counter()
    for _ in range(10):
        scores = torch.matmul(q_gpu, k_gpu.transpose(1, 2)) / (embed_dim ** 0.5)
        weights = torch.softmax(scores, dim=-1)
        _ = torch.matmul(weights, v_gpu)
    torch.cuda.synchronize()
    gpu_attn_ms = ((time.perf_counter() - start) / 10) * 1000
    
    attn_speedup = cpu_attn_ms / gpu_attn_ms
    
    print(f"\nCPU: {cpu_attn_ms:.2f} ms")
    print(f"GPU: {gpu_attn_ms:.2f} ms")
    print(f"Speedup: {attn_speedup:.1f}x")
    
    print("\n" + "=" * 60)
    print("   Benchmark completado")
    print("=" * 60)
    
    return results


if __name__ == "__main__":
    benchmark()
