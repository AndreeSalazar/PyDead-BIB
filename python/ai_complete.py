"""
ADead-BIB AI Complete - Sistema de IA Integrado
================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Combina Python (cabeza) + ADead-BIB (motor de rendimiento)

Componentes:
- Tokenizador con vocabulario expandible
- Embeddings en float16 (bajo RAM)
- Mecanismo de atención (4 heads)
- Feed-Forward Network
- Generación de texto
- Análisis de texto
- FFI Python ↔ ADead-BIB

Uso:
    python ai_complete.py

RAM Total: ~0.25 MB
"""

import os
import sys
import time
import json
from pathlib import Path
from typing import List, Dict, Optional, Tuple
from dataclasses import dataclass, field

# Agregar directorio
sys.path.insert(0, str(Path(__file__).parent))

# NumPy
import numpy as np

# ADead-BIB FFI
from adead_ffi import ADeadBIB

# =============================================================================
# CONFIGURACIÓN
# =============================================================================

@dataclass
class AIConfig:
    """Configuración de la IA."""
    vocab_size: int = 10000
    embed_dim: int = 64
    num_heads: int = 4
    hidden_dim: int = 256
    max_seq_len: int = 128
    temperature: float = 0.7
    use_float16: bool = True


# =============================================================================
# TOKENIZADOR
# =============================================================================

class Tokenizer:
    """Tokenizador con vocabulario expandible."""
    
    # Tokens especiales
    PAD = 0
    EOS = 1
    UNK = 2
    BOS = 3
    
    def __init__(self, vocab_size: int = 10000):
        self.vocab_size = vocab_size
        self.vocab: Dict[str, int] = {}
        self.inv_vocab: Dict[int, str] = {}
        self._build_vocab()
    
    def _build_vocab(self):
        """Construye vocabulario con palabras comunes."""
        # Tokens especiales
        special = ["<PAD>", "<EOS>", "<UNK>", "<BOS>"]
        
        # Palabras comunes (español + inglés + tech)
        words = [
            # Español básico
            "el", "la", "de", "que", "y", "en", "un", "ser", "se", "no",
            "haber", "por", "con", "su", "para", "como", "estar", "tener",
            "todo", "pero", "más", "hacer", "poder", "decir", "este", "ir",
            "hola", "mundo", "gracias", "bien", "mal", "si", "bueno", "malo",
            "grande", "pequeño", "nuevo", "viejo", "primero", "último",
            # Inglés básico
            "the", "be", "to", "of", "and", "a", "in", "that", "have", "i",
            "it", "for", "not", "on", "with", "he", "as", "you", "do", "at",
            "this", "but", "his", "by", "from", "they", "we", "say", "her",
            "hello", "world", "thanks", "yes", "good", "bad", "new", "old",
            "big", "small", "first", "last", "what", "how", "why", "when",
            # Tech/IA
            "ai", "model", "data", "train", "learn", "neural", "network",
            "python", "code", "function", "class", "variable", "array",
            "input", "output", "process", "compute", "memory", "fast",
            "token", "embed", "attention", "layer", "weight", "bias",
            # Números
            "zero", "one", "two", "three", "four", "five", "six", "seven",
            "eight", "nine", "ten", "hundred", "thousand",
            # Puntuación como tokens
            ".", ",", "!", "?", ":", ";",
        ]
        
        # Construir vocabulario
        all_words = special + words
        for i, w in enumerate(all_words):
            self.vocab[w.lower()] = i
            self.inv_vocab[i] = w.lower()
        
        # Expandir con variaciones
        self._expand()
        
        self.actual_size = len(self.vocab)
    
    def _expand(self):
        """Expande vocabulario con variaciones."""
        base = list(self.vocab.keys())[:50]
        suffixes = ["s", "ed", "ing", "er", "est", "ly", "tion"]
        prefixes = ["un", "re", "pre", "dis"]
        
        idx = len(self.vocab)
        for word in base:
            if len(word) > 2 and idx < self.vocab_size:
                for s in suffixes:
                    new = word + s
                    if new not in self.vocab:
                        self.vocab[new] = idx
                        self.inv_vocab[idx] = new
                        idx += 1
                for p in prefixes:
                    new = p + word
                    if new not in self.vocab:
                        self.vocab[new] = idx
                        self.inv_vocab[idx] = new
                        idx += 1
    
    def encode(self, text: str) -> List[int]:
        """Tokeniza texto."""
        tokens = [self.BOS]
        for word in text.lower().split():
            clean = ''.join(c for c in word if c.isalnum())
            if clean:
                tokens.append(self.vocab.get(clean, self.UNK))
        tokens.append(self.EOS)
        return tokens
    
    def decode(self, tokens: List[int]) -> str:
        """Decodifica tokens."""
        words = []
        for t in tokens:
            if t in [self.PAD, self.BOS]:
                continue
            if t == self.EOS:
                break
            words.append(self.inv_vocab.get(t, "<UNK>"))
        return ' '.join(words)
    
    def __len__(self):
        return self.actual_size


# =============================================================================
# EMBEDDINGS
# =============================================================================

class Embeddings:
    """Embeddings en float16 para bajo consumo de RAM."""
    
    def __init__(self, vocab_size: int, embed_dim: int, use_float16: bool = True):
        self.vocab_size = vocab_size
        self.embed_dim = embed_dim
        self.dtype = np.float16 if use_float16 else np.float32
        
        # Inicializar con distribución normal escalada
        self.weights = np.random.randn(vocab_size, embed_dim).astype(self.dtype)
        self.weights *= 0.02
        
        # RAM en MB
        bytes_per = 2 if use_float16 else 4
        self.ram_mb = (vocab_size * embed_dim * bytes_per) / (1024 * 1024)
    
    def forward(self, token_ids: List[int]) -> np.ndarray:
        """Obtiene embeddings para tokens."""
        safe_ids = [min(max(0, t), self.vocab_size - 1) for t in token_ids]
        return self.weights[safe_ids]
    
    def __call__(self, token_ids):
        return self.forward(token_ids)


# =============================================================================
# ATENCIÓN
# =============================================================================

class MultiHeadAttention:
    """Mecanismo de atención multi-cabeza."""
    
    def __init__(self, embed_dim: int, num_heads: int, use_float16: bool = True):
        self.embed_dim = embed_dim
        self.num_heads = num_heads
        self.head_dim = embed_dim // num_heads
        self.dtype = np.float16 if use_float16 else np.float32
        
        # Proyecciones Q, K, V, O
        scale = 0.02
        self.W_q = np.random.randn(embed_dim, embed_dim).astype(self.dtype) * scale
        self.W_k = np.random.randn(embed_dim, embed_dim).astype(self.dtype) * scale
        self.W_v = np.random.randn(embed_dim, embed_dim).astype(self.dtype) * scale
        self.W_o = np.random.randn(embed_dim, embed_dim).astype(self.dtype) * scale
        
        # RAM
        bytes_per = 2 if use_float16 else 4
        self.ram_mb = (4 * embed_dim * embed_dim * bytes_per) / (1024 * 1024)
    
    def forward(self, x: np.ndarray) -> np.ndarray:
        """Aplica atención."""
        # Proyecciones
        Q = x @ self.W_q
        K = x @ self.W_k
        V = x @ self.W_v
        
        # Scores
        scores = Q @ K.T / np.sqrt(self.head_dim)
        
        # Softmax
        exp_scores = np.exp(scores - np.max(scores, axis=-1, keepdims=True))
        weights = exp_scores / (np.sum(exp_scores, axis=-1, keepdims=True) + 1e-8)
        
        # Contexto
        context = weights @ V
        
        # Proyección de salida
        return context @ self.W_o
    
    def __call__(self, x):
        return self.forward(x)


# =============================================================================
# FEED-FORWARD NETWORK
# =============================================================================

class FeedForward:
    """Red feed-forward con activación GELU."""
    
    def __init__(self, embed_dim: int, hidden_dim: int, use_float16: bool = True):
        self.embed_dim = embed_dim
        self.hidden_dim = hidden_dim
        self.dtype = np.float16 if use_float16 else np.float32
        
        scale = 0.02
        self.W1 = np.random.randn(embed_dim, hidden_dim).astype(self.dtype) * scale
        self.W2 = np.random.randn(hidden_dim, embed_dim).astype(self.dtype) * scale
        
        # RAM
        bytes_per = 2 if use_float16 else 4
        self.ram_mb = ((embed_dim * hidden_dim + hidden_dim * embed_dim) * bytes_per) / (1024 * 1024)
    
    def gelu(self, x: np.ndarray) -> np.ndarray:
        """Activación GELU aproximada."""
        return x * 0.5 * (1 + np.tanh(np.sqrt(2 / np.pi) * (x + 0.044715 * x**3)))
    
    def forward(self, x: np.ndarray) -> np.ndarray:
        """Aplica FFN."""
        hidden = self.gelu(x @ self.W1)
        return hidden @ self.W2
    
    def __call__(self, x):
        return self.forward(x)


# =============================================================================
# TRANSFORMER BLOCK
# =============================================================================

class TransformerBlock:
    """Bloque transformer completo."""
    
    def __init__(self, embed_dim: int, num_heads: int, hidden_dim: int, use_float16: bool = True):
        self.attention = MultiHeadAttention(embed_dim, num_heads, use_float16)
        self.ffn = FeedForward(embed_dim, hidden_dim, use_float16)
        self.ram_mb = self.attention.ram_mb + self.ffn.ram_mb
    
    def forward(self, x: np.ndarray) -> np.ndarray:
        """Aplica bloque transformer."""
        # Atención + residual
        attn_out = self.attention(x)
        x = x + attn_out
        
        # FFN + residual
        ffn_out = self.ffn(x)
        x = x + ffn_out
        
        return x
    
    def __call__(self, x):
        return self.forward(x)


# =============================================================================
# MODELO COMPLETO
# =============================================================================

class ADeadAI:
    """
    Sistema de IA completo con bajo consumo de RAM.
    Combina Python + ADead-BIB.
    """
    
    def __init__(self, config: AIConfig = None):
        self.config = config or AIConfig()
        self.adead = ADeadBIB()
        
        print("=" * 60)
        print("   ADead-BIB AI Complete")
        print("=" * 60)
        
        # Componentes
        self.tokenizer = Tokenizer(self.config.vocab_size)
        self.embeddings = Embeddings(
            len(self.tokenizer),
            self.config.embed_dim,
            self.config.use_float16
        )
        self.transformer = TransformerBlock(
            self.config.embed_dim,
            self.config.num_heads,
            self.config.hidden_dim,
            self.config.use_float16
        )
        
        # Capa de salida
        dtype = np.float16 if self.config.use_float16 else np.float32
        self.output_proj = np.random.randn(
            self.config.embed_dim,
            len(self.tokenizer)
        ).astype(dtype) * 0.02
        
        # Calcular RAM total
        self._calc_ram()
        self._print_stats()
    
    def _calc_ram(self):
        """Calcula RAM total."""
        bytes_per = 2 if self.config.use_float16 else 4
        output_ram = (self.config.embed_dim * len(self.tokenizer) * bytes_per) / (1024 * 1024)
        
        self.total_ram = (
            self.embeddings.ram_mb +
            self.transformer.ram_mb +
            output_ram
        )
    
    def _print_stats(self):
        """Imprime estadísticas."""
        print(f"\n📊 Estadísticas:")
        print(f"  Vocabulario: {len(self.tokenizer)} tokens")
        print(f"  Embeddings:  {self.config.embed_dim} dim")
        print(f"  Atención:    {self.config.num_heads} heads")
        print(f"  FFN:         {self.config.hidden_dim} hidden")
        print(f"\n💾 RAM Total:  {self.total_ram:.2f} MB")
        print("=" * 60)
    
    def generate(self, prompt: str, max_tokens: int = 30) -> str:
        """Genera texto a partir de un prompt."""
        tokens = self.tokenizer.encode(prompt)
        
        for _ in range(max_tokens):
            # Limitar contexto
            ctx = tokens[-self.config.max_seq_len:]
            
            # Forward pass
            embeds = self.embeddings(ctx)
            hidden = self.transformer(embeds)
            logits = hidden[-1] @ self.output_proj
            
            # Temperatura
            logits = logits / self.config.temperature
            
            # Softmax
            exp_logits = np.exp(logits - np.max(logits))
            probs = exp_logits / np.sum(exp_logits)
            
            # Muestrear
            next_token = np.random.choice(len(probs), p=probs.astype(np.float64))
            tokens.append(int(next_token))
            
            if next_token == self.tokenizer.EOS:
                break
        
        return self.tokenizer.decode(tokens)
    
    def analyze(self, text: str) -> Dict:
        """Analiza texto."""
        tokens = self.tokenizer.encode(text)
        embeds = self.embeddings(tokens)
        
        return {
            "num_tokens": len(tokens),
            "num_words": len(text.split()),
            "num_chars": len(text),
            "unique_tokens": len(set(tokens)),
            "unk_count": tokens.count(self.tokenizer.UNK),
            "embed_mean": float(np.mean(embeds)),
            "embed_std": float(np.std(embeds)),
        }
    
    def chat(self, message: str) -> str:
        """Interfaz de chat."""
        return self.generate(message, max_tokens=20)
    
    def similarity(self, text1: str, text2: str) -> float:
        """Calcula similitud entre dos textos."""
        tokens1 = self.tokenizer.encode(text1)
        tokens2 = self.tokenizer.encode(text2)
        
        embed1 = np.mean(self.embeddings(tokens1), axis=0)
        embed2 = np.mean(self.embeddings(tokens2), axis=0)
        
        # Similitud coseno
        dot = np.dot(embed1, embed2)
        norm = np.linalg.norm(embed1) * np.linalg.norm(embed2)
        return float(dot / (norm + 1e-8))


# =============================================================================
# DEMO
# =============================================================================

def demo():
    """Demo del sistema de IA completo."""
    print("\n" + "=" * 60)
    print("   DEMO: ADead-BIB AI Complete")
    print("=" * 60)
    
    # Configuración optimizada para bajo RAM
    config = AIConfig(
        vocab_size=5000,
        embed_dim=64,
        num_heads=4,
        hidden_dim=128,
        max_seq_len=64,
        temperature=0.8,
        use_float16=True
    )
    
    # Crear IA
    ai = ADeadAI(config)
    
    # Test de análisis
    print("\n📝 Análisis de Texto:")
    print("-" * 40)
    
    texts = [
        "Hello world, this is a test.",
        "Hola mundo, esto es una prueba.",
        "Python and AI working together.",
    ]
    
    for text in texts:
        stats = ai.analyze(text)
        print(f"\n'{text[:40]}...'")
        print(f"  Tokens: {stats['num_tokens']}, UNK: {stats['unk_count']}")
    
    # Test de generación
    print("\n\n🤖 Generación de Texto:")
    print("-" * 40)
    
    prompts = ["Hello", "The AI", "Python is"]
    
    for prompt in prompts:
        start = time.time()
        response = ai.generate(prompt, max_tokens=15)
        elapsed = (time.time() - start) * 1000
        print(f"\nPrompt: '{prompt}'")
        print(f"Output: '{response}'")
        print(f"Tiempo: {elapsed:.1f} ms")
    
    # Test de similitud
    print("\n\n📊 Similitud de Textos:")
    print("-" * 40)
    
    pairs = [
        ("hello world", "hello there"),
        ("python code", "python programming"),
        ("hello world", "goodbye moon"),
    ]
    
    for t1, t2 in pairs:
        sim = ai.similarity(t1, t2)
        print(f"'{t1}' vs '{t2}': {sim:.2%}")
    
    print("\n" + "=" * 60)
    print("   ✅ Demo Completada")
    print(f"   💾 RAM Total: {ai.total_ram:.2f} MB")
    print("=" * 60)


def main():
    """Función principal."""
    print("=" * 60)
    print("   ADead-BIB + Python: IA Completa")
    print("   Bajo Consumo de RAM")
    print("=" * 60)
    
    demo()


if __name__ == "__main__":
    main()
