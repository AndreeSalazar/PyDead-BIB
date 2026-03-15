"""
Tokenizador Inteligente para Metal-Dead
========================================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪
"""

import re
from typing import List, Dict
from collections import Counter


class SmartTokenizer:
    PAD, EOS, UNK, BOS, SEP = 0, 1, 2, 3, 4
    
    def __init__(self, vocab_size: int = 15000):
        self.vocab_size = vocab_size
        self.vocab: Dict[str, int] = {}
        self.inv_vocab: Dict[int, str] = {}
        self.word_freq: Counter = Counter()
        self._init_vocab()
    
    def _init_vocab(self):
        special = ["<PAD>", "<EOS>", "<UNK>", "<BOS>", "<SEP>", "<MASK>", "<USER>", "<AI>"]
        chars = list("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789")
        chars += list(".,!?;:'-\"()[]{}@#$%^&*+=<>/\\|`~_ \n\t")
        chars += list("áéíóúñüÁÉÍÓÚÑÜ¿¡")
        
        common_words = [
            "hola", "mundo", "gracias", "por", "favor", "bien", "mal", "si", "no",
            "que", "como", "cuando", "donde", "quien", "porque", "para", "con", "sin",
            "el", "la", "los", "las", "un", "una", "de", "en", "a", "es", "son",
            "yo", "tu", "el", "ella", "nosotros", "ustedes", "ellos", "mi", "su",
            "ayuda", "necesito", "quiero", "puedo", "debo", "tengo", "soy", "estoy",
            "hello", "world", "thanks", "please", "good", "bad", "yes", "no",
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
            "i", "you", "he", "she", "it", "we", "they", "my", "your", "his", "her",
            "ai", "ia", "python", "code", "data", "model", "train", "learn",
            "metal", "dead", "adead", "bib", "compiler", "binary", "opcode", "cpu", "gpu",
        ]
        
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
    
    def encode(self, text: str, add_special: bool = True) -> List[int]:
        tokens = []
        if add_special:
            tokens.append(self.BOS)
        
        parts = re.findall(r'\w+|[^\w\s]|\s+', text.lower())
        for part in parts:
            if part in self.vocab:
                tokens.append(self.vocab[part])
            else:
                for char in part:
                    tokens.append(self.vocab.get(char, self.UNK))
        
        if add_special:
            tokens.append(self.EOS)
        return tokens
    
    def decode(self, tokens: List[int], skip_special: bool = True) -> str:
        result = []
        special_ids = {self.PAD, self.EOS, self.BOS, self.SEP} if skip_special else set()
        
        for t in tokens:
            if t in special_ids:
                continue
            if t == self.EOS and skip_special:
                break
            result.append(self.inv_vocab.get(t, ""))
        return "".join(result)
    
    def __len__(self):
        return len(self.vocab)


# Alias para compatibilidad con CPU module
SimpleTokenizer = SmartTokenizer
