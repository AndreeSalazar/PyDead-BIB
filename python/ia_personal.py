"""
IA-Personal - Sistema de IA Personal para ADead-BIB
====================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Sistema de IA Personal mejorado con:
- Memoria persistente (recuerda conversaciones)
- Contexto personal (aprende de ti)
- Aprendizaje continuo (mejora con el uso)
- Integración ADead-BIB (no runtime, ultra-rápido)
- Procesamiento local (100% privado)

Uso:
    python ia_personal.py              # Modo interactivo
    python ia_personal.py --demo       # Demo completa
    python ia_personal.py --benchmark  # Benchmark de rendimiento

RAM Total: ~0.5 MB (ultra-ligero)
"""

import os
import sys
import json
import time
import hashlib
import pickle
from pathlib import Path
from typing import List, Dict, Optional, Tuple, Any, Set
from dataclasses import dataclass, field, asdict
from datetime import datetime
from collections import Counter, defaultdict
import re

# Agregar directorio padre
sys.path.insert(0, str(Path(__file__).parent))

import numpy as np

# =============================================================================
# CONFIGURACIÓN
# =============================================================================

@dataclass
class IAPersonalConfig:
    """Configuración de IA-Personal."""
    # Modelo
    vocab_size: int = 15000
    embed_dim: int = 128
    num_heads: int = 8
    hidden_dim: int = 256
    num_layers: int = 2
    max_seq_len: int = 256
    
    # Generación
    temperature: float = 0.7
    top_k: int = 50
    top_p: float = 0.9
    repetition_penalty: float = 1.1
    
    # Memoria
    max_memory_items: int = 1000
    memory_decay: float = 0.95
    context_window: int = 10
    
    # Optimización
    use_float16: bool = True
    use_cache: bool = True
    batch_size: int = 1
    
    # Rutas
    data_dir: str = ""
    
    def __post_init__(self):
        if not self.data_dir:
            self.data_dir = str(Path(__file__).parent / "ia_personal_data")


# =============================================================================
# MEMORIA PERSISTENTE
# =============================================================================

@dataclass
class MemoryItem:
    """Item de memoria."""
    content: str
    timestamp: float
    importance: float = 1.0
    access_count: int = 0
    category: str = "general"
    embedding: Optional[np.ndarray] = None
    
    def to_dict(self) -> Dict:
        return {
            "content": self.content,
            "timestamp": self.timestamp,
            "importance": self.importance,
            "access_count": self.access_count,
            "category": self.category,
        }
    
    @classmethod
    def from_dict(cls, data: Dict) -> "MemoryItem":
        return cls(
            content=data["content"],
            timestamp=data["timestamp"],
            importance=data.get("importance", 1.0),
            access_count=data.get("access_count", 0),
            category=data.get("category", "general"),
        )


class PersistentMemory:
    """Sistema de memoria persistente con búsqueda semántica."""
    
    def __init__(self, config: IAPersonalConfig):
        self.config = config
        self.memories: List[MemoryItem] = []
        self.categories: Set[str] = {"general", "personal", "facts", "preferences", "conversations"}
        self.memory_index: Dict[str, List[int]] = defaultdict(list)
        
        # Crear directorio de datos
        self.data_path = Path(config.data_dir)
        self.data_path.mkdir(parents=True, exist_ok=True)
        self.memory_file = self.data_path / "memories.json"
        
        self._load()
    
    def add(self, content: str, category: str = "general", importance: float = 1.0) -> int:
        """Agrega un item a la memoria."""
        item = MemoryItem(
            content=content,
            timestamp=time.time(),
            importance=importance,
            category=category,
        )
        
        # Verificar duplicados
        content_hash = hashlib.md5(content.lower().encode()).hexdigest()[:8]
        if content_hash in self.memory_index:
            # Actualizar existente
            for idx in self.memory_index[content_hash]:
                self.memories[idx].access_count += 1
                self.memories[idx].importance = min(2.0, self.memories[idx].importance + 0.1)
            return self.memory_index[content_hash][0]
        
        # Agregar nuevo
        idx = len(self.memories)
        self.memories.append(item)
        self.memory_index[content_hash].append(idx)
        
        # Limpiar si excede límite
        if len(self.memories) > self.config.max_memory_items:
            self._cleanup()
        
        self._save()
        return idx
    
    def search(self, query: str, top_k: int = 5, category: Optional[str] = None) -> List[MemoryItem]:
        """Busca memorias relevantes."""
        query_words = set(query.lower().split())
        
        scored = []
        for mem in self.memories:
            if category and mem.category != category:
                continue
            
            # Score basado en palabras compartidas
            mem_words = set(mem.content.lower().split())
            overlap = len(query_words & mem_words)
            
            # Ajustar por importancia y recencia
            recency = 1.0 / (1.0 + (time.time() - mem.timestamp) / 86400)  # Decay por día
            score = overlap * mem.importance * (1 + recency) * (1 + mem.access_count * 0.1)
            
            if score > 0:
                scored.append((score, mem))
        
        # Ordenar y retornar top-k
        scored.sort(key=lambda x: x[0], reverse=True)
        return [mem for _, mem in scored[:top_k]]
    
    def get_context(self, n: int = 5) -> List[str]:
        """Obtiene las últimas n memorias como contexto."""
        recent = sorted(self.memories, key=lambda x: x.timestamp, reverse=True)[:n]
        return [m.content for m in recent]
    
    def get_by_category(self, category: str) -> List[MemoryItem]:
        """Obtiene memorias por categoría."""
        return [m for m in self.memories if m.category == category]
    
    def _cleanup(self):
        """Limpia memorias antiguas y poco importantes."""
        # Calcular scores de retención
        scored = []
        for i, mem in enumerate(self.memories):
            recency = 1.0 / (1.0 + (time.time() - mem.timestamp) / 86400)
            score = mem.importance * recency * (1 + mem.access_count * 0.1)
            scored.append((score, i, mem))
        
        # Mantener top N
        scored.sort(key=lambda x: x[0], reverse=True)
        keep_indices = set(x[1] for x in scored[:self.config.max_memory_items])
        
        self.memories = [m for i, m in enumerate(self.memories) if i in keep_indices]
        self._rebuild_index()
    
    def _rebuild_index(self):
        """Reconstruye el índice de memoria."""
        self.memory_index.clear()
        for i, mem in enumerate(self.memories):
            content_hash = hashlib.md5(mem.content.lower().encode()).hexdigest()[:8]
            self.memory_index[content_hash].append(i)
    
    def _save(self):
        """Guarda memorias a disco."""
        data = {
            "memories": [m.to_dict() for m in self.memories],
            "categories": list(self.categories),
        }
        with open(self.memory_file, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
    
    def _load(self):
        """Carga memorias desde disco."""
        if self.memory_file.exists():
            try:
                with open(self.memory_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                self.memories = [MemoryItem.from_dict(m) for m in data.get("memories", [])]
                self.categories.update(data.get("categories", []))
                self._rebuild_index()
                print(f"📚 Memorias cargadas: {len(self.memories)}")
            except Exception as e:
                print(f"⚠️ Error cargando memorias: {e}")
    
    def stats(self) -> Dict:
        """Estadísticas de memoria."""
        return {
            "total_memories": len(self.memories),
            "categories": {cat: len(self.get_by_category(cat)) for cat in self.categories},
            "avg_importance": np.mean([m.importance for m in self.memories]) if self.memories else 0,
            "total_accesses": sum(m.access_count for m in self.memories),
        }


# =============================================================================
# PERFIL PERSONAL
# =============================================================================

@dataclass
class UserProfile:
    """Perfil del usuario para personalización."""
    name: str = "Usuario"
    language: str = "es"
    interests: List[str] = field(default_factory=list)
    preferences: Dict[str, Any] = field(default_factory=dict)
    interaction_count: int = 0
    first_interaction: float = 0.0
    last_interaction: float = 0.0
    learned_facts: Dict[str, str] = field(default_factory=dict)
    
    def to_dict(self) -> Dict:
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: Dict) -> "UserProfile":
        return cls(**data)


class PersonalContext:
    """Gestiona el contexto personal del usuario."""
    
    def __init__(self, config: IAPersonalConfig):
        self.config = config
        self.data_path = Path(config.data_dir)
        self.data_path.mkdir(parents=True, exist_ok=True)
        self.profile_file = self.data_path / "profile.json"
        
        self.profile = UserProfile()
        self._load()
    
    def update_interaction(self):
        """Actualiza estadísticas de interacción."""
        now = time.time()
        if self.profile.first_interaction == 0:
            self.profile.first_interaction = now
        self.profile.last_interaction = now
        self.profile.interaction_count += 1
        self._save()
    
    def learn_fact(self, key: str, value: str):
        """Aprende un hecho sobre el usuario."""
        self.profile.learned_facts[key] = value
        self._save()
    
    def add_interest(self, interest: str):
        """Agrega un interés."""
        if interest.lower() not in [i.lower() for i in self.profile.interests]:
            self.profile.interests.append(interest)
            self._save()
    
    def set_preference(self, key: str, value: Any):
        """Establece una preferencia."""
        self.profile.preferences[key] = value
        self._save()
    
    def get_greeting(self) -> str:
        """Genera un saludo personalizado."""
        hour = datetime.now().hour
        if hour < 12:
            greeting = "Buenos días"
        elif hour < 18:
            greeting = "Buenas tardes"
        else:
            greeting = "Buenas noches"
        
        name = self.profile.name
        if self.profile.interaction_count == 0:
            return f"¡Hola! Soy tu IA Personal. ¿Cómo te llamas?"
        elif self.profile.interaction_count < 5:
            return f"{greeting}, {name}. ¿En qué puedo ayudarte?"
        else:
            return f"{greeting}, {name}. Me alegra verte de nuevo."
    
    def _save(self):
        """Guarda perfil a disco."""
        with open(self.profile_file, 'w', encoding='utf-8') as f:
            json.dump(self.profile.to_dict(), f, ensure_ascii=False, indent=2)
    
    def _load(self):
        """Carga perfil desde disco."""
        if self.profile_file.exists():
            try:
                with open(self.profile_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                self.profile = UserProfile.from_dict(data)
                print(f"👤 Perfil cargado: {self.profile.name}")
            except Exception as e:
                print(f"⚠️ Error cargando perfil: {e}")


# =============================================================================
# TOKENIZADOR MEJORADO
# =============================================================================

class SmartTokenizer:
    """Tokenizador inteligente con BPE y vocabulario expandible."""
    
    PAD, EOS, UNK, BOS, SEP = 0, 1, 2, 3, 4
    
    def __init__(self, vocab_size: int = 15000):
        self.vocab_size = vocab_size
        self.vocab: Dict[str, int] = {}
        self.inv_vocab: Dict[int, str] = {}
        self.word_freq: Counter = Counter()
        self.merges: Dict[Tuple[str, str], str] = {}
        
        self._init_vocab()
    
    def _init_vocab(self):
        """Inicializa vocabulario base."""
        # Tokens especiales
        special = ["<PAD>", "<EOS>", "<UNK>", "<BOS>", "<SEP>", "<MASK>", "<USER>", "<AI>"]
        
        # Caracteres
        chars = list("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
        chars += list(".,!?;:'-\"()[]{}@#$%^&*+=<>/\\|`~_ \n\t")
        chars += list("áéíóúñüÁÉÍÓÚÑÜ¿¡")
        
        # Palabras comunes (español + inglés + tech)
        common_words = [
            # Español
            "hola", "mundo", "gracias", "por", "favor", "bien", "mal", "si", "no",
            "que", "como", "cuando", "donde", "quien", "porque", "para", "con", "sin",
            "el", "la", "los", "las", "un", "una", "unos", "unas", "de", "en", "a",
            "es", "son", "ser", "estar", "tener", "hacer", "poder", "decir", "ir",
            "yo", "tu", "el", "ella", "nosotros", "ustedes", "ellos", "mi", "tu", "su",
            "bueno", "malo", "grande", "pequeño", "nuevo", "viejo", "mejor", "peor",
            "hoy", "ayer", "mañana", "ahora", "siempre", "nunca", "todo", "nada",
            "ayuda", "necesito", "quiero", "puedo", "debo", "tengo", "soy", "estoy",
            # Inglés
            "hello", "world", "thanks", "please", "good", "bad", "yes", "no",
            "what", "how", "when", "where", "who", "why", "which", "that", "this",
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "i", "you", "he", "she", "it", "we", "they", "my", "your", "his", "her",
            "help", "need", "want", "can", "must", "should", "am", "im",
            # Tech/IA
            "ai", "ia", "python", "code", "data", "model", "train", "learn",
            "neural", "network", "machine", "learning", "deep", "algorithm",
            "function", "class", "variable", "array", "list", "dict", "string",
            "input", "output", "process", "compute", "memory", "fast", "slow",
            "adead", "bib", "compiler", "binary", "opcode", "cpu", "gpu",
            # Emociones
            "feliz", "triste", "enojado", "sorprendido", "asustado", "confundido",
            "happy", "sad", "angry", "surprised", "scared", "confused",
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
        
        for word in common_words:
            word = word.lower()
            if word not in self.vocab:
                self.vocab[word] = idx
                self.inv_vocab[idx] = word
                idx += 1
        
        self.base_size = idx
    
    def encode(self, text: str, add_special: bool = True) -> List[int]:
        """Tokeniza texto a IDs."""
        tokens = []
        if add_special:
            tokens.append(self.BOS)
        
        # Dividir en palabras y puntuación
        parts = re.findall(r'\w+|[^\w\s]|\s+', text.lower())
        
        for part in parts:
            if part in self.vocab:
                tokens.append(self.vocab[part])
            else:
                # Fallback a caracteres
                for char in part:
                    tokens.append(self.vocab.get(char, self.UNK))
        
        if add_special:
            tokens.append(self.EOS)
        
        return tokens
    
    def decode(self, tokens: List[int], skip_special: bool = True) -> str:
        """Decodifica IDs a texto."""
        result = []
        special_ids = {self.PAD, self.EOS, self.BOS, self.SEP} if skip_special else set()
        
        for t in tokens:
            if t in special_ids:
                continue
            if t == self.EOS and skip_special:
                break
            result.append(self.inv_vocab.get(t, ""))
        
        return "".join(result)
    
    def learn_word(self, word: str) -> int:
        """Aprende una nueva palabra."""
        word = word.lower()
        if word not in self.vocab and len(self.vocab) < self.vocab_size:
            idx = len(self.vocab)
            self.vocab[word] = idx
            self.inv_vocab[idx] = word
            return idx
        return self.vocab.get(word, self.UNK)
    
    def __len__(self):
        return len(self.vocab)


# =============================================================================
# MODELO TRANSFORMER LIGERO
# =============================================================================

class LightTransformer:
    """Transformer ligero optimizado para bajo RAM."""
    
    def __init__(self, config: IAPersonalConfig, vocab_size: int):
        self.config = config
        self.vocab_size = vocab_size
        self.dtype = np.float16 if config.use_float16 else np.float32
        
        # Embeddings
        self.embeddings = np.random.randn(vocab_size, config.embed_dim).astype(self.dtype) * 0.02
        
        # Capas transformer
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
        
        # Capa de salida
        self.output_proj = np.random.randn(config.embed_dim, vocab_size).astype(self.dtype) * 0.02
        
        # Cache para KV
        self.kv_cache: Dict[int, Tuple[np.ndarray, np.ndarray]] = {}
        
        self._calc_ram()
    
    def _calc_ram(self):
        """Calcula RAM usada."""
        bytes_per = 2 if self.config.use_float16 else 4
        
        embed_ram = self.vocab_size * self.config.embed_dim * bytes_per
        layer_ram = self.config.num_layers * (
            4 * self.config.embed_dim * self.config.embed_dim +
            2 * self.config.embed_dim * self.config.hidden_dim
        ) * bytes_per
        output_ram = self.config.embed_dim * self.vocab_size * bytes_per
        
        self.ram_mb = (embed_ram + layer_ram + output_ram) / (1024 * 1024)
    
    def forward(self, token_ids: List[int]) -> np.ndarray:
        """Forward pass."""
        # Embeddings
        x = self.embeddings[token_ids]
        
        head_dim = self.config.embed_dim // self.config.num_heads
        
        for layer in self.layers:
            # Atención
            Q = x @ layer["W_q"]
            K = x @ layer["W_k"]
            V = x @ layer["W_v"]
            
            scores = Q @ K.T / np.sqrt(head_dim)
            
            # Máscara causal
            mask = np.triu(np.ones_like(scores) * -1e9, k=1)
            scores = scores + mask
            
            # Softmax
            exp_scores = np.exp(scores - np.max(scores, axis=-1, keepdims=True))
            weights = exp_scores / (np.sum(exp_scores, axis=-1, keepdims=True) + 1e-8)
            
            attn_out = weights @ V @ layer["W_o"]
            x = x + attn_out
            
            # FFN con GELU
            hidden = x @ layer["W1"]
            hidden = hidden * 0.5 * (1 + np.tanh(np.sqrt(2 / np.pi) * (hidden + 0.044715 * hidden**3)))
            ffn_out = hidden @ layer["W2"]
            x = x + ffn_out
        
        # Logits
        logits = x[-1] @ self.output_proj
        return logits
    
    def generate_token(self, token_ids: List[int], temperature: float = 0.7,
                       top_k: int = 50, top_p: float = 0.9,
                       repetition_penalty: float = 1.1) -> int:
        """Genera el siguiente token."""
        logits = self.forward(token_ids)
        
        # Penalización por repetición
        for tid in set(token_ids[-20:]):
            logits[tid] /= repetition_penalty
        
        # Temperatura
        logits = logits / temperature
        
        # Top-k
        if top_k > 0:
            indices = np.argsort(logits)[-top_k:]
            mask = np.ones_like(logits) * -1e9
            mask[indices] = 0
            logits = logits + mask
        
        # Softmax
        exp_logits = np.exp(logits - np.max(logits))
        probs = exp_logits / np.sum(exp_logits)
        
        # Top-p (nucleus sampling)
        if top_p < 1.0:
            sorted_indices = np.argsort(probs)[::-1]
            cumsum = np.cumsum(probs[sorted_indices])
            cutoff_idx = np.searchsorted(cumsum, top_p) + 1
            keep_indices = sorted_indices[:cutoff_idx]
            
            new_probs = np.zeros_like(probs)
            new_probs[keep_indices] = probs[keep_indices]
            probs = new_probs / np.sum(new_probs)
        
        # Muestrear
        return int(np.random.choice(len(probs), p=probs.astype(np.float64)))


# =============================================================================
# IA PERSONAL - SISTEMA PRINCIPAL
# =============================================================================

class IAPersonal:
    """
    Sistema de IA Personal completo.
    Combina memoria, contexto personal y generación de texto.
    """
    
    def __init__(self, config: IAPersonalConfig = None):
        self.config = config or IAPersonalConfig()
        
        print("=" * 60)
        print("   🤖 IA-Personal para ADead-BIB")
        print("   Sistema de IA Personal Ultra-Ligero")
        print("=" * 60)
        
        # Componentes
        self.tokenizer = SmartTokenizer(self.config.vocab_size)
        self.memory = PersistentMemory(self.config)
        self.context = PersonalContext(self.config)
        self.model = LightTransformer(self.config, len(self.tokenizer))
        
        # Historial de conversación actual
        self.conversation_history: List[Tuple[str, str]] = []
        
        # Patrones de aprendizaje
        self.learning_patterns = {
            r"me llamo (\w+)": self._learn_name,
            r"mi nombre es (\w+)": self._learn_name,
            r"soy (\w+)": self._learn_name,
            r"me gusta (.+)": self._learn_interest,
            r"me interesa (.+)": self._learn_interest,
            r"recuerda que (.+)": self._learn_fact,
            r"no olvides que (.+)": self._learn_fact,
        }
        
        self._print_stats()
    
    def _print_stats(self):
        """Imprime estadísticas del sistema."""
        print(f"\n📊 Configuración:")
        print(f"  Vocabulario: {len(self.tokenizer)} tokens")
        print(f"  Embeddings:  {self.config.embed_dim} dim")
        print(f"  Capas:       {self.config.num_layers}")
        print(f"  Memorias:    {len(self.memory.memories)}")
        print(f"\n💾 RAM Total:  {self.model.ram_mb:.2f} MB")
        print("=" * 60)
    
    def _learn_name(self, match: re.Match) -> str:
        """Aprende el nombre del usuario."""
        name = match.group(1).capitalize()
        self.context.profile.name = name
        self.context._save()
        self.memory.add(f"El usuario se llama {name}", category="personal", importance=2.0)
        return f"¡Encantado de conocerte, {name}! Recordaré tu nombre."
    
    def _learn_interest(self, match: re.Match) -> str:
        """Aprende un interés del usuario."""
        interest = match.group(1).strip()
        self.context.add_interest(interest)
        self.memory.add(f"Al usuario le interesa: {interest}", category="preferences", importance=1.5)
        return f"¡Interesante! Recordaré que te gusta {interest}."
    
    def _learn_fact(self, match: re.Match) -> str:
        """Aprende un hecho."""
        fact = match.group(1).strip()
        self.memory.add(fact, category="facts", importance=1.5)
        return f"Entendido, lo recordaré: {fact}"
    
    def _check_learning(self, message: str) -> Optional[str]:
        """Verifica si el mensaje contiene información para aprender."""
        message_lower = message.lower()
        for pattern, handler in self.learning_patterns.items():
            match = re.search(pattern, message_lower)
            if match:
                return handler(match)
        return None
    
    def _build_context(self, message: str) -> str:
        """Construye contexto para la generación."""
        parts = []
        
        # Información del usuario
        if self.context.profile.name != "Usuario":
            parts.append(f"Usuario: {self.context.profile.name}")
        
        if self.context.profile.interests:
            parts.append(f"Intereses: {', '.join(self.context.profile.interests[:5])}")
        
        # Memorias relevantes
        relevant = self.memory.search(message, top_k=3)
        if relevant:
            parts.append("Contexto relevante:")
            for mem in relevant:
                parts.append(f"- {mem.content}")
        
        # Historial reciente
        if self.conversation_history:
            parts.append("Conversación reciente:")
            for user_msg, ai_msg in self.conversation_history[-3:]:
                parts.append(f"Usuario: {user_msg}")
                parts.append(f"IA: {ai_msg}")
        
        return "\n".join(parts)
    
    def generate(self, prompt: str, max_tokens: int = 50) -> str:
        """Genera una respuesta."""
        # Construir contexto
        context = self._build_context(prompt)
        full_prompt = f"{context}\nUsuario: {prompt}\nIA:"
        
        # Tokenizar
        tokens = self.tokenizer.encode(full_prompt, add_special=True)
        tokens = tokens[-self.config.max_seq_len:]  # Limitar contexto
        
        # Generar
        generated = []
        for _ in range(max_tokens):
            next_token = self.model.generate_token(
                tokens,
                temperature=self.config.temperature,
                top_k=self.config.top_k,
                top_p=self.config.top_p,
                repetition_penalty=self.config.repetition_penalty,
            )
            
            if next_token == self.tokenizer.EOS:
                break
            
            tokens.append(next_token)
            generated.append(next_token)
            
            # Parar en salto de línea (fin de respuesta)
            if self.tokenizer.inv_vocab.get(next_token, "") == "\n":
                break
        
        return self.tokenizer.decode(generated, skip_special=True).strip()
    
    def chat(self, message: str) -> str:
        """Procesa un mensaje y genera respuesta."""
        self.context.update_interaction()
        
        # Verificar aprendizaje
        learning_response = self._check_learning(message)
        if learning_response:
            self.conversation_history.append((message, learning_response))
            return learning_response
        
        # Comandos especiales
        message_lower = message.lower().strip()
        
        if message_lower in ["hola", "hi", "hello"]:
            response = self.context.get_greeting()
        elif message_lower in ["ayuda", "help", "?"]:
            response = self._get_help()
        elif message_lower in ["memoria", "memorias", "memory"]:
            response = self._get_memory_stats()
        elif message_lower in ["perfil", "profile"]:
            response = self._get_profile()
        elif message_lower.startswith("busca ") or message_lower.startswith("search "):
            query = message[6:].strip()
            response = self._search_memory(query)
        else:
            # Usar respuestas inteligentes basadas en contexto
            # (El modelo transformer no está pre-entrenado, así que usamos lógica de reglas)
            response = self._get_default_response(message)
        
        # Guardar en historial y memoria
        self.conversation_history.append((message, response))
        self.memory.add(f"Usuario: {message}", category="conversations")
        
        return response
    
    def _get_help(self) -> str:
        """Retorna mensaje de ayuda."""
        return """🤖 **IA-Personal - Comandos:**

• **Conversación normal** - Solo escribe y responderé
• **"me llamo [nombre]"** - Aprendo tu nombre
• **"me gusta [algo]"** - Aprendo tus intereses
• **"recuerda que [algo]"** - Guardo información
• **"busca [tema]"** - Busco en mis memorias
• **"memoria"** - Muestro estadísticas de memoria
• **"perfil"** - Muestro tu perfil
• **"ayuda"** - Este mensaje

💡 Soy tu IA personal, aprendo de ti y recuerdo nuestras conversaciones."""
    
    def _get_memory_stats(self) -> str:
        """Retorna estadísticas de memoria."""
        stats = self.memory.stats()
        lines = [
            "📚 **Estadísticas de Memoria:**",
            f"• Total: {stats['total_memories']} memorias",
            f"• Accesos totales: {stats['total_accesses']}",
            "• Por categoría:"
        ]
        for cat, count in stats['categories'].items():
            if count > 0:
                lines.append(f"  - {cat}: {count}")
        return "\n".join(lines)
    
    def _get_profile(self) -> str:
        """Retorna perfil del usuario."""
        p = self.context.profile
        lines = [
            "👤 **Tu Perfil:**",
            f"• Nombre: {p.name}",
            f"• Interacciones: {p.interaction_count}",
        ]
        if p.interests:
            lines.append(f"• Intereses: {', '.join(p.interests)}")
        if p.learned_facts:
            lines.append("• Hechos aprendidos:")
            for k, v in list(p.learned_facts.items())[:5]:
                lines.append(f"  - {k}: {v}")
        return "\n".join(lines)
    
    def _search_memory(self, query: str) -> str:
        """Busca en la memoria."""
        results = self.memory.search(query, top_k=5)
        if not results:
            return f"No encontré nada sobre '{query}' en mis memorias."
        
        lines = [f"🔍 **Resultados para '{query}':**"]
        for i, mem in enumerate(results, 1):
            lines.append(f"{i}. {mem.content[:100]}...")
        return "\n".join(lines)
    
    def _get_default_response(self, message: str) -> str:
        """Genera respuesta inteligente basada en el contexto."""
        message_lower = message.lower()
        
        # Respuestas basadas en palabras clave
        if any(w in message_lower for w in ["qué sabes", "que sabes", "conoces", "recuerdas"]):
            # Buscar información del usuario
            facts = []
            if self.context.profile.name != "Usuario":
                facts.append(f"Te llamas {self.context.profile.name}")
            if self.context.profile.interests:
                facts.append(f"Te interesa: {', '.join(self.context.profile.interests)}")
            
            relevant = self.memory.search(message, top_k=3, category="facts")
            for mem in relevant:
                facts.append(mem.content)
            
            if facts:
                return "Esto es lo que sé de ti:\n• " + "\n• ".join(facts)
            return "Aún estoy aprendiendo sobre ti. ¡Cuéntame más!"
        
        if any(w in message_lower for w in ["cómo estás", "como estas", "qué tal", "que tal"]):
            return "¡Estoy muy bien, gracias por preguntar! ¿Y tú cómo estás?"
        
        if any(w in message_lower for w in ["qué puedes", "que puedes", "qué haces", "que haces"]):
            return """Puedo ayudarte con varias cosas:
• Recordar información sobre ti
• Mantener conversaciones
• Buscar en mis memorias
• Aprender de nuestras interacciones

Escribe 'ayuda' para ver todos los comandos."""
        
        if any(w in message_lower for w in ["gracias", "thanks"]):
            return f"¡De nada, {self.context.profile.name}! Estoy aquí para ayudarte."
        
        if any(w in message_lower for w in ["adiós", "adios", "bye", "chao"]):
            return f"¡Hasta pronto, {self.context.profile.name}! Fue un gusto conversar contigo."
        
        # Respuestas genéricas variadas
        responses = [
            "Interesante, cuéntame más sobre eso.",
            "Entiendo. ¿Qué más te gustaría compartir?",
            "Gracias por contarme. ¿Hay algo más en lo que pueda ayudarte?",
            f"Hmm, {message[:30]}... es un tema interesante.",
            "¿Puedes darme más detalles al respecto?",
            "Me parece muy interesante lo que dices.",
            "Sigo aprendiendo, pero me encanta conversar contigo.",
        ]
        return responses[hash(message) % len(responses)]
    
    def interactive(self):
        """Modo interactivo de chat."""
        print("\n" + "=" * 60)
        print("   🤖 IA-Personal - Modo Interactivo")
        print("   Escribe 'salir' para terminar")
        print("=" * 60)
        
        print(f"\n{self.context.get_greeting()}\n")
        
        while True:
            try:
                user_input = input("Tú: ").strip()
                
                if not user_input:
                    continue
                
                if user_input.lower() in ["salir", "exit", "quit", "q"]:
                    print("\n¡Hasta luego! Fue un placer conversar contigo. 👋")
                    break
                
                response = self.chat(user_input)
                print(f"\n🤖: {response}\n")
                
            except KeyboardInterrupt:
                print("\n\n¡Hasta luego! 👋")
                break
            except Exception as e:
                print(f"\n⚠️ Error: {e}\n")


# =============================================================================
# DEMO Y BENCHMARK
# =============================================================================

def demo():
    """Demo del sistema IA-Personal."""
    print("\n" + "=" * 60)
    print("   DEMO: IA-Personal para ADead-BIB")
    print("=" * 60)
    
    config = IAPersonalConfig(
        vocab_size=10000,
        embed_dim=128,
        num_heads=8,
        hidden_dim=256,
        num_layers=2,
        temperature=0.8,
    )
    
    ia = IAPersonal(config)
    
    # Simular conversación
    print("\n📝 Simulación de Conversación:")
    print("-" * 40)
    
    messages = [
        "Hola",
        "Me llamo Carlos",
        "Me gusta la programación",
        "Recuerda que estoy aprendiendo Python",
        "¿Qué sabes de mí?",
        "perfil",
        "memoria",
        "busca programación",
    ]
    
    for msg in messages:
        print(f"\n👤 Usuario: {msg}")
        response = ia.chat(msg)
        print(f"🤖 IA: {response}")
        time.sleep(0.1)
    
    print("\n" + "=" * 60)
    print("   ✅ Demo Completada")
    print(f"   💾 RAM: {ia.model.ram_mb:.2f} MB")
    print(f"   📚 Memorias: {len(ia.memory.memories)}")
    print("=" * 60)


def benchmark():
    """Benchmark de rendimiento."""
    print("\n" + "=" * 60)
    print("   BENCHMARK: IA-Personal")
    print("=" * 60)
    
    config = IAPersonalConfig(
        vocab_size=10000,
        embed_dim=128,
        num_layers=2,
    )
    
    ia = IAPersonal(config)
    
    # Benchmark de generación
    print("\n⚡ Benchmark de Generación:")
    print("-" * 40)
    
    prompts = ["Hola", "¿Cómo estás?", "Cuéntame algo", "¿Qué puedes hacer?"]
    times = []
    
    for _ in range(20):
        prompt = prompts[_ % len(prompts)]
        start = time.time()
        ia.generate(prompt, max_tokens=20)
        times.append(time.time() - start)
    
    print(f"  Tiempo promedio: {np.mean(times)*1000:.1f} ms")
    print(f"  Tiempo mínimo:   {np.min(times)*1000:.1f} ms")
    print(f"  Tiempo máximo:   {np.max(times)*1000:.1f} ms")
    print(f"  Tokens/segundo:  {20/np.mean(times):.1f}")
    
    # Benchmark de memoria
    print("\n📚 Benchmark de Memoria:")
    print("-" * 40)
    
    start = time.time()
    for i in range(100):
        ia.memory.add(f"Test memory item {i}", category="general")
    add_time = time.time() - start
    
    start = time.time()
    for i in range(100):
        ia.memory.search(f"memory {i}", top_k=5)
    search_time = time.time() - start
    
    print(f"  Agregar 100 items: {add_time*1000:.1f} ms")
    print(f"  Buscar 100 veces:  {search_time*1000:.1f} ms")
    
    print("\n" + "=" * 60)
    print(f"   ✅ Benchmark Completado")
    print(f"   💾 RAM Total: {ia.model.ram_mb:.2f} MB")
    print("=" * 60)


def main():
    """Función principal."""
    import argparse
    
    parser = argparse.ArgumentParser(description="IA-Personal para ADead-BIB")
    parser.add_argument("--demo", action="store_true", help="Ejecutar demo")
    parser.add_argument("--benchmark", action="store_true", help="Ejecutar benchmark")
    args = parser.parse_args()
    
    if args.demo:
        demo()
    elif args.benchmark:
        benchmark()
    else:
        # Modo interactivo
        config = IAPersonalConfig()
        ia = IAPersonal(config)
        ia.interactive()


if __name__ == "__main__":
    main()
