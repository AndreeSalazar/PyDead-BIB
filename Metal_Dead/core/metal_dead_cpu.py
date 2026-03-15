"""
Metal-Dead CPU-First
=====================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪

IA Personal optimizada para CPU con integración ADead-BIB FFI.
Prioriza CPU antes de GPU para máxima compatibilidad.
"""

import sys
import time
import json
from pathlib import Path
from typing import List, Dict, Optional, Any
from dataclasses import dataclass, field

import numpy as np

# Importar módulos locales
from .cpu_compute import CPUCompute, CPUTransformer, ComputeBackend
from .tokenizer import SimpleTokenizer
from .memory import Memory
from .intelligence import IntelligenceEngine, ThoughtProcess


@dataclass
class MetalDeadCPUConfig:
    """Configuración de Metal-Dead CPU."""
    vocab_size: int = 5000
    embed_dim: int = 128
    num_heads: int = 4
    hidden_dim: int = 256
    num_layers: int = 2
    max_context: int = 512
    temperature: float = 0.8
    use_adead_ffi: bool = True
    num_threads: int = None  # Auto-detect


class MetalDeadCPU:
    """
    IA Personal Metal-Dead optimizada para CPU.
    Integra con ADead-BIB FFI para operaciones críticas.
    """
    
    def __init__(self, config: MetalDeadCPUConfig = None):
        self.config = config or MetalDeadCPUConfig()
        
        print("\n" + "=" * 60)
        print("   🤖 Metal-Dead CPU-First v1.0")
        print("   Integración ADead-BIB FFI")
        print("=" * 60)
        
        # Inicializar CPU Compute
        backend = ComputeBackend.CPU_ADEAD if self.config.use_adead_ffi else ComputeBackend.CPU_PARALLEL
        self.compute = CPUCompute(backend=backend, num_threads=self.config.num_threads)
        
        # Inicializar tokenizer
        self.tokenizer = SimpleTokenizer()
        actual_vocab_size = len(self.tokenizer)
        
        # Ajustar vocab_size si es necesario
        if actual_vocab_size < self.config.vocab_size:
            self.config.vocab_size = actual_vocab_size
        
        # Inicializar modelo transformer
        self.model = CPUTransformer(
            vocab_size=self.config.vocab_size,
            embed_dim=self.config.embed_dim,
            num_heads=self.config.num_heads,
            hidden_dim=self.config.hidden_dim,
            num_layers=self.config.num_layers,
            compute=self.compute
        )
        
        # Inicializar memoria y inteligencia
        self.memory = Memory()
        self.intelligence = IntelligenceEngine()
        
        # Estadísticas
        self.stats = {
            "total_queries": 0,
            "total_tokens_generated": 0,
            "avg_response_time_ms": 0,
        }
        
        print(f"\n✅ Metal-Dead CPU inicializado")
        print(f"   Vocab: {self.config.vocab_size}")
        print(f"   Embed: {self.config.embed_dim}")
        print(f"   Layers: {self.config.num_layers}")
        print(f"   Memoria modelo: {self.model.memory_mb:.1f} MB")
    
    def think(self, text: str) -> ThoughtProcess:
        """Procesa el pensamiento sobre el texto."""
        context = {
            "user_name": self.memory.get("user_name"),
            "interests": self.memory.get("interests", []),
            "recent_topics": self.memory.get("recent_topics", []),
        }
        return self.intelligence.think(text, context)
    
    def tokenize(self, text: str) -> List[int]:
        """Tokeniza texto."""
        return self.tokenizer.encode(text)
    
    def detokenize(self, tokens: List[int]) -> str:
        """Detokeniza tokens a texto."""
        return self.tokenizer.decode(tokens)
    
    def generate_response(self, prompt: str, max_tokens: int = 50) -> str:
        """
        Genera respuesta usando el modelo transformer.
        
        Args:
            prompt: Texto de entrada
            max_tokens: Máximo de tokens a generar
            
        Returns:
            Respuesta generada
        """
        start_time = time.perf_counter()
        
        # Tokenizar
        tokens = self.tokenize(prompt)
        
        # Limitar contexto
        if len(tokens) > self.config.max_context:
            tokens = tokens[-self.config.max_context:]
        
        # Generar
        generated = self.model.generate(
            tokens,
            max_tokens=max_tokens,
            temperature=self.config.temperature
        )
        
        # Detokenizar solo los nuevos tokens
        new_tokens = generated[len(tokens):]
        response = self.detokenize(new_tokens)
        
        # Actualizar estadísticas
        elapsed = (time.perf_counter() - start_time) * 1000
        self.stats["total_queries"] += 1
        self.stats["total_tokens_generated"] += len(new_tokens)
        self.stats["avg_response_time_ms"] = (
            (self.stats["avg_response_time_ms"] * (self.stats["total_queries"] - 1) + elapsed)
            / self.stats["total_queries"]
        )
        
        return response
    
    def chat(self, user_input: str) -> str:
        """
        Interfaz de chat principal.
        Combina pensamiento crítico con generación.
        
        Args:
            user_input: Mensaje del usuario
            
        Returns:
            Respuesta del asistente
        """
        from .intelligence import IntentType, SentimentType
        import re
        
        # Pensar sobre el input
        thought = self.think(user_input)
        
        # Respuestas basadas en intención (prioridad sobre modelo)
        response = None
        
        # Manejar saludos
        if thought.intent == IntentType.GREETING:
            user_name = self.memory.get("user_name", "")
            if user_name:
                response = f"¡Hola {user_name}! ¿En qué puedo ayudarte hoy?"
            else:
                response = "¡Hola! Soy Metal-Dead CPU, tu asistente de IA. ¿Cómo te llamas?"
        
        # Manejar aprendizaje (nombre, preferencias)
        elif thought.intent == IntentType.LEARNING:
            if "me llamo" in user_input.lower():
                match = re.search(r"me llamo\s+(\w+)", user_input.lower())
                if match:
                    name = match.group(1).capitalize()
                    self.memory.set("user_name", name)
                    response = f"¡Encantado de conocerte, {name}! Lo recordaré."
            elif "me gusta" in user_input.lower():
                match = re.search(r"me gusta\s+(.+)", user_input.lower())
                if match:
                    interest = match.group(1).strip()
                    interests = self.memory.get("interests", [])
                    interests.append(interest)
                    self.memory.set("interests", interests[:10])
                    response = f"¡Genial! Anotado que te gusta {interest}."
        
        # Manejar preguntas
        elif thought.intent == IntentType.QUESTION:
            # Buscar en conocimiento
            for kw in thought.keywords[:3]:
                knowledge = self.intelligence.get_knowledge_response(kw)
                if knowledge:
                    response = knowledge
                    break
            if not response:
                response = f"Interesante pregunta sobre {', '.join(thought.keywords[:2]) if thought.keywords else 'eso'}. Déjame pensar..."
        
        # Manejar comandos
        elif thought.intent == IntentType.COMMAND:
            response = "Entendido. Procesando tu solicitud..."
        
        # Manejar ayuda
        elif thought.intent == IntentType.HELP:
            response = "¡Estoy aquí para ayudarte! Puedes preguntarme sobre programación, IA, o simplemente chatear."
        
        # Respuesta por defecto basada en sentimiento
        if not response:
            if thought.sentiment == SentimentType.FRUSTRATED:
                response = "Entiendo que puede ser frustrante. ¿Puedo ayudarte de alguna manera?"
            elif thought.sentiment == SentimentType.EXCITED:
                response = "¡Me alegra tu entusiasmo! Cuéntame más."
            elif thought.sentiment == SentimentType.CURIOUS:
                response = "Buena pregunta. Déjame explicarte..."
            else:
                # Buscar conocimiento relevante
                for kw in thought.keywords[:3]:
                    knowledge = self.intelligence.get_knowledge_response(kw)
                    if knowledge:
                        response = knowledge
                        break
                if not response:
                    response = "Entendido. ¿Hay algo más en lo que pueda ayudarte?"
        
        # Actualizar temas recientes
        recent = self.memory.get("recent_topics", [])
        recent = thought.keywords[:3] + recent[:7]
        self.memory.set("recent_topics", recent)
        
        return response
    
    def process_command(self, command: str) -> str:
        """Procesa comandos especiales."""
        cmd = command.lower().strip()
        
        if cmd == "/stats":
            return self._format_stats()
        elif cmd == "/benchmark":
            return self._run_benchmark()
        elif cmd == "/memory":
            return self._show_memory()
        elif cmd == "/help":
            return self._show_help()
        elif cmd == "/clear":
            self.memory.clear()
            return "Memoria limpiada."
        else:
            return f"Comando desconocido: {command}"
    
    def _format_stats(self) -> str:
        """Formatea estadísticas."""
        lines = ["📊 Estadísticas Metal-Dead CPU:"]
        lines.append(f"   Queries: {self.stats['total_queries']}")
        lines.append(f"   Tokens generados: {self.stats['total_tokens_generated']}")
        lines.append(f"   Tiempo promedio: {self.stats['avg_response_time_ms']:.1f} ms")
        
        compute_metrics = self.compute.get_metrics()
        lines.append(f"\n🖥️ CPU Compute:")
        for key, value in compute_metrics.items():
            lines.append(f"   {key}: {value}")
        
        intel_stats = self.intelligence.get_stats()
        lines.append(f"\n🧠 Inteligencia:")
        for key, value in intel_stats.items():
            lines.append(f"   {key}: {value}")
        
        return "\n".join(lines)
    
    def _run_benchmark(self) -> str:
        """Ejecuta benchmark."""
        lines = ["⚡ Benchmark CPU:"]
        results = self.compute.benchmark(size=256, iterations=5)
        for name, time_ms in results.items():
            lines.append(f"   {name}: {time_ms:.2f} ms")
        return "\n".join(lines)
    
    def _show_memory(self) -> str:
        """Muestra contenido de memoria."""
        lines = ["💾 Memoria:"]
        for key, value in self.memory.data.items():
            lines.append(f"   {key}: {value}")
        return "\n".join(lines) if len(lines) > 1 else "Memoria vacía."
    
    def _show_help(self) -> str:
        """Muestra ayuda."""
        return """
🤖 Metal-Dead CPU - Comandos:
   /stats     - Ver estadísticas
   /benchmark - Ejecutar benchmark CPU
   /memory    - Ver memoria
   /clear     - Limpiar memoria
   /help      - Esta ayuda
   
Escribe cualquier mensaje para chatear.
"""
    
    def shutdown(self):
        """Cierra recursos."""
        self.compute.shutdown()
        print("👋 Metal-Dead CPU cerrado.")


# =============================================================================
# CLI INTERACTIVO
# =============================================================================

def main():
    """CLI interactivo de Metal-Dead CPU."""
    print("\n" + "=" * 60)
    print("   🤖 Metal-Dead CPU-First CLI")
    print("   Escribe /help para ver comandos")
    print("   Escribe 'salir' para terminar")
    print("=" * 60)
    
    ai = MetalDeadCPU()
    
    while True:
        try:
            user_input = input("\n👤 Tú: ").strip()
            
            if not user_input:
                continue
            
            if user_input.lower() in ["salir", "exit", "quit"]:
                ai.shutdown()
                break
            
            if user_input.startswith("/"):
                response = ai.process_command(user_input)
            else:
                response = ai.chat(user_input)
            
            print(f"\n🤖 Metal-Dead: {response}")
            
        except KeyboardInterrupt:
            ai.shutdown()
            break
        except Exception as e:
            print(f"\n❌ Error: {e}")


if __name__ == "__main__":
    main()
