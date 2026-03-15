"""
Sistema de Vocabulario Escalable para ADead-BIB AI
===================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Permite crear, entrenar y guardar vocabularios de 10K-50K tokens.
"""

import os
import json
import re
from pathlib import Path
from typing import List, Dict, Tuple, Set
from collections import Counter
from dataclasses import dataclass


@dataclass
class VocabConfig:
    """Configuración del vocabulario."""
    vocab_size: int = 30000
    min_freq: int = 2
    num_merges: int = 25000
    save_path: str = "vocab.json"


class VocabularyBuilder:
    """
    Construye vocabularios grandes usando BPE.
    """
    
    # Tokens especiales
    SPECIAL_TOKENS = ["<PAD>", "<EOS>", "<UNK>", "<BOS>", "<SEP>", "<MASK>", "<CLS>"]
    
    def __init__(self, config: VocabConfig = None):
        self.config = config or VocabConfig()
        self.vocab: Dict[str, int] = {}
        self.inv_vocab: Dict[int, str] = {}
        self.merges: List[Tuple[str, str]] = []
        self.token_freqs: Counter = Counter()
        
        self._init_base()
    
    def _init_base(self):
        """Inicializa vocabulario base."""
        idx = 0
        
        # Tokens especiales
        for token in self.SPECIAL_TOKENS:
            self.vocab[token] = idx
            self.inv_vocab[idx] = token
            idx += 1
        
        # Caracteres ASCII
        for c in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ":
            self.vocab[c] = idx
            self.inv_vocab[idx] = c
            idx += 1
        
        # Dígitos
        for c in "0123456789":
            self.vocab[c] = idx
            self.inv_vocab[idx] = c
            idx += 1
        
        # Puntuación y símbolos
        for c in ".,!?;:'-\"()[]{}@#$%^&*+=<>/\\|`~_ \n\t":
            if c not in self.vocab:
                self.vocab[c] = idx
                self.inv_vocab[idx] = c
                idx += 1
        
        # Caracteres especiales español
        for c in "áéíóúñüÁÉÍÓÚÑÜ¿¡":
            self.vocab[c] = idx
            self.inv_vocab[idx] = c
            idx += 1
        
        self.base_size = idx
        print(f"Vocabulario base: {self.base_size} tokens")
    
    def train_from_texts(self, texts: List[str], verbose: bool = True):
        """
        Entrena BPE desde una lista de textos.
        """
        if verbose:
            print(f"Entrenando BPE con {len(texts)} textos...")
            print(f"Objetivo: {self.config.num_merges} fusiones")
        
        # Tokenizar a nivel de caracteres
        word_freqs: Counter = Counter()
        for text in texts:
            words = text.lower().split()
            for word in words:
                # Limpiar y agregar marcador de fin
                clean = ''.join(c for c in word if c.isalnum() or c in "'-")
                if clean:
                    word_chars = tuple(list(clean) + ["</w>"])
                    word_freqs[word_chars] += 1
        
        if verbose:
            print(f"Palabras únicas: {len(word_freqs)}")
        
        # Aprender fusiones BPE
        for i in range(self.config.num_merges):
            # Contar pares
            pair_freqs: Counter = Counter()
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
            self.merges.append(best_pair)
            
            # Aplicar fusión a todas las palabras
            new_word_freqs: Counter = Counter()
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
            
            if verbose and (i + 1) % 5000 == 0:
                print(f"  {i + 1}/{self.config.num_merges} fusiones ({len(self.vocab)} tokens)")
        
        if verbose:
            print(f"Vocabulario final: {len(self.vocab)} tokens")
    
    def train_from_file(self, filepath: str, max_lines: int = None):
        """Entrena desde un archivo de texto."""
        texts = []
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            for i, line in enumerate(f):
                if max_lines and i >= max_lines:
                    break
                line = line.strip()
                if line:
                    texts.append(line)
        
        self.train_from_texts(texts)
    
    def save(self, filepath: str = None):
        """Guarda vocabulario a JSON."""
        filepath = filepath or self.config.save_path
        
        data = {
            "vocab": self.vocab,
            "merges": [f"{a}|{b}" for a, b in self.merges],
            "config": {
                "vocab_size": self.config.vocab_size,
                "num_merges": self.config.num_merges,
            }
        }
        
        with open(filepath, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        
        print(f"Vocabulario guardado: {filepath} ({len(self.vocab)} tokens)")
    
    def load(self, filepath: str):
        """Carga vocabulario desde JSON."""
        with open(filepath, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        self.vocab = {k: int(v) for k, v in data["vocab"].items()}
        self.inv_vocab = {v: k for k, v in self.vocab.items()}
        self.merges = [tuple(m.split("|")) for m in data["merges"]]
        
        print(f"Vocabulario cargado: {len(self.vocab)} tokens")
    
    def __len__(self):
        return len(self.vocab)


class FastTokenizer:
    """
    Tokenizador rápido usando vocabulario pre-entrenado.
    """
    
    PAD = 0
    EOS = 1
    UNK = 2
    BOS = 3
    
    def __init__(self, vocab_path: str = None):
        self.vocab: Dict[str, int] = {}
        self.inv_vocab: Dict[int, str] = {}
        self.merges: List[Tuple[str, str]] = []
        
        if vocab_path and Path(vocab_path).exists():
            self.load(vocab_path)
        else:
            self._init_default()
    
    def _init_default(self):
        """Vocabulario por defecto."""
        special = ["<PAD>", "<EOS>", "<UNK>", "<BOS>"]
        for i, t in enumerate(special):
            self.vocab[t] = i
            self.inv_vocab[i] = t
    
    def load(self, filepath: str):
        """Carga vocabulario."""
        with open(filepath, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        self.vocab = {k: int(v) for k, v in data["vocab"].items()}
        self.inv_vocab = {v: k for k, v in self.vocab.items()}
        self.merges = [tuple(m.split("|")) for m in data.get("merges", [])]
        
        # Crear diccionario de fusiones para búsqueda rápida
        self.merge_dict = {pair: i for i, pair in enumerate(self.merges)}
    
    def _apply_bpe(self, word: str) -> List[str]:
        """Aplica BPE a una palabra."""
        if not word:
            return []
        
        tokens = list(word) + ["</w>"]
        
        # Aplicar fusiones en orden
        changed = True
        while changed and len(tokens) > 1:
            changed = False
            best_pair = None
            best_idx = len(self.merges)
            
            # Encontrar el par con menor índice (más frecuente)
            for i in range(len(tokens) - 1):
                pair = (tokens[i], tokens[i + 1])
                if pair in self.merge_dict:
                    idx = self.merge_dict[pair]
                    if idx < best_idx:
                        best_idx = idx
                        best_pair = pair
            
            if best_pair:
                # Aplicar fusión
                new_tokens = []
                i = 0
                while i < len(tokens):
                    if i < len(tokens) - 1 and (tokens[i], tokens[i + 1]) == best_pair:
                        new_tokens.append(tokens[i] + tokens[i + 1])
                        i += 2
                    else:
                        new_tokens.append(tokens[i])
                        i += 1
                tokens = new_tokens
                changed = True
        
        # Remover marcador
        if tokens and tokens[-1] == "</w>":
            tokens = tokens[:-1]
        
        return tokens
    
    def encode(self, text: str) -> List[int]:
        """Tokeniza texto a IDs."""
        tokens = [self.BOS]
        
        words = re.findall(r'\w+|[^\w\s]', text.lower())
        
        for word in words:
            subwords = self._apply_bpe(word)
            for sw in subwords:
                if sw in self.vocab:
                    tokens.append(self.vocab[sw])
                else:
                    # Fallback a caracteres
                    for c in sw:
                        tokens.append(self.vocab.get(c, self.UNK))
        
        tokens.append(self.EOS)
        return tokens
    
    def decode(self, tokens: List[int]) -> str:
        """Decodifica IDs a texto."""
        words = []
        current = []
        
        for t in tokens:
            if t in [self.PAD, self.BOS]:
                continue
            if t == self.EOS:
                break
            
            token = self.inv_vocab.get(t, "")
            if token.endswith("</w>"):
                current.append(token[:-4])
                words.append("".join(current))
                current = []
            elif token == " ":
                if current:
                    words.append("".join(current))
                    current = []
            else:
                current.append(token)
        
        if current:
            words.append("".join(current))
        
        return " ".join(words)
    
    def vocab_size(self) -> int:
        return len(self.vocab)


def generate_corpus() -> List[str]:
    """Genera corpus de entrenamiento para demo."""
    corpus = []
    
    # Textos en inglés
    en_texts = [
        "The quick brown fox jumps over the lazy dog",
        "Machine learning is transforming the world",
        "Artificial intelligence and deep learning",
        "Natural language processing with transformers",
        "Python programming for data science",
        "Neural networks learn from data",
        "Computer vision and image recognition",
        "Reinforcement learning for games",
        "Big data analytics and visualization",
        "Cloud computing and distributed systems",
    ]
    
    # Textos en español
    es_texts = [
        "La inteligencia artificial cambia el mundo",
        "Aprendizaje automático y redes neuronales",
        "Procesamiento de lenguaje natural",
        "Programación en Python para ciencia de datos",
        "Visión por computadora y reconocimiento",
        "Aprendizaje profundo con transformers",
        "Análisis de datos y visualización",
        "Computación en la nube distribuida",
        "Sistemas inteligentes y automatización",
        "Desarrollo de software moderno",
    ]
    
    # Expandir corpus
    for text in en_texts + es_texts:
        corpus.append(text)
        corpus.append(text.lower())
        words = text.split()
        # Generar n-gramas
        for i in range(len(words)):
            for j in range(i + 1, min(i + 5, len(words) + 1)):
                corpus.append(" ".join(words[i:j]))
    
    # Agregar palabras técnicas
    tech_words = [
        "algorithm", "function", "variable", "class", "method",
        "array", "list", "dictionary", "tuple", "set",
        "loop", "condition", "exception", "module", "package",
        "import", "export", "async", "await", "promise",
        "token", "embedding", "attention", "layer", "model",
        "training", "inference", "optimization", "gradient", "loss",
    ]
    
    for word in tech_words:
        corpus.append(word)
        corpus.append(f"the {word} is important")
        corpus.append(f"{word} processing")
    
    return corpus


def demo():
    """Demo del sistema de vocabulario."""
    print("=" * 60)
    print("   Demo: Sistema de Vocabulario Escalable")
    print("=" * 60)
    
    # Generar corpus
    print("\n📚 Generando corpus de entrenamiento...")
    corpus = generate_corpus()
    print(f"  Textos: {len(corpus)}")
    
    # Crear y entrenar vocabulario
    config = VocabConfig(
        vocab_size=10000,
        num_merges=5000,
        save_path="vocab_10k.json"
    )
    
    builder = VocabularyBuilder(config)
    builder.train_from_texts(corpus)
    
    # Guardar
    vocab_path = Path(__file__).parent / "vocab_10k.json"
    builder.save(str(vocab_path))
    
    # Probar tokenizador
    print("\n🔤 Probando tokenizador...")
    tokenizer = FastTokenizer(str(vocab_path))
    
    test_texts = [
        "Hello world",
        "Machine learning is amazing",
        "Inteligencia artificial",
        "Python programming",
    ]
    
    for text in test_texts:
        tokens = tokenizer.encode(text)
        decoded = tokenizer.decode(tokens)
        unk_count = tokens.count(tokenizer.UNK)
        print(f"\n  '{text}'")
        print(f"    Tokens: {len(tokens)}, UNK: {unk_count}")
        print(f"    IDs: {tokens[:10]}...")
    
    print("\n" + "=" * 60)
    print(f"   ✅ Vocabulario creado: {len(builder)} tokens")
    print(f"   📁 Guardado en: {vocab_path}")
    print("=" * 60)


if __name__ == "__main__":
    demo()
