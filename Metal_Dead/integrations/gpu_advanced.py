"""
GPU Advanced para Metal-Dead
=============================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪

Optimizaciones avanzadas: Flash Attention, BF16, Tensor Cores
"""

import sys
import time
from pathlib import Path
from typing import Dict, List, Optional
from dataclasses import dataclass
from enum import Enum

import numpy as np

HAS_TORCH = False
TORCH_CUDA = False

try:
    import torch
    import torch.nn.functional as F
    HAS_TORCH = True
    TORCH_CUDA = torch.cuda.is_available()
    if TORCH_CUDA:
        DEVICE = torch.device("cuda")
        GPU_NAME = torch.cuda.get_device_name(0)
        GPU_MEMORY = torch.cuda.get_device_properties(0).total_memory / (1024**3)
        GPU_CAPABILITY = torch.cuda.get_device_capability(0)
        HAS_BF16 = GPU_CAPABILITY[0] >= 8
    else:
        DEVICE = torch.device("cpu")
        GPU_NAME = None
        GPU_MEMORY = 0
        GPU_CAPABILITY = (0, 0)
        HAS_BF16 = False
except ImportError:
    HAS_BF16 = False


class PrecisionMode(Enum):
    FP32 = "fp32"
    FP16 = "fp16"
    BF16 = "bf16"
    AUTO = "auto"


@dataclass
class GPUConfig:
    precision: PrecisionMode = PrecisionMode.AUTO
    use_flash_attention: bool = True
    persistent_weights: bool = True
    benchmark_cudnn: bool = True


class GPUAdvanced:
    def __init__(self, config: GPUConfig = None):
        if not HAS_TORCH or not TORCH_CUDA:
            raise RuntimeError("CUDA no disponible")
        
        self.config = config or GPUConfig()
        
        if self.config.precision == PrecisionMode.AUTO:
            self.dtype = torch.bfloat16 if HAS_BF16 else torch.float16
            self.precision_name = "BF16" if HAS_BF16 else "FP16"
        elif self.config.precision == PrecisionMode.BF16:
            self.dtype = torch.bfloat16
            self.precision_name = "BF16"
        elif self.config.precision == PrecisionMode.FP16:
            self.dtype = torch.float16
            self.precision_name = "FP16"
        else:
            self.dtype = torch.float32
            self.precision_name = "FP32"
        
        if self.config.benchmark_cudnn:
            torch.backends.cudnn.benchmark = True
        if GPU_CAPABILITY[0] >= 8:
            torch.backends.cuda.matmul.allow_tf32 = True
        
        self.weight_cache: Dict[str, torch.Tensor] = {}
        self._print_config()
    
    def _print_config(self):
        print("\n" + "=" * 60)
        print("   🚀 GPU Advanced - Metal-Dead")
        print("=" * 60)
        print(f"\n🎮 GPU: {GPU_NAME}")
        print(f"   VRAM: {GPU_MEMORY:.1f} GB")
        print(f"   Precisión: {self.precision_name}")
        print(f"   Flash Attention: {'✅' if self.config.use_flash_attention else '❌'}")
        print("=" * 60)
    
    def cache_weights(self, name: str, weights: np.ndarray) -> torch.Tensor:
        if name not in self.weight_cache:
            self.weight_cache[name] = torch.from_numpy(weights.astype(np.float32)).to(DEVICE, dtype=self.dtype)
        return self.weight_cache[name]
    
    def matmul(self, a: np.ndarray, b: np.ndarray) -> np.ndarray:
        a_gpu = torch.from_numpy(a.astype(np.float32)).to(DEVICE, dtype=self.dtype)
        b_gpu = torch.from_numpy(b.astype(np.float32)).to(DEVICE, dtype=self.dtype)
        c_gpu = torch.matmul(a_gpu, b_gpu)
        torch.cuda.synchronize()
        return c_gpu.float().cpu().numpy()
    
    def flash_attention(self, q: np.ndarray, k: np.ndarray, v: np.ndarray, causal: bool = True) -> np.ndarray:
        q_gpu = torch.from_numpy(q.astype(np.float32)).to(DEVICE, dtype=self.dtype).unsqueeze(0).unsqueeze(0)
        k_gpu = torch.from_numpy(k.astype(np.float32)).to(DEVICE, dtype=self.dtype).unsqueeze(0).unsqueeze(0)
        v_gpu = torch.from_numpy(v.astype(np.float32)).to(DEVICE, dtype=self.dtype).unsqueeze(0).unsqueeze(0)
        
        if hasattr(F, 'scaled_dot_product_attention'):
            output = F.scaled_dot_product_attention(q_gpu, k_gpu, v_gpu, is_causal=causal)
        else:
            scale = 1.0 / (q.shape[-1] ** 0.5)
            scores = torch.matmul(q_gpu, k_gpu.transpose(-2, -1)) * scale
            if causal:
                seq_len = q.shape[0]
                mask = torch.triu(torch.ones(seq_len, seq_len, device=DEVICE) * -1e9, diagonal=1)
                scores = scores + mask
            weights = F.softmax(scores, dim=-1)
            output = torch.matmul(weights, v_gpu)
        
        torch.cuda.synchronize()
        return output.squeeze(0).squeeze(0).float().cpu().numpy()
    
    def transformer_layer_fused(self, x: np.ndarray, w_q, w_k, w_v, w_o, w1, w2, num_heads: int = 8, layer_id: int = 0) -> np.ndarray:
        seq_len, dim = x.shape
        x_gpu = torch.from_numpy(x.astype(np.float32)).to(DEVICE, dtype=self.dtype)
        
        prefix = f"layer_{layer_id}_"
        w_q_gpu = self.cache_weights(f"{prefix}w_q", w_q)
        w_k_gpu = self.cache_weights(f"{prefix}w_k", w_k)
        w_v_gpu = self.cache_weights(f"{prefix}w_v", w_v)
        w_o_gpu = self.cache_weights(f"{prefix}w_o", w_o)
        w1_gpu = self.cache_weights(f"{prefix}w1", w1)
        w2_gpu = self.cache_weights(f"{prefix}w2", w2)
        
        q = torch.matmul(x_gpu, w_q_gpu)
        k = torch.matmul(x_gpu, w_k_gpu)
        v = torch.matmul(x_gpu, w_v_gpu)
        
        head_dim = dim // num_heads
        q = q.view(seq_len, num_heads, head_dim).transpose(0, 1)
        k = k.view(seq_len, num_heads, head_dim).transpose(0, 1)
        v = v.view(seq_len, num_heads, head_dim).transpose(0, 1)
        
        if hasattr(F, 'scaled_dot_product_attention'):
            attn_out = F.scaled_dot_product_attention(q.unsqueeze(0), k.unsqueeze(0), v.unsqueeze(0), is_causal=True).squeeze(0)
        else:
            scale = 1.0 / (head_dim ** 0.5)
            scores = torch.matmul(q, k.transpose(-2, -1)) * scale
            mask = torch.triu(torch.ones(seq_len, seq_len, device=DEVICE) * -1e9, diagonal=1)
            scores = scores + mask
            weights = F.softmax(scores, dim=-1)
            attn_out = torch.matmul(weights, v)
        
        attn_out = attn_out.transpose(0, 1).contiguous().view(seq_len, dim)
        attn_out = torch.matmul(attn_out, w_o_gpu)
        x_gpu = x_gpu + attn_out
        
        hidden = F.gelu(torch.matmul(x_gpu, w1_gpu))
        ffn_out = torch.matmul(hidden, w2_gpu)
        x_gpu = x_gpu + ffn_out
        
        torch.cuda.synchronize()
        return x_gpu.float().cpu().numpy()
    
    def get_metrics(self) -> Dict:
        return {"precision": self.precision_name, "gpu": GPU_NAME, "vram_gb": GPU_MEMORY}


sys.path.insert(0, str(Path(__file__).parent.parent))
from Metal_Dead.core.metal_dead import MetalDead, MetalDeadConfig


class AdvancedGPUTransformer:
    def __init__(self, vocab_size: int, embed_dim: int, num_heads: int, hidden_dim: int, num_layers: int, config: GPUConfig = None):
        self.vocab_size = vocab_size
        self.embed_dim = embed_dim
        self.num_heads = num_heads
        self.hidden_dim = hidden_dim
        self.num_layers = num_layers
        self.gpu = GPUAdvanced(config)
        
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
        
        print("📦 Cacheando pesos en GPU...")
        self.gpu.cache_weights("embeddings", self.embeddings)
        self.gpu.cache_weights("output_proj", self.output_proj)
        for i, layer in enumerate(self.layers):
            for name, weight in layer.items():
                self.gpu.cache_weights(f"layer_{i}_{name}", weight)
    
    def forward(self, token_ids: List[int]) -> np.ndarray:
        safe_ids = [min(max(0, t), self.vocab_size - 1) for t in token_ids]
        x = self.embeddings[safe_ids]
        
        for i, layer in enumerate(self.layers):
            x = self.gpu.transformer_layer_fused(x, layer["W_q"], layer["W_k"], layer["W_v"], layer["W_o"], layer["W1"], layer["W2"], self.num_heads, i)
        
        return self.gpu.matmul(x[-1:], self.output_proj)[0]


class MetalDeadGPUMax(MetalDead):
    def __init__(self, config: MetalDeadConfig = None):
        self.gpu_config = GPUConfig(precision=PrecisionMode.AUTO, use_flash_attention=True, persistent_weights=True)
        super().__init__(config)
        
        self.model = AdvancedGPUTransformer(
            vocab_size=len(self.tokenizer),
            embed_dim=self.config.embed_dim,
            num_heads=self.config.num_heads,
            hidden_dim=self.config.hidden_dim,
            num_layers=self.config.num_layers,
            config=self.gpu_config
        )
        
        print(f"\n🔥 Metal-Dead GPU MAX inicializado")
        print(f"   Precisión: {self.model.gpu.precision_name}")
        print(f"   Flash Attention: ✅")
