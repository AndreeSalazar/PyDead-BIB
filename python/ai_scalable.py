"""
ADead-BIB AI Scalable - Sistema de IA Escalable
================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Versión mejorada con:
- Tokenizador BPE básico
- Vocabulario expandible (10K+)
- Caché de embeddings
- Mejor integración Python + ADead-BIB

Uso:
    python ai_scalable.py
"""

import os
import sys
import time
import json
import re
from pathlib import Path
from typing import List, Dict, Optional, Tuple, Set
from dataclasses import dataclass, field
from collections import Counter, defaultdict

sys.path.insert(0, str(Path(__file__).parent))

import numpy as np
from adead_ffi import ADeadBIB


# =============================================================================
# CONFIGURACIÓN
# =============================================================================

@dataclass
class ScalableConfig:
    """Configuración escalable."""
    vocab_size: int = 10000
    embed_dim: int = 128
    num_heads: int = 8
    hidden_dim: int = 512
    num_layers: int = 2
    max_seq_len: int = 256
    temperature: float = 0.7
    use_float16: bool = True
    use_bpe: bool = True


# =============================================================================
# TOKENIZADOR BPE
# =============================================================================

class BPETokenizer:
    """
    Tokenizador BPE (Byte Pair Encoding).
    Reduce tokens desconocidos dividiendo palabras en subpalabras.
    """
    
    PAD = 0
    EOS = 1
    UNK = 2
    BOS = 3
    
    def __init__(self, vocab_size: int = 10000):
        self.vocab_size = vocab_size
        self.vocab: Dict[str, int] = {}
        self.inv_vocab: Dict[int, str] = {}
        self.merges: Dict[Tuple[str, str], str] = {}
        self.token_freqs: Counter = Counter()
        
        self._init_base_vocab()
    
    def _init_base_vocab(self):
        """Inicializa vocabulario base con caracteres y palabras comunes."""
        # Tokens especiales
        special = ["<PAD>", "<EOS>", "<UNK>", "<BOS>", "<SEP>", "<MASK>"]
        
        # Caracteres base (ASCII imprimibles)
        chars = list("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
        chars += list(".,!?;:'-\"()[]{}@#$%^&*+=<>/\\|`~_ ")
        chars += list("áéíóúñüÁÉÍÓÚÑÜ¿¡")  # Español
        
        # Palabras muy comunes (subpalabras frecuentes)
        common_subwords = [
            # Prefijos
            "un", "re", "pre", "dis", "mis", "over", "under", "out", "in", "im",
            "de", "en", "con", "para", "por", "sin", "sobre", "entre",
            # Sufijos
            "ing", "ed", "er", "est", "ly", "tion", "ness", "ment", "able", "ible",
            "ando", "endo", "ado", "ido", "ción", "mente", "dad", "oso", "osa",
            # Palabras cortas comunes
            "the", "be", "to", "of", "and", "a", "in", "that", "have", "it",
            "el", "la", "de", "que", "y", "en", "un", "ser", "se", "no",
            "is", "was", "for", "on", "are", "with", "as", "at", "by", "this",
            "es", "por", "con", "su", "para", "como", "más", "pero", "todo",
            # Tech
            "ai", "ml", "data", "code", "func", "var", "class", "def", "return",
            "python", "java", "rust", "cpp", "html", "css", "json", "api",
        ]
        
        # Construir vocabulario
        idx = 0
        for token in special:
            self.vocab[token] = idx
            self.inv_vocab[idx] = token
            idx += 1
        
        for char in chars:
            if char not in self.vocab:
                self.vocab[char] = idx
                self.inv_vocab[idx] = char
                idx += 1
        
        for word in common_subwords:
            if word not in self.vocab:
                self.vocab[word] = idx
                self.inv_vocab[idx] = word
                idx += 1
        
        self.base_vocab_size = idx
        print(f"Vocabulario base: {self.base_vocab_size} tokens")
    
    def train(self, texts: List[str], num_merges: int = 5000):
        """
        Entrena BPE en un corpus de texto.
        Aprende las fusiones más frecuentes.
        """
        print(f"Entrenando BPE con {len(texts)} textos...")
        
        # Tokenizar a nivel de caracteres
        word_freqs = Counter()
        for text in texts:
            words = text.lower().split()
            for word in words:
                # Agregar marcador de fin de palabra
                word_chars = tuple(list(word) + ["</w>"])
                word_freqs[word_chars] += 1
        
        # Aprender fusiones
        for i in range(num_merges):
            # Contar pares
            pair_freqs = Counter()
            for word, freq in word_freqs.items():
                for j in range(len(word) - 1):
                    pair = (word[j], word[j + 1])
                    pair_freqs[pair] += freq
            
            if not pair_freqs:
                break
            
            # Mejor par
            best_pair = pair_freqs.most_common(1)[0][0]
            new_token = best_pair[0] + best_pair[1]
            
            # Agregar al vocabulario
            if new_token not in self.vocab:
                idx = len(self.vocab)
                self.vocab[new_token] = idx
                self.inv_vocab[idx] = new_token
            
            # Guardar fusión
            self.merges[best_pair] = new_token
            
            # Aplicar fusión
            new_word_freqs = Counter()
            for word, freq in word_freqs.items():
                new_word = []
                j = 0
                while j < len(word):
                    if j < len(word) - 1 and (word[j], word[j + 1]) == best_pair:
                        new_word.append(new_token)
                        j += 2
                    else:
                        new_word.append(word[j])
                        j += 1
                new_word_freqs[tuple(new_word)] = freq
            word_freqs = new_word_freqs
            
            if (i + 1) % 1000 == 0:
                print(f"  {i + 1}/{num_merges} fusiones aprendidas")
        
        print(f"Vocabulario final: {len(self.vocab)} tokens")
    
    def _apply_bpe(self, word: str) -> List[str]:
        """Aplica BPE a una palabra."""
        if not word:
            return []
        
        # Convertir a lista de caracteres + marcador
        tokens = list(word) + ["</w>"]
        
        # Aplicar fusiones
        changed = True
        while changed:
            changed = False
            new_tokens = []
            i = 0
            while i < len(tokens):
                if i < len(tokens) - 1:
                    pair = (tokens[i], tokens[i + 1])
                    if pair in self.merges:
                        new_tokens.append(self.merges[pair])
                        i += 2
                        changed = True
                        continue
                new_tokens.append(tokens[i])
                i += 1
            tokens = new_tokens
        
        # Remover marcador de fin
        if tokens and tokens[-1] == "</w>":
            tokens = tokens[:-1]
        
        return tokens
    
    def encode(self, text: str) -> List[int]:
        """Tokeniza texto a IDs."""
        tokens = [self.BOS]
        
        # Dividir en palabras
        words = re.findall(r'\w+|[^\w\s]', text.lower())
        
        for word in words:
            # Aplicar BPE
            subwords = self._apply_bpe(word)
            for sw in subwords:
                if sw in self.vocab:
                    tokens.append(self.vocab[sw])
                else:
                    # Fallback a caracteres
                    for char in sw:
                        tokens.append(self.vocab.get(char, self.UNK))
        
        tokens.append(self.EOS)
        return tokens
    
    def decode(self, tokens: List[int]) -> str:
        """Decodifica IDs a texto."""
        words = []
        current_word = []
        
        for t in tokens:
            if t in [self.PAD, self.BOS]:
                continue
            if t == self.EOS:
                break
            
            token = self.inv_vocab.get(t, "<UNK>")
            
            if token == " ":
                if current_word:
                    words.append("".join(current_word))
                    current_word = []
            else:
                current_word.append(token)
        
        if current_word:
            words.append("".join(current_word))
        
        return " ".join(words)
    
    def __len__(self):
        return len(self.vocab)


# =============================================================================
# EMBEDDINGS CON CACHÉ
# =============================================================================

class CachedEmbeddings:
    """Embeddings con caché para tokens frecuentes."""
    
    def __init__(self, vocab_size: int, embed_dim: int, use_float16: bool = True):
        self.vocab_size = vocab_size
        self.embed_dim = embed_dim
        self.dtype = np.float16 if use_float16 else np.float32
        
        # Embeddings principales
        self.weights = np.random.randn(vocab_size, embed_dim).astype(self.dtype)
        self.weights *= 0.02
        
        # Caché para tokens frecuentes
        self.cache: Dict[int, np.ndarray] = {}
        self.cache_hits = 0
        self.cache_misses = 0
        
        # RAM
        bytes_per = 2 if use_float16 else 4
        self.ram_mb = (vocab_size * embed_dim * bytes_per) / (1024 * 1024)
    
    def get(self, token_ids: List[int]) -> np.ndarray:
        """Obtiene embeddings con caché."""
        result = np.zeros((len(token_ids), self.embed_dim), dtype=self.dtype)
        
        for i, tid in enumerate(token_ids):
            tid = min(max(0, tid), self.vocab_size - 1)
            
            if tid in self.cache:
                result[i] = self.cache[tid]
                self.cache_hits += 1
            else:
                result[i] = self.weights[tid]
                self.cache_misses += 1
                
                # Agregar a caché si hay espacio
                if len(self.cache) < 1000:
                    self.cache[tid] = self.weights[tid].copy()
        
        return result
    
    def cache_stats(self) -> Dict:
        """Estadísticas de caché."""
        total = self.cache_hits + self.cache_misses
        hit_rate = self.cache_hits / total if total > 0 else 0
        return {
            "hits": self.cache_hits,
            "misses": self.cache_misses,
            "hit_rate": hit_rate,
            "cache_size": len(self.cache),
        }


# =============================================================================
# TRANSFORMER ESCALABLE
# =============================================================================

class ScalableTransformer:
    """Transformer escalable con múltiples capas."""
    
    def __init__(self, config: ScalableConfig):
        self.config = config
        self.dtype = np.float16 if config.use_float16 else np.float32
        
        # Capas
        self.layers = []
        for _ in range(config.num_layers):
            layer = {
                "W_q": np.random.randn(config.embed_dim, config.embed_dim).astype(self.dtype) * 0.02,
                "W_k": np.random.randn(config.embed_dim, config.embed_dim).astype(self.dtype) * 0.02,
                "W_v": np.random.randn(config.embed_dim, config.embed_dim).astype(self.dtype) * 0.02,
                "W_o": np.random.randn(config.embed_dim, config.embed_dim).astype(self.dtype) * 0.02,
                "W1": np.random.randn(config.embed_dim, config.hidden_dim).astype(self.dtype) * 0.02,
                "W2": np.random.randn(config.hidden_dim, config.embed_dim).astype(self.dtype) * 0.02,
            }
            self.layers.append(layer)
        
        # RAM
        bytes_per = 2 if config.use_float16 else 4
        params_per_layer = (4 * config.embed_dim * config.embed_dim + 
                          2 * config.embed_dim * config.hidden_dim)
        self.ram_mb = (config.num_layers * params_per_layer * bytes_per) / (1024 * 1024)
    
    def forward(self, x: np.ndarray) -> np.ndarray:
        """Forward pass por todas las capas."""
        head_dim = self.config.embed_dim // self.config.num_heads
        
        for layer in self.layers:
            # Atención
            Q = x @ layer["W_q"]
            K = x @ layer["W_k"]
            V = x @ layer["W_v"]
            
            scores = Q @ K.T / np.sqrt(head_dim)
            exp_scores = np.exp(scores - np.max(scores, axis=-1, keepdims=True))
            weights = exp_scores / (np.sum(exp_scores, axis=-1, keepdims=True) + 1e-8)
            
            attn_out = weights @ V @ layer["W_o"]
            x = x + attn_out
            
            # FFN
            hidden = x @ layer["W1"]
            hidden = hidden * 0.5 * (1 + np.tanh(np.sqrt(2 / np.pi) * (hidden + 0.044715 * hidden**3)))
            ffn_out = hidden @ layer["W2"]
            x = x + ffn_out
        
        return x


# =============================================================================
# MODELO ESCALABLE
# =============================================================================

class ScalableAI:
    """Sistema de IA escalable."""
    
    def __init__(self, config: ScalableConfig = None):
        self.config = config or ScalableConfig()
        self.adead = ADeadBIB()
        
        print("=" * 60)
        print("   ADead-BIB AI Scalable")
        print("=" * 60)
        
        # Tokenizador
        self.tokenizer = BPETokenizer(self.config.vocab_size)
        
        # Entrenar BPE con corpus básico
        corpus = self._generate_training_corpus()
        self.tokenizer.train(corpus, num_merges=min(2000, self.config.vocab_size - 500))
        
        # Componentes
        self.embeddings = CachedEmbeddings(
            len(self.tokenizer),
            self.config.embed_dim,
            self.config.use_float16
        )
        
        self.transformer = ScalableTransformer(self.config)
        
        # Capa de salida
        self.output_proj = np.random.randn(
            self.config.embed_dim,
            len(self.tokenizer)
        ).astype(np.float16 if self.config.use_float16 else np.float32) * 0.02
        
        self._calc_ram()
        self._print_stats()
    
    def _generate_training_corpus(self) -> List[str]:
        """Genera corpus de entrenamiento."""
        corpus = [
            # Inglés
            "The quick brown fox jumps over the lazy dog",
            "Hello world this is a test of the AI system",
            "Python programming is fun and powerful",
            "Machine learning and artificial intelligence",
            "Natural language processing with transformers",
            "Deep learning neural networks are amazing",
            "Data science and analytics for business",
            "Software engineering best practices",
            "Computer vision and image recognition",
            "Reinforcement learning for games",
            # Español
            "Hola mundo esta es una prueba del sistema",
            "La inteligencia artificial es el futuro",
            "Programación en Python es muy útil",
            "El aprendizaje automático cambia todo",
            "Procesamiento de lenguaje natural",
            "Redes neuronales profundas funcionan bien",
            "Ciencia de datos para negocios",
            "Ingeniería de software moderna",
            "Visión por computadora avanzada",
            "Aprendizaje por refuerzo para juegos",
            # Tech
            "API REST JSON HTTP request response",
            "Database SQL NoSQL MongoDB PostgreSQL",
            "Cloud computing AWS Azure GCP",
            "Docker Kubernetes containers orchestration",
            "Git version control branching merging",
            "Testing unit integration end to end",
            "Security authentication authorization",
            "Performance optimization caching",
            "Microservices architecture design patterns",
            "DevOps CI CD pipeline automation",
        ]
        
        # Expandir corpus
        expanded = []
        for text in corpus:
            expanded.append(text)
            expanded.append(text.lower())
            expanded.append(text.upper())
            words = text.split()
            for i in range(0, len(words) - 2, 2):
                expanded.append(" ".join(words[i:i+3]))
        
        return expanded
    
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
        print(f"\n📊 Configuración:")
        print(f"  Vocabulario: {len(self.tokenizer)} tokens")
        print(f"  Embeddings:  {self.config.embed_dim} dim")
        print(f"  Atención:    {self.config.num_heads} heads")
        print(f"  Capas:       {self.config.num_layers}")
        print(f"  FFN:         {self.config.hidden_dim} hidden")
        print(f"\n💾 RAM Total:  {self.total_ram:.2f} MB")
        print("=" * 60)
    
    def generate(self, prompt: str, max_tokens: int = 50) -> str:
        """Genera texto."""
        tokens = self.tokenizer.encode(prompt)
        
        for _ in range(max_tokens):
            ctx = tokens[-self.config.max_seq_len:]
            embeds = self.embeddings.get(ctx)
            hidden = self.transformer.forward(embeds)
            logits = hidden[-1] @ self.output_proj
            
            logits = logits / self.config.temperature
            exp_logits = np.exp(logits - np.max(logits))
            probs = exp_logits / np.sum(exp_logits)
            
            next_token = np.random.choice(len(probs), p=probs.astype(np.float64))
            tokens.append(int(next_token))
            
            if next_token == self.tokenizer.EOS:
                break
        
        return self.tokenizer.decode(tokens)
    
    def analyze(self, text: str) -> Dict:
        """Analiza texto."""
        tokens = self.tokenizer.encode(text)
        
        return {
            "num_tokens": len(tokens),
            "num_words": len(text.split()),
            "compression_ratio": len(text.split()) / len(tokens) if tokens else 0,
            "unk_count": tokens.count(self.tokenizer.UNK),
            "unk_ratio": tokens.count(self.tokenizer.UNK) / len(tokens) if tokens else 0,
            "cache_stats": self.embeddings.cache_stats(),
        }
    
    def benchmark(self, num_iterations: int = 100) -> Dict:
        """Benchmark de rendimiento."""
        prompts = ["Hello", "The AI", "Python is", "Machine learning"]
        
        times = []
        for _ in range(num_iterations):
            prompt = prompts[_ % len(prompts)]
            start = time.time()
            self.generate(prompt, max_tokens=10)
            times.append(time.time() - start)
        
        return {
            "avg_time_ms": np.mean(times) * 1000,
            "min_time_ms": np.min(times) * 1000,
            "max_time_ms": np.max(times) * 1000,
            "tokens_per_sec": 10 / np.mean(times),
        }


# =============================================================================
# DEMO
# =============================================================================

def demo():
    """Demo del sistema escalable."""
    print("\n" + "=" * 60)
    print("   DEMO: ADead-BIB AI Scalable")
    print("=" * 60)
    
    # Configuración escalable
    config = ScalableConfig(
        vocab_size=5000,
        embed_dim=128,
        num_heads=8,
        hidden_dim=256,
        num_layers=2,
        max_seq_len=128,
        temperature=0.8,
        use_float16=True,
        use_bpe=True
    )
    
    ai = ScalableAI(config)
    
    # Análisis
    print("\n📝 Análisis de Texto (con BPE):")
    print("-" * 40)
    
    texts = [
        "Hello world, this is a test.",
        "Programming in Python is amazing.",
        "Inteligencia artificial y aprendizaje automático.",
        "The transformer architecture revolutionized NLP.",
    ]
    
    for text in texts:
        stats = ai.analyze(text)
        print(f"\n'{text[:45]}...'")
        print(f"  Tokens: {stats['num_tokens']}, UNK: {stats['unk_count']}")
        print(f"  Compresión: {stats['compression_ratio']:.2f}x")
    
    # Generación
    print("\n\n🤖 Generación de Texto:")
    print("-" * 40)
    
    prompts = ["Hello", "The AI", "Python", "Machine learning"]
    
    for prompt in prompts:
        start = time.time()
        response = ai.generate(prompt, max_tokens=20)
        elapsed = (time.time() - start) * 1000
        print(f"\nPrompt: '{prompt}'")
        print(f"Output: '{response[:60]}...'")
        print(f"Tiempo: {elapsed:.1f} ms")
    
    # Benchmark
    print("\n\n⚡ Benchmark:")
    print("-" * 40)
    bench = ai.benchmark(50)
    print(f"  Tiempo promedio: {bench['avg_time_ms']:.1f} ms")
    print(f"  Tokens/segundo:  {bench['tokens_per_sec']:.1f}")
    
    # Caché stats
    print("\n\n📊 Estadísticas de Caché:")
    print("-" * 40)
    cache = ai.embeddings.cache_stats()
    print(f"  Hit rate: {cache['hit_rate']:.1%}")
    print(f"  Cache size: {cache['cache_size']} tokens")
    
    print("\n" + "=" * 60)
    print(f"   ✅ Demo Completada")
    print(f"   💾 RAM Total: {ai.total_ram:.2f} MB")
    print(f"   📚 Vocabulario: {len(ai.tokenizer)} tokens")
    print("=" * 60)


if __name__ == "__main__":
    demo()
