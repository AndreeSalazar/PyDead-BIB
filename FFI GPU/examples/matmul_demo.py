"""
MatMul Demo - FFI GPU
======================
Demostración de multiplicación de matrices con FFI GPU.
"""

import sys
from pathlib import Path

# Agregar path
sys.path.insert(0, str(Path(__file__).parent.parent))

import numpy as np
from python import GPU, GPUOptimizer

def main():
    print("=" * 60)
    print("   🎮 MatMul Demo - ADead-BIB FFI GPU")
    print("=" * 60)
    
    # Inicializar GPU
    gpu = GPU()
    optimizer = GPUOptimizer()
    
    # Crear matrices de prueba
    N = 128
    print(f"\n📊 Matrices {N}x{N}")
    
    A = np.random.randn(N, N).astype(np.float32)
    B = np.random.randn(N, N).astype(np.float32)
    
    # Analizar optimización
    analysis = optimizer.analyze(A)
    print(f"\n🔧 Análisis de optimización:")
    print(f"   Layout: {analysis.layout.value}")
    print(f"   Tile size: {analysis.tile_size}")
    print(f"   Speedup estimado: {analysis.speedup_estimate:.1f}x")
    
    # Ejecutar matmul
    print("\n⚡ Ejecutando MatMul...")
    import time
    
    # GPU
    start = time.perf_counter()
    C_gpu = gpu.matmul(A, B)
    gpu_time = (time.perf_counter() - start) * 1000
    
    # NumPy (referencia)
    start = time.perf_counter()
    C_np = np.matmul(A, B)
    np_time = (time.perf_counter() - start) * 1000
    
    # Verificar resultado
    error = np.max(np.abs(C_gpu - C_np))
    print(f"\n📈 Resultados:")
    print(f"   GPU time: {gpu_time:.2f} ms")
    print(f"   NumPy time: {np_time:.2f} ms")
    print(f"   Max error: {error:.6f}")
    print(f"   Correcto: {'✅' if error < 0.01 else '❌'}")
    
    # Métricas
    metrics = gpu.get_metrics()
    print(f"\n📊 Métricas GPU:")
    print(f"   Dispatches: {metrics['dispatches']}")
    print(f"   Total time: {metrics['total_time_ms']:.2f} ms")
    
    gpu.shutdown()
    print("\n✅ Demo completado")

if __name__ == "__main__":
    main()
