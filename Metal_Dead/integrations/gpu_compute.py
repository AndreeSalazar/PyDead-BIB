"""
GPU Compute para Metal-Dead
============================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪
"""

import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Any
from dataclasses import dataclass
from enum import Enum

import numpy as np

HAS_TORCH = False
TORCH_CUDA = False

try:
    import torch
    HAS_TORCH = True
    TORCH_CUDA = torch.cuda.is_available()
    if TORCH_CUDA:
        DEVICE = torch.device("cuda")
        GPU_NAME = torch.cuda.get_device_name(0)
        GPU_MEMORY = torch.cuda.get_device_properties(0).total_memory / (1024**3)
    else:
        DEVICE = torch.device("cpu")
        GPU_NAME = None
        GPU_MEMORY = 0
except ImportError:
    pass


class ComputeBackend(Enum):
    CPU = "cpu"
    CUDA_TORCH = "cuda_torch"
    AUTO = "auto"


class GPUCompute:
    def __init__(self, backend: ComputeBackend = ComputeBackend.AUTO):
        self.active_backend = ComputeBackend.CPU
        
        if backend == ComputeBackend.AUTO and HAS_TORCH and TORCH_CUDA:
            self.active_backend = ComputeBackend.CUDA_TORCH
            self._warmup()
        
        self._print_info()
    
    def _warmup(self):
        if self.active_backend == ComputeBackend.CUDA_TORCH:
            a = torch.randn(512, 512, device=DEVICE)
            for _ in range(5):
                _ = torch.matmul(a, a)
            torch.cuda.synchronize()
    
    def _print_info(self):
        print("\n" + "=" * 60)
        print("   🎮 GPU Compute para Metal-Dead")
        print("=" * 60)
        if HAS_TORCH and TORCH_CUDA:
            print(f"\n✅ GPU: {GPU_NAME}")
            print(f"   VRAM: {GPU_MEMORY:.1f} GB")
        else:
            print(f"\n⚠️ GPU no disponible, usando CPU")
        print("=" * 60)
    
    def is_gpu_available(self) -> bool:
        return self.active_backend == ComputeBackend.CUDA_TORCH
    
    def matmul(self, a: np.ndarray, b: np.ndarray) -> np.ndarray:
        if self.is_gpu_available():
            a_gpu = torch.from_numpy(a.astype(np.float32)).to(DEVICE)
            b_gpu = torch.from_numpy(b.astype(np.float32)).to(DEVICE)
            c_gpu = torch.matmul(a_gpu, b_gpu)
            torch.cuda.synchronize()
            return c_gpu.cpu().numpy()
        return np.matmul(a.astype(np.float32), b.astype(np.float32))
    
    def softmax(self, x: np.ndarray, axis: int = -1) -> np.ndarray:
        if self.is_gpu_available():
            x_gpu = torch.from_numpy(x.astype(np.float32)).to(DEVICE)
            y_gpu = torch.softmax(x_gpu, dim=axis)
            torch.cuda.synchronize()
            return y_gpu.cpu().numpy()
        x = x.astype(np.float32)
        x_max = np.max(x, axis=axis, keepdims=True)
        exp_x = np.exp(x - x_max)
        return exp_x / (np.sum(exp_x, axis=axis, keepdims=True) + 1e-8)
    
    def gelu(self, x: np.ndarray) -> np.ndarray:
        if self.is_gpu_available():
            x_gpu = torch.from_numpy(x.astype(np.float32)).to(DEVICE)
            y_gpu = torch.nn.functional.gelu(x_gpu)
            torch.cuda.synchronize()
            return y_gpu.cpu().numpy()
        x = x.astype(np.float32)
        return x * 0.5 * (1 + np.tanh(np.sqrt(2 / np.pi) * (x + 0.044715 * x**3)))


class GPUTransformer:
    def __init__(self, vocab_size: int, embed_dim: int, num_heads: int,
                 hidden_dim: int, num_layers: int, gpu_compute: GPUCompute):
        self.vocab_size = vocab_size
        self.embed_dim = embed_dim
        self.num_heads = num_heads
        self.hidden_dim = hidden_dim
        self.num_layers = num_layers
        self.gpu = gpu_compute
        
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
        self.memory_mb = (vocab_size * embed_dim + num_layers * (4 * embed_dim**2 + 2 * embed_dim * hidden_dim) + embed_dim * vocab_size) * 4 / (1024**2)
    
    def forward(self, token_ids: List[int]) -> np.ndarray:
        safe_ids = [min(max(0, t), self.vocab_size - 1) for t in token_ids]
        x = self.embeddings[safe_ids]
        
        for layer in self.layers:
            q = self.gpu.matmul(x, layer["W_q"])
            k = self.gpu.matmul(x, layer["W_k"])
            v = self.gpu.matmul(x, layer["W_v"])
            
            scores = self.gpu.matmul(q, k.T) / np.sqrt(self.embed_dim // self.num_heads)
            seq_len = len(token_ids)
            mask = np.triu(np.ones((seq_len, seq_len)) * -1e9, k=1)
            scores = scores + mask
            
            weights = self.gpu.softmax(scores)
            attn = self.gpu.matmul(weights, v)
            attn_out = self.gpu.matmul(attn, layer["W_o"])
            x = x + attn_out
            
            hidden = self.gpu.gelu(self.gpu.matmul(x, layer["W1"]))
            ffn_out = self.gpu.matmul(hidden, layer["W2"])
            x = x + ffn_out
        
        return self.gpu.matmul(x[-1:], self.output_proj)[0]


sys.path.insert(0, str(Path(__file__).parent.parent))
from Metal_Dead.core.metal_dead import MetalDead, MetalDeadConfig


class MetalDeadGPU(MetalDead):
    def __init__(self, config: MetalDeadConfig = None):
        self.gpu_compute = GPUCompute(ComputeBackend.AUTO)
        super().__init__(config)
        
        self.model = GPUTransformer(
            vocab_size=len(self.tokenizer),
            embed_dim=self.config.embed_dim,
            num_heads=self.config.num_heads,
            hidden_dim=self.config.hidden_dim,
            num_layers=self.config.num_layers,
            gpu_compute=self.gpu_compute
        )
        
        print(f"🚀 GPU: {'Activa' if self.gpu_compute.is_gpu_available() else 'CPU'}")
