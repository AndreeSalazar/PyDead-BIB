"""
Metal-Dead Smart - IA con Pensamiento Crítico
===============================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Metal-Dead con inteligencia avanzada:
- Pensamiento crítico
- Razonamiento lógico
- Base de conocimiento
- Análisis de contexto profundo
- Inferencia y deducción
"""

import re
import time
from typing import Dict, List, Optional, Tuple
from pathlib import Path

from .metal_dead import MetalDead, MetalDeadConfig
from .intelligence import IntelligenceEngine, CriticalThinking, IntentType, SentimentType


class MetalDeadSmart(MetalDead):
    """
    Metal-Dead con inteligencia avanzada.
    Piensa críticamente y razona antes de responder.
    """
    
    def __init__(self, config: MetalDeadConfig = None):
        super().__init__(config)
        
        # Motor de inteligencia
        self.intelligence = IntelligenceEngine()
        self.critical_thinking = CriticalThinking(self.intelligence)
        
        # Modo de pensamiento
        self.show_thinking = False  # Mostrar proceso de pensamiento
        self.deep_analysis = True   # Análisis profundo
        
        # Historial de pensamientos
        self.thought_history: List[Dict] = []
        
        # Temas recientes
        self.recent_topics: List[str] = []
        
        print("\n🧠 Motor de Inteligencia Avanzada activado")
        print(f"   Base de conocimiento: {len(self.intelligence.knowledge.get_all_topics())} temas")
    
    def _get_context(self) -> Dict:
        """Obtiene contexto actual para el razonamiento."""
        return {
            "user_name": self.context.profile.name,
            "interests": self.context.profile.interests,
            "recent_topics": self.recent_topics[-5:] if self.recent_topics else [],
            "interaction_count": self.context.profile.interaction_count,
            "learned_facts": self.context.profile.learned_facts,
        }
    
    def think(self, message: str) -> Dict:
        """
        Proceso de pensamiento antes de responder.
        Analiza, razona y evalúa.
        """
        context = self._get_context()
        thought = self.intelligence.think(message, context)
        
        # Guardar en historial
        thought_record = {
            "input": message,
            "intent": thought.intent.value,
            "sentiment": thought.sentiment.value,
            "keywords": thought.keywords,
            "confidence": thought.confidence,
            "reasoning": thought.reasoning_steps,
            "conclusion": thought.conclusion,
            "time_ms": thought.processing_time_ms,
        }
        self.thought_history.append(thought_record)
        
        # Actualizar temas recientes
        self.recent_topics.extend(thought.keywords[:3])
        self.recent_topics = self.recent_topics[-10:]
        
        return thought_record
    
    def _generate_intelligent_response(self, message: str, thought: Dict) -> str:
        """Genera respuesta basada en pensamiento crítico."""
        intent = IntentType(thought["intent"])
        sentiment = SentimentType(thought["sentiment"])
        keywords = thought["keywords"]
        name = self.context.profile.name
        
        # Buscar conocimiento relevante
        knowledge_responses = []
        for kw in keywords[:3]:
            knowledge = self.intelligence.get_knowledge_response(kw)
            if knowledge:
                knowledge_responses.append(knowledge)
        
        # Generar respuesta según intención
        if intent == IntentType.GREETING:
            return self._smart_greeting(sentiment)
        
        elif intent == IntentType.QUESTION:
            return self._smart_answer(message, keywords, knowledge_responses)
        
        elif intent == IntentType.LEARNING:
            # Usar el sistema de aprendizaje del padre
            learning_response = self._check_learning(message)
            if learning_response:
                return learning_response
            return "Entendido, lo tendré en cuenta."
        
        elif intent == IntentType.HELP:
            return self._smart_help(sentiment)
        
        elif intent == IntentType.COMMAND:
            return self._smart_command(message, keywords)
        
        elif intent == IntentType.SEARCH:
            return self._smart_search(message, keywords, knowledge_responses)
        
        elif sentiment == SentimentType.FRUSTRATED:
            return self._empathetic_response(message, name)
        
        elif sentiment == SentimentType.EXCITED:
            return self._enthusiastic_response(message, name)
        
        else:
            return self._contextual_response(message, keywords, knowledge_responses)
    
    def _smart_greeting(self, sentiment: SentimentType) -> str:
        """Saludo inteligente basado en contexto."""
        base_greeting = self.context.get_greeting()
        
        if sentiment == SentimentType.EXCITED:
            return f"{base_greeting} ¡Veo que estás con energía hoy!"
        elif sentiment == SentimentType.FRUSTRATED:
            return f"{base_greeting} ¿Todo bien? Estoy aquí para ayudarte."
        
        return base_greeting
    
    def _smart_answer(self, question: str, keywords: List[str], knowledge: List[str]) -> str:
        """Respuesta inteligente a preguntas."""
        # Si hay conocimiento relevante, usarlo
        if knowledge:
            response = knowledge[0]
            
            # Agregar información adicional si hay más conocimiento
            if len(knowledge) > 1:
                response += f"\n\nAdemás, {knowledge[1][:100]}..."
            
            return response
        
        # Respuestas basadas en keywords
        if "gpu" in keywords or "rtx" in keywords or "3060" in keywords:
            return """La GPU (Graphics Processing Unit) es un procesador especializado en cálculos paralelos.

Tu RTX 3060 tiene:
• 12GB VRAM (mucha memoria para IA)
• Arquitectura Ampere (última generación)
• Tensor Cores (aceleración de IA)
• CUDA Cores para cómputo paralelo

Es excelente para:
- Deep Learning y entrenamiento de modelos
- Inferencia de IA en tiempo real
- Procesamiento de datos masivos
- Renderizado y gráficos

¿Te gustaría saber cómo aprovecharla mejor?"""
        
        if "ia" in keywords or "inteligencia" in keywords:
            return """La Inteligencia Artificial es la simulación de procesos de inteligencia humana por sistemas computacionales.

Incluye:
• Machine Learning (aprendizaje automático)
• Deep Learning (redes neuronales profundas)
• NLP (procesamiento de lenguaje natural)
• Computer Vision (visión por computadora)

Yo, Metal-Dead, uso técnicas de IA para:
- Entender tus mensajes
- Aprender de nuestras conversaciones
- Razonar y pensar críticamente
- Recordar información importante

¿Qué aspecto de la IA te interesa más?"""
        
        if "adead" in keywords or "compilador" in keywords:
            return """ADead-BIB es un compilador revolucionario que genera binarios ultra-compactos.

Características:
• Sin runtime - código máquina directo
• Binarios de 1-2 KB
• Sintaxis similar a Python
• Soporte para GPU (Vulkan, CUDA)
• Generación de opcodes optimizados

Metal-Dead está diseñado para funcionar perfectamente con ADead-BIB, aprovechando su eficiencia."""
        
        # Respuesta general para preguntas
        return f"Interesante pregunta sobre {', '.join(keywords[:2]) if keywords else 'eso'}. Déjame pensar... ¿Podrías darme más contexto para darte una mejor respuesta?"
    
    def _smart_help(self, sentiment: SentimentType) -> str:
        """Ayuda inteligente."""
        base_help = self._get_help()
        
        if sentiment == SentimentType.FRUSTRATED:
            return f"Entiendo que puede ser frustrante. Aquí tienes ayuda:\n\n{base_help}\n\n¿En qué específicamente necesitas ayuda?"
        
        return base_help
    
    def _smart_command(self, message: str, keywords: List[str]) -> str:
        """Procesa comandos de forma inteligente."""
        message_lower = message.lower()
        
        if "busca" in message_lower or "encuentra" in message_lower:
            query = re.sub(r'^(busca|encuentra|search)\s+', '', message_lower).strip()
            return self._search_memory(query)
        
        if "abre" in message_lower:
            return "Para abrir aplicaciones, usa el modo de voz con --voice. Por ahora puedo ayudarte con información."
        
        return "Entendido. ¿Qué te gustaría que haga específicamente?"
    
    def _smart_search(self, message: str, keywords: List[str], knowledge: List[str]) -> str:
        """Búsqueda inteligente."""
        # Buscar en memoria
        memory_results = self.memory.search(" ".join(keywords), top_k=3)
        
        results = []
        
        # Agregar conocimiento
        if knowledge:
            results.append(f"📚 **Conocimiento:**\n{knowledge[0]}")
        
        # Agregar memorias
        if memory_results:
            mem_text = "\n".join([f"• {m.content[:80]}..." for m in memory_results])
            results.append(f"\n💾 **Memorias relacionadas:**\n{mem_text}")
        
        if results:
            return "\n".join(results)
        
        return f"No encontré información específica sobre '{' '.join(keywords[:2])}'. ¿Podrías contarme más?"
    
    def _empathetic_response(self, message: str, name: str) -> str:
        """Respuesta empática para frustración."""
        return f"""Entiendo tu frustración, {name}. A veces las cosas no salen como esperamos.

¿Puedo ayudarte con algo específico? Cuéntame qué está pasando y veré cómo puedo asistirte.

Recuerda: cada problema tiene solución, solo hay que encontrarla paso a paso. 💪"""
    
    def _enthusiastic_response(self, message: str, name: str) -> str:
        """Respuesta entusiasta."""
        return f"¡Genial, {name}! Me encanta tu entusiasmo. ¿Qué te tiene tan emocionado? ¡Cuéntame más!"
    
    def _contextual_response(self, message: str, keywords: List[str], knowledge: List[str]) -> str:
        """Respuesta contextual general."""
        name = self.context.profile.name
        
        # Si hay conocimiento, usarlo
        if knowledge:
            return f"Sobre eso, {knowledge[0][:150]}... ¿Te gustaría saber más?"
        
        # Buscar en memorias
        relevant = self.memory.search(message, top_k=1)
        if relevant:
            return f"Hmm, recuerdo algo relacionado: {relevant[0].content[:100]}... ¿Es sobre esto que quieres hablar?"
        
        # Respuesta basada en intereses del usuario
        if self.context.profile.interests:
            interest = self.context.profile.interests[0]
            return f"Interesante, {name}. ¿Esto tiene que ver con tu interés en {interest}?"
        
        return f"Entiendo, {name}. Cuéntame más sobre eso para poder ayudarte mejor."
    
    def chat(self, message: str) -> str:
        """Chat con pensamiento crítico."""
        self.context.update_interaction()
        
        # Primero, pensar
        thought = self.think(message)
        
        # Verificar aprendizaje primero
        learning_response = self._check_learning(message)
        if learning_response:
            return learning_response
        
        # Comandos especiales
        message_lower = message.lower().strip()
        
        if message_lower in ["hola", "hi", "hello"]:
            return self._smart_greeting(SentimentType(thought["sentiment"]))
        
        if message_lower in ["ayuda", "help", "?"]:
            return self._smart_help(SentimentType(thought["sentiment"]))
        
        if message_lower in ["memoria", "memorias", "memory"]:
            return self._get_memory_stats()
        
        if message_lower in ["perfil", "profile"]:
            return self.context.get_summary()
        
        if message_lower in ["pensamiento", "thinking", "razonamiento"]:
            return self._show_last_thought()
        
        if message_lower in ["estadísticas", "stats", "inteligencia"]:
            return self._show_intelligence_stats()
        
        if message_lower.startswith("busca ") or message_lower.startswith("search "):
            query = message[6:].strip()
            return self._search_memory(query)
        
        # Generar respuesta inteligente
        response = self._generate_intelligent_response(message, thought)
        
        # Evaluar y refinar respuesta
        if self.deep_analysis:
            evaluation = self.critical_thinking.evaluate_response(message, response)
            if evaluation["average"] < 0.7:
                response = self.critical_thinking.refine_response(message, response)
        
        # Guardar en memoria
        self.conversation_history.append((message, response))
        self.memory.add(f"Usuario: {message}", category="conversations")
        
        return response
    
    def _show_last_thought(self) -> str:
        """Muestra el último proceso de pensamiento."""
        if not self.thought_history:
            return "Aún no he procesado ningún pensamiento."
        
        last = self.thought_history[-1]
        lines = [
            "🧠 **Último Proceso de Pensamiento:**",
            f"• Entrada: {last['input'][:50]}...",
            f"• Intención: {last['intent']}",
            f"• Sentimiento: {last['sentiment']}",
            f"• Keywords: {', '.join(last['keywords'][:5])}",
            f"• Confianza: {last['confidence']:.1%}",
            f"• Tiempo: {last['time_ms']:.2f} ms",
            "\n📋 **Razonamiento:**",
        ]
        for i, step in enumerate(last['reasoning'], 1):
            lines.append(f"   {i}. {step}")
        lines.append(f"\n✅ **Conclusión:** {last['conclusion']}")
        
        return "\n".join(lines)
    
    def _show_intelligence_stats(self) -> str:
        """Muestra estadísticas de inteligencia."""
        stats = self.intelligence.get_stats()
        lines = [
            "🧠 **Estadísticas de Inteligencia:**",
            f"• Pensamientos procesados: {stats['total_thoughts']}",
            f"• Confianza promedio: {stats['avg_confidence']:.1%}",
            f"• Temas en conocimiento: {stats['knowledge_topics']}",
            "\n📊 **Intenciones detectadas:**",
        ]
        for intent, count in stats['intents'].items():
            lines.append(f"   • {intent}: {count}")
        
        return "\n".join(lines)
    
    def enable_thinking_display(self, enabled: bool = True):
        """Activa/desactiva mostrar proceso de pensamiento."""
        self.show_thinking = enabled
    
    def get_stats(self) -> Dict:
        """Obtiene estadísticas extendidas."""
        base_stats = super().get_stats()
        intel_stats = self.intelligence.get_stats()
        
        return {
            **base_stats,
            "intelligence": intel_stats,
            "thought_count": len(self.thought_history),
            "recent_topics": self.recent_topics[-5:],
        }
