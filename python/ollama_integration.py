"""
Integración con Ollama para ADead-BIB AI
=========================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Permite usar modelos de lenguaje reales con ADead-BIB.

Requisitos:
    1. Instalar Ollama: winget install Ollama.Ollama
    2. Descargar modelo: ollama pull tinyllama
    3. pip install ollama (opcional, usamos HTTP directo)
"""

import os
import sys
import json
import time
import subprocess
from pathlib import Path
from typing import Dict, List, Optional
from dataclasses import dataclass

# Para HTTP requests
try:
    import urllib.request
    import urllib.error
    HAS_URLLIB = True
except ImportError:
    HAS_URLLIB = False

sys.path.insert(0, str(Path(__file__).parent))

from adead_ffi import ADeadBIB


@dataclass
class OllamaConfig:
    """Configuración de Ollama."""
    model: str = "tinyllama"
    host: str = "http://localhost:11434"
    timeout: int = 30
    max_tokens: int = 100
    temperature: float = 0.7


class OllamaClient:
    """
    Cliente HTTP simple para Ollama.
    No requiere la librería ollama.
    """
    
    def __init__(self, config: OllamaConfig = None):
        self.config = config or OllamaConfig()
        self.available = False
        self._check_availability()
    
    def _check_availability(self):
        """Verifica si Ollama está disponible."""
        try:
            url = f"{self.config.host}/api/tags"
            req = urllib.request.Request(url, method='GET')
            with urllib.request.urlopen(req, timeout=5) as response:
                data = json.loads(response.read().decode())
                models = [m["name"] for m in data.get("models", [])]
                self.available = True
                print(f"✅ Ollama disponible. Modelos: {models[:3]}...")
                return True
        except Exception as e:
            print(f"⚠️ Ollama no disponible: {e}")
            self.available = False
            return False
    
    def generate(self, prompt: str, system: str = None) -> str:
        """
        Genera texto usando Ollama.
        """
        if not self.available:
            return "[Ollama no disponible]"
        
        url = f"{self.config.host}/api/generate"
        
        payload = {
            "model": self.config.model,
            "prompt": prompt,
            "stream": False,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens,
            }
        }
        
        if system:
            payload["system"] = system
        
        try:
            data = json.dumps(payload).encode('utf-8')
            req = urllib.request.Request(
                url,
                data=data,
                headers={'Content-Type': 'application/json'},
                method='POST'
            )
            
            with urllib.request.urlopen(req, timeout=self.config.timeout) as response:
                result = json.loads(response.read().decode())
                return result.get("response", "")
        
        except urllib.error.URLError as e:
            return f"[Error de conexión: {e}]"
        except Exception as e:
            return f"[Error: {e}]"
    
    def chat(self, messages: List[Dict[str, str]]) -> str:
        """
        Chat con historial de mensajes.
        """
        if not self.available:
            return "[Ollama no disponible]"
        
        url = f"{self.config.host}/api/chat"
        
        payload = {
            "model": self.config.model,
            "messages": messages,
            "stream": False,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens,
            }
        }
        
        try:
            data = json.dumps(payload).encode('utf-8')
            req = urllib.request.Request(
                url,
                data=data,
                headers={'Content-Type': 'application/json'},
                method='POST'
            )
            
            with urllib.request.urlopen(req, timeout=self.config.timeout) as response:
                result = json.loads(response.read().decode())
                return result.get("message", {}).get("content", "")
        
        except Exception as e:
            return f"[Error: {e}]"


class HybridAI:
    """
    IA Híbrida: ADead-BIB + Ollama
    
    - ADead-BIB: Pre/post procesamiento rápido
    - Ollama: Generación de texto de alta calidad
    """
    
    def __init__(self, ollama_config: OllamaConfig = None):
        self.adead = ADeadBIB()
        self.ollama = OllamaClient(ollama_config)
        
        print("=" * 60)
        print("   HybridAI: ADead-BIB + Ollama")
        print("=" * 60)
        print(f"  ADead-BIB: ✅ Disponible")
        print(f"  Ollama:    {'✅' if self.ollama.available else '❌'} {'Disponible' if self.ollama.available else 'No disponible'}")
        print("=" * 60)
    
    def preprocess(self, text: str) -> str:
        """Pre-procesa texto con ADead-BIB."""
        # Limpiar y normalizar
        text = text.strip()
        text = ' '.join(text.split())  # Normalizar espacios
        return text
    
    def postprocess(self, text: str) -> str:
        """Post-procesa respuesta."""
        text = text.strip()
        return text
    
    def generate(self, prompt: str, use_ollama: bool = True) -> str:
        """Genera texto usando el mejor método disponible."""
        prompt = self.preprocess(prompt)
        
        if use_ollama and self.ollama.available:
            response = self.ollama.generate(prompt)
        else:
            response = f"[Respuesta local para: {prompt}]"
        
        return self.postprocess(response)
    
    def chat(self, message: str, history: List[Dict] = None) -> str:
        """Chat con historial."""
        if history is None:
            history = []
        
        message = self.preprocess(message)
        history.append({"role": "user", "content": message})
        
        if self.ollama.available:
            response = self.ollama.chat(history)
        else:
            response = f"[Respuesta para: {message}]"
        
        return self.postprocess(response)
    
    def analyze_with_ai(self, text: str) -> Dict:
        """Analiza texto usando IA."""
        prompt = f"Analyze this text briefly: {text}"
        
        analysis = {
            "text": text,
            "word_count": len(text.split()),
            "char_count": len(text),
        }
        
        if self.ollama.available:
            analysis["ai_summary"] = self.ollama.generate(prompt)
        
        return analysis


def demo():
    """Demo de integración con Ollama."""
    print("\n" + "=" * 60)
    print("   Demo: Integración Ollama + ADead-BIB")
    print("=" * 60)
    
    # Crear IA híbrida
    config = OllamaConfig(
        model="tinyllama",
        max_tokens=50,
        temperature=0.7
    )
    
    ai = HybridAI(config)
    
    if ai.ollama.available:
        print("\n🤖 Generando texto con Ollama...")
        
        prompts = [
            "What is Python?",
            "Explain AI briefly",
        ]
        
        for prompt in prompts:
            print(f"\nPrompt: '{prompt}'")
            start = time.time()
            response = ai.generate(prompt)
            elapsed = time.time() - start
            print(f"Respuesta: {response[:100]}...")
            print(f"Tiempo: {elapsed:.1f}s")
    else:
        print("\n⚠️ Ollama no está disponible.")
        print("Para instalar:")
        print("  1. winget install Ollama.Ollama")
        print("  2. ollama pull tinyllama")
        print("  3. Ejecutar 'ollama serve' en otra terminal")
    
    print("\n" + "=" * 60)
    print("   Demo completada")
    print("=" * 60)


if __name__ == "__main__":
    demo()
