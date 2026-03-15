"""
Sistema de Embeddings Pre-entrenados para ADead-BIB AI
======================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Soporta:
- Embeddings aleatorios optimizados
- Carga de GloVe (si disponible)
- Cuantización a int8 para bajo RAM
"""

import os
import json
import numpy as np
from pathlib import Path
from typing import Dict, List, Optional, Tuple


class EmbeddingManager:
    """
    Gestiona embeddings con soporte para:
    - Generación aleatoria semántica
    - Carga de pre-entrenados (GloVe)
    - Cuantización para bajo RAM
    """
    
    def __init__(self, vocab_size: int, embed_dim: int = 128, use_float16: bool = True):
        self.vocab_size = vocab_size
        self.embed_dim = embed_dim
        self.use_float16 = use_float16
        self.dtype = np.float16 if use_float16 else np.float32
        
        self.embeddings: Optional[np.ndarray] = None
        self.quantized: Optional[np.ndarray] = None
        self.quant_params: Optional[Dict] = None
        
        self._init_embeddings()
    
    def _init_embeddings(self):
        """Inicializa embeddings con distribución semántica."""
        print(f"Inicializando embeddings: {self.vocab_size}x{self.embed_dim}")
        
        # Usar distribución normal escalada
        self.embeddings = np.random.randn(
            self.vocab_size, self.embed_dim
        ).astype(self.dtype)
        
        # Escalar para estabilidad
        self.embeddings *= 0.02
        
        # Normalizar cada embedding
        norms = np.linalg.norm(self.embeddings, axis=1, keepdims=True)
        self.embeddings = self.embeddings / (norms + 1e-8)
        
        self._calc_ram()
    
    def _calc_ram(self):
        """Calcula RAM usada."""
        bytes_per = 2 if self.use_float16 else 4
        self.ram_mb = (self.vocab_size * self.embed_dim * bytes_per) / (1024 * 1024)
        print(f"RAM embeddings: {self.ram_mb:.2f} MB")
    
    def load_glove(self, glove_path: str, vocab: Dict[str, int]) -> int:
        """
        Carga embeddings de GloVe para palabras en el vocabulario.
        
        Args:
            glove_path: Ruta al archivo glove.6B.50d.txt (o similar)
            vocab: Diccionario palabra -> índice
            
        Returns:
            Número de palabras cargadas
        """
        if not Path(glove_path).exists():
            print(f"⚠️ Archivo GloVe no encontrado: {glove_path}")
            return 0
        
        print(f"Cargando GloVe desde: {glove_path}")
        
        loaded = 0
        with open(glove_path, 'r', encoding='utf-8', errors='ignore') as f:
            for line in f:
                parts = line.strip().split()
                if len(parts) < 2:
                    continue
                
                word = parts[0]
                if word in vocab:
                    idx = vocab[word]
                    if idx < self.vocab_size:
                        try:
                            vector = np.array([float(x) for x in parts[1:]], dtype=self.dtype)
                            # Ajustar dimensión si es necesario
                            if len(vector) >= self.embed_dim:
                                self.embeddings[idx] = vector[:self.embed_dim]
                            else:
                                self.embeddings[idx, :len(vector)] = vector
                            loaded += 1
                        except ValueError:
                            continue
        
        print(f"Palabras cargadas de GloVe: {loaded}/{len(vocab)}")
        return loaded
    
    def quantize(self, bits: int = 8) -> Tuple[np.ndarray, Dict]:
        """
        Cuantiza embeddings a int8 para reducir RAM.
        
        Returns:
            Tuple de (embeddings cuantizados, parámetros de cuantización)
        """
        print(f"Cuantizando embeddings a {bits} bits...")
        
        # Calcular min/max por dimensión
        min_vals = self.embeddings.min(axis=0)
        max_vals = self.embeddings.max(axis=0)
        
        # Escala por dimensión
        scales = (max_vals - min_vals) / (2**bits - 1)
        scales = np.where(scales == 0, 1, scales)  # Evitar división por cero
        
        # Cuantizar
        self.quantized = ((self.embeddings - min_vals) / scales).astype(np.uint8)
        
        self.quant_params = {
            "min_vals": min_vals.astype(np.float32),
            "scales": scales.astype(np.float32),
            "bits": bits
        }
        
        # Calcular RAM ahorrada
        quant_ram = (self.vocab_size * self.embed_dim) / (1024 * 1024)
        print(f"RAM cuantizada: {quant_ram:.2f} MB (antes: {self.ram_mb:.2f} MB)")
        print(f"Reducción: {(1 - quant_ram/self.ram_mb)*100:.1f}%")
        
        return self.quantized, self.quant_params
    
    def dequantize(self, quantized: np.ndarray = None) -> np.ndarray:
        """Dequantiza embeddings."""
        if quantized is None:
            quantized = self.quantized
        
        if self.quant_params is None:
            raise ValueError("No hay parámetros de cuantización")
        
        return (quantized.astype(np.float32) * self.quant_params["scales"] + 
                self.quant_params["min_vals"])
    
    def get(self, indices: List[int], use_quantized: bool = False) -> np.ndarray:
        """Obtiene embeddings para una lista de índices."""
        safe_indices = [min(max(0, i), self.vocab_size - 1) for i in indices]
        
        if use_quantized and self.quantized is not None:
            quant = self.quantized[safe_indices]
            return self.dequantize(quant)
        
        return self.embeddings[safe_indices]
    
    def similarity(self, idx1: int, idx2: int) -> float:
        """Calcula similitud coseno entre dos embeddings."""
        e1 = self.embeddings[idx1]
        e2 = self.embeddings[idx2]
        return float(np.dot(e1, e2) / (np.linalg.norm(e1) * np.linalg.norm(e2) + 1e-8))
    
    def most_similar(self, idx: int, top_k: int = 5) -> List[Tuple[int, float]]:
        """Encuentra los embeddings más similares."""
        target = self.embeddings[idx]
        
        # Calcular similitudes
        similarities = np.dot(self.embeddings, target)
        norms = np.linalg.norm(self.embeddings, axis=1) * np.linalg.norm(target)
        similarities = similarities / (norms + 1e-8)
        
        # Top-k (excluyendo el mismo)
        top_indices = np.argsort(similarities)[::-1][1:top_k+1]
        
        return [(int(i), float(similarities[i])) for i in top_indices]
    
    def save(self, filepath: str):
        """Guarda embeddings a archivo."""
        np.savez_compressed(
            filepath,
            embeddings=self.embeddings,
            quantized=self.quantized if self.quantized is not None else np.array([]),
            quant_min=self.quant_params["min_vals"] if self.quant_params else np.array([]),
            quant_scales=self.quant_params["scales"] if self.quant_params else np.array([]),
        )
        print(f"Embeddings guardados: {filepath}")
    
    def load(self, filepath: str):
        """Carga embeddings desde archivo."""
        data = np.load(filepath)
        self.embeddings = data["embeddings"]
        self.vocab_size, self.embed_dim = self.embeddings.shape
        
        if data["quantized"].size > 0:
            self.quantized = data["quantized"]
            self.quant_params = {
                "min_vals": data["quant_min"],
                "scales": data["quant_scales"],
                "bits": 8
            }
        
        self._calc_ram()
        print(f"Embeddings cargados: {self.vocab_size}x{self.embed_dim}")


class SemanticEmbeddings(EmbeddingManager):
    """
    Embeddings con inicialización semántica.
    Palabras similares tienen embeddings similares.
    """
    
    def __init__(self, vocab: Dict[str, int], embed_dim: int = 128):
        self.word_vocab = vocab
        super().__init__(len(vocab), embed_dim)
        self._init_semantic()
    
    def _init_semantic(self):
        """Inicializa embeddings con estructura semántica básica."""
        print("Aplicando estructura semántica...")
        
        # Grupos semánticos
        groups = {
            "numbers": ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"],
            "programming": ["code", "function", "class", "variable", "method", "loop", "array", "list"],
            "ai": ["ai", "ml", "model", "train", "learn", "neural", "network", "data"],
            "positive": ["good", "great", "amazing", "excellent", "wonderful", "best"],
            "negative": ["bad", "terrible", "awful", "worst", "poor"],
            "actions": ["run", "walk", "jump", "move", "go", "come", "start", "stop"],
        }
        
        # Asignar embeddings similares a palabras del mismo grupo
        for group_name, words in groups.items():
            # Vector base para el grupo
            base_vector = np.random.randn(self.embed_dim).astype(self.dtype) * 0.5
            
            for word in words:
                if word in self.word_vocab:
                    idx = self.word_vocab[word]
                    if idx < self.vocab_size:
                        # Agregar ruido pequeño al vector base
                        noise = np.random.randn(self.embed_dim).astype(self.dtype) * 0.1
                        self.embeddings[idx] = base_vector + noise
        
        # Re-normalizar
        norms = np.linalg.norm(self.embeddings, axis=1, keepdims=True)
        self.embeddings = self.embeddings / (norms + 1e-8)


def demo():
    """Demo del sistema de embeddings."""
    print("=" * 60)
    print("   Demo: Sistema de Embeddings")
    print("=" * 60)
    
    # Crear vocabulario de prueba
    vocab = {
        "<PAD>": 0, "<EOS>": 1, "<UNK>": 2, "<BOS>": 3,
        "hello": 4, "world": 5, "python": 6, "code": 7,
        "ai": 8, "ml": 9, "model": 10, "train": 11,
        "good": 12, "bad": 13, "great": 14, "terrible": 15,
        "one": 16, "two": 17, "three": 18, "four": 19,
    }
    
    # Crear embeddings semánticos
    print("\n📊 Creando embeddings semánticos...")
    embeddings = SemanticEmbeddings(vocab, embed_dim=64)
    
    # Probar similitud
    print("\n🔗 Similitudes:")
    pairs = [
        ("ai", "ml"),
        ("good", "great"),
        ("good", "bad"),
        ("one", "two"),
        ("hello", "ai"),
    ]
    
    for w1, w2 in pairs:
        if w1 in vocab and w2 in vocab:
            sim = embeddings.similarity(vocab[w1], vocab[w2])
            print(f"  {w1} <-> {w2}: {sim:.2%}")
    
    # Cuantizar
    print("\n📦 Cuantizando embeddings...")
    embeddings.quantize(bits=8)
    
    # Guardar
    embed_path = Path(__file__).parent / "embeddings.npz"
    embeddings.save(str(embed_path))
    
    print("\n" + "=" * 60)
    print(f"   ✅ Embeddings creados: {embeddings.vocab_size}x{embeddings.embed_dim}")
    print(f"   💾 RAM: {embeddings.ram_mb:.2f} MB")
    print("=" * 60)


if __name__ == "__main__":
    demo()
