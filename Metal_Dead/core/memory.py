"""
Sistema de Memoria Persistente para Metal-Dead
===============================================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪
"""

import os
import json
import time
import hashlib
from pathlib import Path
from typing import List, Dict, Optional, Set
from dataclasses import dataclass
from collections import defaultdict

import numpy as np


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
    
    def __init__(self, data_dir: str, max_items: int = 1000):
        self.max_items = max_items
        self.memories: List[MemoryItem] = []
        self.categories: Set[str] = {"general", "personal", "facts", "preferences", "conversations"}
        self.memory_index: Dict[str, List[int]] = defaultdict(list)
        
        self.data_path = Path(data_dir)
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
        
        content_hash = hashlib.md5(content.lower().encode()).hexdigest()[:8]
        if content_hash in self.memory_index:
            for idx in self.memory_index[content_hash]:
                if idx < len(self.memories):
                    self.memories[idx].access_count += 1
                    self.memories[idx].importance = min(2.0, self.memories[idx].importance + 0.1)
            return self.memory_index[content_hash][0] if self.memory_index[content_hash] else -1
        
        idx = len(self.memories)
        self.memories.append(item)
        self.memory_index[content_hash].append(idx)
        
        if len(self.memories) > self.max_items:
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
            
            mem_words = set(mem.content.lower().split())
            overlap = len(query_words & mem_words)
            
            recency = 1.0 / (1.0 + (time.time() - mem.timestamp) / 86400)
            score = overlap * mem.importance * (1 + recency) * (1 + mem.access_count * 0.1)
            
            if score > 0:
                scored.append((score, mem))
        
        scored.sort(key=lambda x: x[0], reverse=True)
        return [mem for _, mem in scored[:top_k]]
    
    def get_recent(self, n: int = 5) -> List[MemoryItem]:
        return sorted(self.memories, key=lambda x: x.timestamp, reverse=True)[:n]
    
    def get_by_category(self, category: str) -> List[MemoryItem]:
        return [m for m in self.memories if m.category == category]
    
    def clear(self, category: Optional[str] = None):
        if category:
            self.memories = [m for m in self.memories if m.category != category]
        else:
            self.memories = []
        self._rebuild_index()
        self._save()
    
    def _cleanup(self):
        scored = []
        for i, mem in enumerate(self.memories):
            recency = 1.0 / (1.0 + (time.time() - mem.timestamp) / 86400)
            score = mem.importance * recency * (1 + mem.access_count * 0.1)
            scored.append((score, i, mem))
        
        scored.sort(key=lambda x: x[0], reverse=True)
        keep_indices = set(x[1] for x in scored[:self.max_items])
        
        self.memories = [m for i, m in enumerate(self.memories) if i in keep_indices]
        self._rebuild_index()
    
    def _rebuild_index(self):
        self.memory_index.clear()
        for i, mem in enumerate(self.memories):
            content_hash = hashlib.md5(mem.content.lower().encode()).hexdigest()[:8]
            self.memory_index[content_hash].append(i)
    
    def _save(self):
        data = {
            "memories": [m.to_dict() for m in self.memories],
            "categories": list(self.categories),
        }
        with open(self.memory_file, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
    
    def _load(self):
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
        return {
            "total_memories": len(self.memories),
            "categories": {cat: len(self.get_by_category(cat)) for cat in self.categories},
            "avg_importance": float(np.mean([m.importance for m in self.memories])) if self.memories else 0,
            "total_accesses": sum(m.access_count for m in self.memories),
        }


class Memory:
    """
    Memoria simple en RAM para Metal-Dead CPU.
    Sin persistencia, optimizada para velocidad.
    """
    
    def __init__(self):
        self.data: Dict = {}
    
    def get(self, key: str, default=None):
        """Obtiene valor de memoria."""
        return self.data.get(key, default)
    
    def set(self, key: str, value):
        """Establece valor en memoria."""
        self.data[key] = value
    
    def delete(self, key: str):
        """Elimina valor de memoria."""
        if key in self.data:
            del self.data[key]
    
    def clear(self):
        """Limpia toda la memoria."""
        self.data.clear()
    
    def has(self, key: str) -> bool:
        """Verifica si existe una clave."""
        return key in self.data
    
    def keys(self) -> List[str]:
        """Retorna todas las claves."""
        return list(self.data.keys())
    
    def __len__(self) -> int:
        return len(self.data)
