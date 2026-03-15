"""
CPU Compute para Metal-Dead
============================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪

Implementación CPU-first optimizada con integración ADead-BIB FFI.
Prioriza CPU con SIMD y paralelismo antes de GPU.
"""

import os
import sys
import time
import math
from pathlib import Path
from typing import List, Dict, Optional, Tuple, Any
from dataclasses import dataclass
from enum import Enum
from concurrent.futures import ThreadPoolExecutor
import threading

import numpy as np

# Intentar importar ADead-BIB FFI
ADEAD_FFI_AVAILABLE = False
try:
    sys.path.insert(0, str(Path(__file__).parent.parent.parent / "FFI" / "python"))
    from adead_py import ADeadFFI
    ADEAD_FFI_AVAILABLE = True
except ImportError:
    pass


class ComputeBackend(Enum):
    """Backends de cómputo disponibles."""
    CPU_NUMPY = "cpu_numpy"
    CPU_ADEAD = "cpu_adead"      # ADead-BIB FFI
    CPU_SIMD = "cpu_simd"        # SIMD optimizado
    CPU_PARALLEL = "cpu_parallel" # Multi-threaded
    AUTO = "auto"


@dataclass
class CPUInfo:
    """Información del CPU."""
    cores: int
    threads: int
    name: str
    has_avx: bool
    has_avx2: bool
    has_avx512: bool
    cache_l1: int
    cache_l2: int
    cache_l3: int


def get_cpu_info() -> CPUInfo:
    """Obtiene información del CPU."""
    cores = os.cpu_count() or 4
    threads = cores * 2  # Asumiendo hyperthreading
    
    # Detectar características (simplificado)
    has_avx = True   # Asumir AVX disponible en CPUs modernos
    has_avx2 = True
    has_avx512 = False  # Conservador
    
    return CPUInfo(
        cores=cores,
        threads=threads,
        name="CPU",
        has_avx=has_avx,
        has_avx2=has_avx2,
        has_avx512=has_avx512,
        cache_l1=32 * 1024,      # 32KB típico
        cache_l2=256 * 1024,     # 256KB típico
        cache_l3=8 * 1024 * 1024 # 8MB típico
    )


class CPUCompute:
    """
    Motor de cómputo CPU-first para Metal-Dead.
    Prioriza CPU con optimizaciones SIMD y paralelismo.
    """
    
    def __init__(self, backend: ComputeBackend = ComputeBackend.AUTO, num_threads: int = None):
        self.cpu_info = get_cpu_info()
        self.num_threads = num_threads or self.cpu_info.cores
        self.executor = ThreadPoolExecutor(max_workers=self.num_threads)
        
        # Seleccionar backend
        if backend == ComputeBackend.AUTO:
            if ADEAD_FFI_AVAILABLE:
                self.backend = ComputeBackend.CPU_ADEAD
                self.adead_ffi = ADeadFFI()
            else:
                self.backend = ComputeBackend.CPU_PARALLEL
                self.adead_ffi = None
        else:
            self.backend = backend
            self.adead_ffi = ADeadFFI() if backend == ComputeBackend.CPU_ADEAD and ADEAD_FFI_AVAILABLE else None
        
        # Cache para optimización
        self._cache = {}
        self._cache_lock = threading.Lock()
        
        self._print_info()
    
    def _print_info(self):
        """Imprime información del sistema."""
        print("\n" + "=" * 60)
        print("   🖥️  CPU Compute para Metal-Dead")
        print("=" * 60)
        print(f"\n✅ CPU: {self.cpu_info.cores} cores, {self.cpu_info.threads} threads")
        print(f"   Backend: {self.backend.value}")
        print(f"   ADead-BIB FFI: {'✅ Disponible' if ADEAD_FFI_AVAILABLE else '❌ No disponible'}")
        print(f"   SIMD: AVX{'2' if self.cpu_info.has_avx2 else ''} {'+ AVX-512' if self.cpu_info.has_avx512 else ''}")
        print("=" * 60)
    
    # =========================================================================
    # OPERACIONES BÁSICAS
    # =========================================================================
    
    def matmul(self, a: np.ndarray, b: np.ndarray) -> np.ndarray:
        """
        Multiplicación de matrices optimizada para CPU.
        Usa tiling para mejor uso de cache.
        """
        a = a.astype(np.float32)
        b = b.astype(np.float32)
        
        # Para matrices pequeñas, usar numpy directamente
        if a.shape[0] * a.shape[1] < 10000:
            return np.matmul(a, b)
        
        # Para matrices grandes, usar tiling
        return self._matmul_tiled(a, b)
    
    def _matmul_tiled(self, a: np.ndarray, b: np.ndarray, tile_size: int = 64) -> np.ndarray:
        """Multiplicación de matrices con tiling para mejor cache."""
        m, k = a.shape
        k2, n = b.shape
        assert k == k2, "Dimensiones incompatibles"
        
        c = np.zeros((m, n), dtype=np.float32)
        
        # Tiling para mejor uso de L1/L2 cache
        for i0 in range(0, m, tile_size):
            for j0 in range(0, n, tile_size):
                for k0 in range(0, k, tile_size):
                    i1 = min(i0 + tile_size, m)
                    j1 = min(j0 + tile_size, n)
                    k1 = min(k0 + tile_size, k)
                    
                    c[i0:i1, j0:j1] += np.matmul(
                        a[i0:i1, k0:k1],
                        b[k0:k1, j0:j1]
                    )
        
        return c
    
    def matmul_parallel(self, a: np.ndarray, b: np.ndarray) -> np.ndarray:
        """Multiplicación de matrices paralela por filas."""
        a = a.astype(np.float32)
        b = b.astype(np.float32)
        m, k = a.shape
        
        # Dividir trabajo por filas
        chunk_size = max(1, m // self.num_threads)
        
        def compute_chunk(start: int) -> Tuple[int, np.ndarray]:
            end = min(start + chunk_size, m)
            return start, np.matmul(a[start:end], b)
        
        # Ejecutar en paralelo
        futures = []
        for i in range(0, m, chunk_size):
            futures.append(self.executor.submit(compute_chunk, i))
        
        # Recolectar resultados
        c = np.zeros((m, b.shape[1]), dtype=np.float32)
        for future in futures:
            start, result = future.result()
            end = min(start + chunk_size, m)
            c[start:end] = result
        
        return c
    
    def softmax(self, x: np.ndarray, axis: int = -1) -> np.ndarray:
        """Softmax optimizado para CPU."""
        x = x.astype(np.float32)
        x_max = np.max(x, axis=axis, keepdims=True)
        exp_x = np.exp(x - x_max)
        return exp_x / (np.sum(exp_x, axis=axis, keepdims=True) + 1e-8)
    
    def gelu(self, x: np.ndarray) -> np.ndarray:
        """GELU activation optimizado."""
        x = x.astype(np.float32)
        # Aproximación rápida de GELU
        return x * 0.5 * (1.0 + np.tanh(0.7978845608 * (x + 0.044715 * x * x * x)))
    
    def layer_norm(self, x: np.ndarray, eps: float = 1e-5) -> np.ndarray:
        """Layer normalization."""
        x = x.astype(np.float32)
        mean = np.mean(x, axis=-1, keepdims=True)
        var = np.var(x, axis=-1, keepdims=True)
        return (x - mean) / np.sqrt(var + eps)
    
    def relu(self, x: np.ndarray) -> np.ndarray:
        """ReLU activation."""
        return np.maximum(0, x.astype(np.float32))
    
    # =========================================================================
    # OPERACIONES VECTORIALES
    # =========================================================================
    
    def dot(self, a: np.ndarray, b: np.ndarray) -> float:
        """Producto punto optimizado."""
        return float(np.dot(a.flatten().astype(np.float32), b.flatten().astype(np.float32)))
    
    def cosine_similarity(self, a: np.ndarray, b: np.ndarray) -> float:
        """Similitud coseno."""
        a = a.flatten().astype(np.float32)
        b = b.flatten().astype(np.float32)
        norm_a = np.linalg.norm(a)
        norm_b = np.linalg.norm(b)
        if norm_a == 0 or norm_b == 0:
            return 0.0
        return float(np.dot(a, b) / (norm_a * norm_b))
    
    def normalize(self, x: np.ndarray) -> np.ndarray:
        """Normaliza vector a norma unitaria."""
        x = x.astype(np.float32)
        norm = np.linalg.norm(x)
        if norm == 0:
            return x
        return x / norm
    
    # =========================================================================
    # ATTENTION
    # =========================================================================
    
    def attention(self, q: np.ndarray, k: np.ndarray, v: np.ndarray, 
                  mask: np.ndarray = None, scale: float = None) -> np.ndarray:
        """
        Scaled dot-product attention optimizado para CPU.
        
        Args:
            q: Query [seq_len, dim]
            k: Key [seq_len, dim]
            v: Value [seq_len, dim]
            mask: Máscara causal opcional
            scale: Factor de escala (default: 1/sqrt(dim))
        """
        q = q.astype(np.float32)
        k = k.astype(np.float32)
        v = v.astype(np.float32)
        
        dim = q.shape[-1]
        scale = scale or (1.0 / math.sqrt(dim))
        
        # Scores = Q @ K^T * scale
        scores = self.matmul(q, k.T) * scale
        
        # Aplicar máscara causal si se proporciona
        if mask is not None:
            scores = scores + mask
        
        # Softmax
        weights = self.softmax(scores, axis=-1)
        
        # Output = weights @ V
        return self.matmul(weights, v)
    
    def causal_mask(self, seq_len: int) -> np.ndarray:
        """Genera máscara causal para attention."""
        mask = np.triu(np.ones((seq_len, seq_len), dtype=np.float32) * -1e9, k=1)
        return mask
    
    # =========================================================================
    # TRANSFORMER LAYER
    # =========================================================================
    
    def transformer_layer(self, x: np.ndarray, 
                         w_q: np.ndarray, w_k: np.ndarray, w_v: np.ndarray, w_o: np.ndarray,
                         w1: np.ndarray, w2: np.ndarray,
                         num_heads: int = 8) -> np.ndarray:
        """
        Capa de transformer completa optimizada para CPU.
        
        Args:
            x: Input [seq_len, dim]
            w_q, w_k, w_v, w_o: Pesos de attention
            w1, w2: Pesos de FFN
            num_heads: Número de cabezas de attention
        """
        seq_len, dim = x.shape
        head_dim = dim // num_heads
        
        # Multi-head attention
        q = self.matmul(x, w_q)
        k = self.matmul(x, w_k)
        v = self.matmul(x, w_v)
        
        # Reshape para multi-head
        q = q.reshape(seq_len, num_heads, head_dim)
        k = k.reshape(seq_len, num_heads, head_dim)
        v = v.reshape(seq_len, num_heads, head_dim)
        
        # Attention por cabeza
        mask = self.causal_mask(seq_len)
        attn_outputs = []
        
        for h in range(num_heads):
            attn_out = self.attention(q[:, h, :], k[:, h, :], v[:, h, :], mask)
            attn_outputs.append(attn_out)
        
        # Concatenar cabezas
        attn_concat = np.concatenate(attn_outputs, axis=-1)
        
        # Output projection + residual
        attn_out = self.matmul(attn_concat, w_o)
        x = x + attn_out
        
        # FFN: GELU(x @ W1) @ W2 + residual
        hidden = self.gelu(self.matmul(x, w1))
        ffn_out = self.matmul(hidden, w2)
        x = x + ffn_out
        
        return x
    
    # =========================================================================
    # UTILIDADES
    # =========================================================================
    
    def benchmark(self, size: int = 512, iterations: int = 10) -> Dict[str, float]:
        """Benchmark de operaciones CPU."""
        results = {}
        
        # MatMul
        a = np.random.randn(size, size).astype(np.float32)
        b = np.random.randn(size, size).astype(np.float32)
        
        start = time.perf_counter()
        for _ in range(iterations):
            _ = self.matmul(a, b)
        elapsed = (time.perf_counter() - start) / iterations
        results["matmul_ms"] = elapsed * 1000
        
        # Softmax
        x = np.random.randn(size, size).astype(np.float32)
        start = time.perf_counter()
        for _ in range(iterations):
            _ = self.softmax(x)
        elapsed = (time.perf_counter() - start) / iterations
        results["softmax_ms"] = elapsed * 1000
        
        # GELU
        start = time.perf_counter()
        for _ in range(iterations):
            _ = self.gelu(x)
        elapsed = (time.perf_counter() - start) / iterations
        results["gelu_ms"] = elapsed * 1000
        
        # Attention
        q = np.random.randn(64, 64).astype(np.float32)
        k = np.random.randn(64, 64).astype(np.float32)
        v = np.random.randn(64, 64).astype(np.float32)
        
        start = time.perf_counter()
        for _ in range(iterations):
            _ = self.attention(q, k, v)
        elapsed = (time.perf_counter() - start) / iterations
        results["attention_ms"] = elapsed * 1000
        
        return results
    
    def get_metrics(self) -> Dict[str, Any]:
        """Obtiene métricas del sistema."""
        return {
            "backend": self.backend.value,
            "cores": self.cpu_info.cores,
            "threads": self.num_threads,
            "adead_ffi": ADEAD_FFI_AVAILABLE,
            "simd": f"AVX{'2' if self.cpu_info.has_avx2 else ''}",
        }
    
    def shutdown(self):
        """Cierra el executor."""
        self.executor.shutdown(wait=False)


class CPUTransformer:
    """
    Transformer completo optimizado para CPU.
    Usa CPUCompute para todas las operaciones.
    """
    
    def __init__(self, vocab_size: int, embed_dim: int, num_heads: int,
                 hidden_dim: int, num_layers: int, compute: CPUCompute = None):
        self.vocab_size = vocab_size
        self.embed_dim = embed_dim
        self.num_heads = num_heads
        self.hidden_dim = hidden_dim
        self.num_layers = num_layers
        self.compute = compute or CPUCompute()
        
        # Inicializar pesos
        self.embeddings = np.random.randn(vocab_size, embed_dim).astype(np.float32) * 0.02
        
        self.layers = []
        for _ in range(num_layers):
            self.layers.append({
                "W_q": np.random.randn(embed_dim, embed_dim).astype(np.float32) * 0.02,
                "W_k": np.random.randn(embed_dim, embed_dim).astype(np.float32) * 0.02,
                "W_v": np.random.randn(embed_dim, embed_dim).astype(np.float32) * 0.02,
                "W_o": np.random.randn(embed_dim, embed_dim).astype(np.float32) * 0.02,
                "W1": np.random.randn(embed_dim, hidden_dim).astype(np.float32) * 0.02,
                "W2": np.random.randn(hidden_dim, embed_dim).astype(np.float32) * 0.02,
            })
        
        self.output_proj = np.random.randn(embed_dim, vocab_size).astype(np.float32) * 0.02
        
        # Calcular memoria usada
        self.memory_mb = (
            vocab_size * embed_dim +
            num_layers * (4 * embed_dim**2 + 2 * embed_dim * hidden_dim) +
            embed_dim * vocab_size
        ) * 4 / (1024**2)
        
        print(f"📦 Modelo: {self.memory_mb:.1f} MB en RAM")
    
    def forward(self, token_ids: List[int]) -> np.ndarray:
        """Forward pass del transformer."""
        # Clamp token IDs
        safe_ids = [min(max(0, t), self.vocab_size - 1) for t in token_ids]
        
        # Embeddings
        x = self.embeddings[safe_ids]
        
        # Transformer layers
        for layer in self.layers:
            x = self.compute.transformer_layer(
                x,
                layer["W_q"], layer["W_k"], layer["W_v"], layer["W_o"],
                layer["W1"], layer["W2"],
                self.num_heads
            )
        
        # Output projection (solo último token)
        logits = self.compute.matmul(x[-1:], self.output_proj)
        
        return logits[0]
    
    def generate(self, token_ids: List[int], max_tokens: int = 20, 
                 temperature: float = 0.8) -> List[int]:
        """Genera tokens autoregressivamente."""
        generated = list(token_ids)
        
        for _ in range(max_tokens):
            logits = self.forward(generated)
            
            # Aplicar temperatura
            logits = logits / temperature
            
            # Softmax para probabilidades
            probs = self.compute.softmax(logits)
            
            # Sampling
            next_token = np.random.choice(len(probs), p=probs)
            generated.append(int(next_token))
            
            # Stop token (asumiendo 0 es EOS)
            if next_token == 0:
                break
        
        return generated


# =============================================================================
# DEMO
# =============================================================================

def demo():
    """Demo de CPU Compute."""
    print("\n" + "=" * 60)
    print("   🖥️  Demo CPU Compute para Metal-Dead")
    print("=" * 60)
    
    compute = CPUCompute()
    
    # Benchmark
    print("\n📊 Benchmark:")
    results = compute.benchmark(size=256, iterations=5)
    for name, time_ms in results.items():
        print(f"   {name}: {time_ms:.2f} ms")
    
    # Test transformer
    print("\n🤖 Test Transformer CPU:")
    transformer = CPUTransformer(
        vocab_size=1000,
        embed_dim=128,
        num_heads=4,
        hidden_dim=256,
        num_layers=2,
        compute=compute
    )
    
    # Forward pass
    start = time.perf_counter()
    logits = transformer.forward([1, 2, 3, 4, 5])
    elapsed = (time.perf_counter() - start) * 1000
    
    print(f"   Forward pass: {elapsed:.2f} ms")
    print(f"   Output shape: {logits.shape}")
    
    # Métricas
    print("\n📈 Métricas:")
    metrics = compute.get_metrics()
    for key, value in metrics.items():
        print(f"   {key}: {value}")
    
    compute.shutdown()
    print("\n✅ Demo completado")


if __name__ == "__main__":
    demo()
