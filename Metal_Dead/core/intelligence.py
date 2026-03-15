"""
Sistema de Inteligencia Avanzada para Metal-Dead
=================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Sistema de pensamiento crítico e inteligencia:
- Análisis de contexto profundo
- Razonamiento lógico
- Inferencia y deducción
- Pensamiento crítico
- Base de conocimiento
- Análisis de sentimiento
- Detección de intenciones
"""

import re
import time
import math
from typing import List, Dict, Optional, Tuple, Any
from dataclasses import dataclass, field
from enum import Enum
from collections import defaultdict

import numpy as np


class IntentType(Enum):
    """Tipos de intención del usuario."""
    GREETING = "greeting"
    QUESTION = "question"
    COMMAND = "command"
    INFORMATION = "information"
    REQUEST = "request"
    OPINION = "opinion"
    EMOTION = "emotion"
    LEARNING = "learning"
    SEARCH = "search"
    HELP = "help"
    UNKNOWN = "unknown"


class SentimentType(Enum):
    """Tipos de sentimiento."""
    POSITIVE = "positive"
    NEGATIVE = "negative"
    NEUTRAL = "neutral"
    CURIOUS = "curious"
    FRUSTRATED = "frustrated"
    EXCITED = "excited"


@dataclass
class ThoughtProcess:
    """Proceso de pensamiento estructurado."""
    input_text: str
    intent: IntentType
    sentiment: SentimentType
    entities: List[str]
    keywords: List[str]
    context_relevance: float
    reasoning_steps: List[str]
    conclusion: str
    confidence: float
    processing_time_ms: float


@dataclass
class KnowledgeItem:
    """Item de conocimiento."""
    topic: str
    content: str
    source: str
    confidence: float
    related_topics: List[str] = field(default_factory=list)
    usage_count: int = 0


class KnowledgeBase:
    """Base de conocimiento integrada."""
    
    def __init__(self):
        self.knowledge: Dict[str, List[KnowledgeItem]] = defaultdict(list)
        self._init_base_knowledge()
    
    def _init_base_knowledge(self):
        """Inicializa conocimiento base."""
        base_knowledge = [
            # Programación
            KnowledgeItem("python", "Python es un lenguaje de programación interpretado, de alto nivel y propósito general.", "base", 1.0, ["programación", "código", "desarrollo"]),
            KnowledgeItem("programación", "La programación es el proceso de crear instrucciones para que una computadora ejecute tareas.", "base", 1.0, ["código", "desarrollo", "software"]),
            KnowledgeItem("ia", "La Inteligencia Artificial es la simulación de procesos de inteligencia humana por sistemas computacionales.", "base", 1.0, ["machine learning", "deep learning", "neural networks"]),
            KnowledgeItem("gpu", "Una GPU (Graphics Processing Unit) es un procesador especializado en cálculos paralelos, ideal para IA y gráficos.", "base", 1.0, ["cuda", "nvidia", "rtx", "procesamiento"]),
            KnowledgeItem("cuda", "CUDA es una plataforma de computación paralela de NVIDIA para GPUs.", "base", 1.0, ["gpu", "nvidia", "programación paralela"]),
            KnowledgeItem("rtx 3060", "La RTX 3060 es una GPU de NVIDIA con 12GB VRAM, arquitectura Ampere, Tensor Cores y Ray Tracing.", "base", 1.0, ["gpu", "nvidia", "cuda", "gaming", "ia"]),
            
            # ADead-BIB
            KnowledgeItem("adead-bib", "ADead-BIB es un compilador sin runtime que genera binarios ultra-compactos directamente a código máquina.", "base", 1.0, ["compilador", "binarios", "código máquina"]),
            KnowledgeItem("metal-dead", "Metal-Dead es una IA personal ultra-eficiente diseñada para funcionar con ADead-BIB.", "base", 1.0, ["ia", "asistente", "adead-bib"]),
            
            # Conceptos de IA
            KnowledgeItem("transformer", "Un Transformer es una arquitectura de red neuronal basada en mecanismos de atención.", "base", 1.0, ["ia", "deep learning", "attention"]),
            KnowledgeItem("attention", "El mecanismo de atención permite a los modelos enfocarse en partes relevantes de la entrada.", "base", 1.0, ["transformer", "ia", "neural networks"]),
            KnowledgeItem("embeddings", "Los embeddings son representaciones vectoriales densas de datos discretos como palabras.", "base", 1.0, ["ia", "nlp", "vectores"]),
            
            # General
            KnowledgeItem("pensamiento crítico", "El pensamiento crítico es el análisis objetivo de hechos para formar un juicio.", "base", 1.0, ["razonamiento", "lógica", "análisis"]),
            KnowledgeItem("razonamiento", "El razonamiento es el proceso de pensar de manera lógica para llegar a conclusiones.", "base", 1.0, ["lógica", "pensamiento", "deducción"]),
        ]
        
        for item in base_knowledge:
            self.knowledge[item.topic.lower()].append(item)
    
    def query(self, topic: str, threshold: float = 0.3) -> List[KnowledgeItem]:
        """Busca conocimiento sobre un tema."""
        topic_lower = topic.lower()
        results = []
        
        # Búsqueda directa
        if topic_lower in self.knowledge:
            results.extend(self.knowledge[topic_lower])
        
        # Búsqueda por palabras clave
        topic_words = set(topic_lower.split())
        for key, items in self.knowledge.items():
            key_words = set(key.split())
            overlap = len(topic_words & key_words) / max(len(topic_words), 1)
            if overlap >= threshold and key != topic_lower:
                results.extend(items)
        
        # Búsqueda en temas relacionados
        for key, items in self.knowledge.items():
            for item in items:
                for related in item.related_topics:
                    if topic_lower in related.lower() or related.lower() in topic_lower:
                        if item not in results:
                            results.append(item)
        
        return results
    
    def add(self, topic: str, content: str, source: str = "learned", confidence: float = 0.8, related: List[str] = None):
        """Agrega conocimiento."""
        item = KnowledgeItem(
            topic=topic,
            content=content,
            source=source,
            confidence=confidence,
            related_topics=related or []
        )
        self.knowledge[topic.lower()].append(item)
    
    def get_all_topics(self) -> List[str]:
        """Obtiene todos los temas."""
        return list(self.knowledge.keys())


class IntelligenceEngine:
    """
    Motor de inteligencia avanzada para Metal-Dead.
    Implementa pensamiento crítico, razonamiento y análisis profundo.
    """
    
    def __init__(self):
        self.knowledge = KnowledgeBase()
        
        # Patrones de intención
        self.intent_patterns = {
            IntentType.GREETING: [
                r"^(hola|hi|hello|hey|buenos?\s*(días|tardes|noches)|saludos)",
            ],
            IntentType.QUESTION: [
                r"^(qué|que|cómo|como|cuál|cual|cuándo|cuando|dónde|donde|por\s*qué|porque|quién|quien)",
                r"\?$",
                r"(puedes|podrías|sabes|conoces|entiendes)",
            ],
            IntentType.COMMAND: [
                r"^(haz|hazme|ejecuta|corre|abre|cierra|mueve|escribe|busca|encuentra)",
            ],
            IntentType.INFORMATION: [
                r"(te cuento|te digo|sabías que|fíjate que|mira)",
                r"(es|son|fue|fueron|será|serán)\s+\w+",
            ],
            IntentType.REQUEST: [
                r"(necesito|quiero|quisiera|me gustaría|podrías|puedes)",
            ],
            IntentType.LEARNING: [
                r"(me llamo|mi nombre es|soy|me gusta|me interesa|recuerda que)",
            ],
            IntentType.SEARCH: [
                r"(busca|encuentra|dime sobre|qué sabes de|información sobre)",
            ],
            IntentType.HELP: [
                r"(ayuda|help|socorro|no entiendo|estoy perdido)",
            ],
            IntentType.EMOTION: [
                r"(estoy|me siento|siento)\s+(feliz|triste|enojado|frustrado|emocionado|cansado)",
            ],
        }
        
        # Palabras de sentimiento
        self.sentiment_words = {
            SentimentType.POSITIVE: ["bien", "genial", "excelente", "perfecto", "gracias", "bueno", "increíble", "fantástico", "amor", "feliz"],
            SentimentType.NEGATIVE: ["mal", "terrible", "horrible", "odio", "triste", "frustrado", "enojado", "problema", "error", "falla"],
            SentimentType.CURIOUS: ["cómo", "por qué", "qué", "cuál", "interesante", "curioso", "explica", "entender"],
            SentimentType.EXCITED: ["wow", "increíble", "asombroso", "genial", "emocionado", "impresionante"],
            SentimentType.FRUSTRATED: ["no funciona", "no puedo", "error", "problema", "ayuda", "frustrado", "difícil"],
        }
        
        # Estadísticas
        self.stats = {
            "total_thoughts": 0,
            "avg_confidence": 0,
            "intents_detected": defaultdict(int),
        }
    
    def detect_intent(self, text: str) -> IntentType:
        """Detecta la intención del usuario."""
        text_lower = text.lower().strip()
        
        for intent, patterns in self.intent_patterns.items():
            for pattern in patterns:
                if re.search(pattern, text_lower, re.IGNORECASE):
                    return intent
        
        return IntentType.UNKNOWN
    
    def detect_sentiment(self, text: str) -> SentimentType:
        """Detecta el sentimiento del texto."""
        text_lower = text.lower()
        scores = {sentiment: 0 for sentiment in SentimentType}
        
        for sentiment, words in self.sentiment_words.items():
            for word in words:
                if word in text_lower:
                    scores[sentiment] += 1
        
        max_sentiment = max(scores, key=scores.get)
        if scores[max_sentiment] == 0:
            return SentimentType.NEUTRAL
        return max_sentiment
    
    def extract_entities(self, text: str) -> List[str]:
        """Extrae entidades del texto."""
        entities = []
        
        # Nombres propios (palabras capitalizadas)
        names = re.findall(r'\b[A-Z][a-z]+\b', text)
        entities.extend(names)
        
        # Números
        numbers = re.findall(r'\b\d+(?:\.\d+)?\b', text)
        entities.extend(numbers)
        
        # Tecnologías conocidas
        tech_patterns = [
            r'\b(python|java|javascript|rust|c\+\+|gpu|cpu|cuda|rtx|nvidia|amd|intel)\b',
            r'\b(ia|ai|ml|deep\s*learning|machine\s*learning|neural\s*network)\b',
            r'\b(adead|metal-dead|transformer|attention)\b',
        ]
        for pattern in tech_patterns:
            matches = re.findall(pattern, text, re.IGNORECASE)
            entities.extend(matches)
        
        return list(set(entities))
    
    def extract_keywords(self, text: str) -> List[str]:
        """Extrae palabras clave del texto."""
        # Remover stopwords básicas
        stopwords = {"el", "la", "los", "las", "un", "una", "de", "en", "a", "que", "y", "o", "es", "son", "por", "para", "con", "sin", "sobre", "como", "más", "pero", "si", "no", "me", "te", "se", "mi", "tu", "su"}
        
        words = re.findall(r'\b\w+\b', text.lower())
        keywords = [w for w in words if w not in stopwords and len(w) > 2]
        
        # Ordenar por frecuencia
        from collections import Counter
        freq = Counter(keywords)
        return [w for w, _ in freq.most_common(10)]
    
    def reason(self, text: str, context: Dict = None) -> List[str]:
        """Genera pasos de razonamiento."""
        steps = []
        intent = self.detect_intent(text)
        sentiment = self.detect_sentiment(text)
        keywords = self.extract_keywords(text)
        
        # Paso 1: Análisis de entrada
        steps.append(f"Analizando entrada: '{text[:50]}...' " if len(text) > 50 else f"Analizando entrada: '{text}'")
        
        # Paso 2: Identificación de intención
        steps.append(f"Intención detectada: {intent.value}")
        
        # Paso 3: Análisis de sentimiento
        steps.append(f"Sentimiento: {sentiment.value}")
        
        # Paso 4: Búsqueda de conocimiento relevante
        relevant_knowledge = []
        for kw in keywords[:3]:
            knowledge = self.knowledge.query(kw)
            if knowledge:
                relevant_knowledge.extend(knowledge)
        
        if relevant_knowledge:
            topics = list(set(k.topic for k in relevant_knowledge[:3]))
            steps.append(f"Conocimiento relevante encontrado: {', '.join(topics)}")
        else:
            steps.append("No se encontró conocimiento específico, usando razonamiento general")
        
        # Paso 5: Contexto
        if context:
            if context.get("user_name"):
                steps.append(f"Contexto: Usuario conocido como {context['user_name']}")
            if context.get("interests"):
                steps.append(f"Intereses del usuario: {', '.join(context['interests'][:3])}")
        
        # Paso 6: Formulación de respuesta
        if intent == IntentType.QUESTION:
            steps.append("Formulando respuesta informativa basada en conocimiento")
        elif intent == IntentType.GREETING:
            steps.append("Preparando saludo personalizado")
        elif intent == IntentType.LEARNING:
            steps.append("Procesando nueva información para aprender")
        elif intent == IntentType.COMMAND:
            steps.append("Evaluando comando para ejecución")
        else:
            steps.append("Generando respuesta contextual")
        
        return steps
    
    def think(self, text: str, context: Dict = None) -> ThoughtProcess:
        """
        Proceso de pensamiento completo.
        Analiza, razona y genera conclusiones.
        """
        start = time.perf_counter()
        
        intent = self.detect_intent(text)
        sentiment = self.detect_sentiment(text)
        entities = self.extract_entities(text)
        keywords = self.extract_keywords(text)
        reasoning_steps = self.reason(text, context)
        
        # Calcular relevancia del contexto
        context_relevance = 0.5
        if context:
            if any(kw in str(context.get("interests", [])).lower() for kw in keywords):
                context_relevance += 0.3
            if context.get("user_name"):
                context_relevance += 0.1
            if context.get("recent_topics"):
                if any(kw in context["recent_topics"] for kw in keywords):
                    context_relevance += 0.1
        
        # Generar conclusión
        conclusion = self._generate_conclusion(intent, sentiment, keywords, context)
        
        # Calcular confianza
        confidence = self._calculate_confidence(intent, keywords, context)
        
        elapsed = (time.perf_counter() - start) * 1000
        
        # Actualizar estadísticas
        self.stats["total_thoughts"] += 1
        self.stats["intents_detected"][intent.value] += 1
        self.stats["avg_confidence"] = (
            (self.stats["avg_confidence"] * (self.stats["total_thoughts"] - 1) + confidence) 
            / self.stats["total_thoughts"]
        )
        
        return ThoughtProcess(
            input_text=text,
            intent=intent,
            sentiment=sentiment,
            entities=entities,
            keywords=keywords,
            context_relevance=min(1.0, context_relevance),
            reasoning_steps=reasoning_steps,
            conclusion=conclusion,
            confidence=confidence,
            processing_time_ms=elapsed
        )
    
    def _generate_conclusion(self, intent: IntentType, sentiment: SentimentType, 
                            keywords: List[str], context: Dict = None) -> str:
        """Genera una conclusión basada en el análisis."""
        if intent == IntentType.GREETING:
            return "Responder con saludo personalizado"
        elif intent == IntentType.QUESTION:
            if keywords:
                return f"Buscar información sobre: {', '.join(keywords[:3])}"
            return "Responder pregunta general"
        elif intent == IntentType.LEARNING:
            return "Almacenar nueva información en memoria"
        elif intent == IntentType.COMMAND:
            return "Evaluar y ejecutar comando si es posible"
        elif intent == IntentType.HELP:
            return "Proporcionar ayuda y guía"
        elif sentiment == SentimentType.FRUSTRATED:
            return "Ofrecer asistencia empática"
        elif sentiment == SentimentType.EXCITED:
            return "Responder con entusiasmo"
        else:
            return "Generar respuesta contextual relevante"
    
    def _calculate_confidence(self, intent: IntentType, keywords: List[str], 
                             context: Dict = None) -> float:
        """Calcula la confianza en la respuesta."""
        base_confidence = 0.6
        
        # Mayor confianza si la intención es clara
        if intent != IntentType.UNKNOWN:
            base_confidence += 0.15
        
        # Mayor confianza si hay keywords reconocidos
        known_topics = self.knowledge.get_all_topics()
        matching_keywords = sum(1 for kw in keywords if kw.lower() in known_topics)
        base_confidence += min(0.15, matching_keywords * 0.05)
        
        # Mayor confianza con contexto
        if context:
            if context.get("user_name"):
                base_confidence += 0.05
            if context.get("interests"):
                base_confidence += 0.05
        
        return min(0.95, base_confidence)
    
    def get_knowledge_response(self, topic: str) -> Optional[str]:
        """Obtiene respuesta basada en conocimiento."""
        knowledge = self.knowledge.query(topic)
        if knowledge:
            best = max(knowledge, key=lambda k: k.confidence)
            best.usage_count += 1
            return best.content
        return None
    
    def learn_from_interaction(self, text: str, response: str, feedback: str = None):
        """Aprende de la interacción."""
        keywords = self.extract_keywords(text)
        
        # Si hay feedback positivo, reforzar conocimiento
        if feedback and feedback.lower() in ["bien", "correcto", "sí", "exacto"]:
            for kw in keywords:
                existing = self.knowledge.query(kw)
                for item in existing:
                    item.confidence = min(1.0, item.confidence + 0.05)
    
    def get_stats(self) -> Dict:
        """Obtiene estadísticas del motor de inteligencia."""
        return {
            "total_thoughts": self.stats["total_thoughts"],
            "avg_confidence": round(self.stats["avg_confidence"], 3),
            "intents": dict(self.stats["intents_detected"]),
            "knowledge_topics": len(self.knowledge.get_all_topics()),
        }


class CriticalThinking:
    """
    Sistema de pensamiento crítico.
    Evalúa, analiza y cuestiona para mejorar respuestas.
    """
    
    def __init__(self, intelligence: IntelligenceEngine):
        self.intelligence = intelligence
        self.evaluation_criteria = [
            "relevancia",
            "precisión",
            "completitud",
            "claridad",
            "utilidad",
        ]
    
    def evaluate_response(self, question: str, response: str) -> Dict:
        """Evalúa críticamente una respuesta."""
        scores = {}
        
        # Relevancia: ¿La respuesta aborda la pregunta?
        q_keywords = set(self.intelligence.extract_keywords(question))
        r_keywords = set(self.intelligence.extract_keywords(response))
        overlap = len(q_keywords & r_keywords) / max(len(q_keywords), 1)
        scores["relevancia"] = min(1.0, overlap + 0.3)
        
        # Completitud: ¿La respuesta es suficientemente detallada?
        word_count = len(response.split())
        scores["completitud"] = min(1.0, word_count / 20)
        
        # Claridad: ¿La respuesta es clara?
        avg_word_len = sum(len(w) for w in response.split()) / max(len(response.split()), 1)
        scores["claridad"] = 1.0 if avg_word_len < 8 else 0.7
        
        # Utilidad: ¿La respuesta es útil?
        useful_indicators = ["porque", "ya que", "esto significa", "por ejemplo", "puedes"]
        scores["utilidad"] = 0.5 + sum(0.1 for ind in useful_indicators if ind in response.lower())
        scores["utilidad"] = min(1.0, scores["utilidad"])
        
        # Precisión: Basada en conocimiento
        knowledge_match = False
        for kw in q_keywords:
            if self.intelligence.get_knowledge_response(kw):
                knowledge_match = True
                break
        scores["precisión"] = 0.8 if knowledge_match else 0.5
        
        # Promedio
        avg_score = sum(scores.values()) / len(scores)
        
        return {
            "scores": scores,
            "average": round(avg_score, 3),
            "needs_improvement": [k for k, v in scores.items() if v < 0.6],
        }
    
    def suggest_improvements(self, evaluation: Dict) -> List[str]:
        """Sugiere mejoras basadas en la evaluación."""
        suggestions = []
        
        for criterion in evaluation.get("needs_improvement", []):
            if criterion == "relevancia":
                suggestions.append("Incluir más términos relacionados con la pregunta")
            elif criterion == "completitud":
                suggestions.append("Expandir la respuesta con más detalles")
            elif criterion == "claridad":
                suggestions.append("Usar palabras más simples y directas")
            elif criterion == "utilidad":
                suggestions.append("Agregar ejemplos o explicaciones prácticas")
            elif criterion == "precisión":
                suggestions.append("Verificar información con base de conocimiento")
        
        return suggestions
    
    def refine_response(self, question: str, original_response: str) -> str:
        """Refina una respuesta usando pensamiento crítico."""
        evaluation = self.evaluate_response(question, original_response)
        
        if evaluation["average"] >= 0.8:
            return original_response
        
        # Intentar mejorar
        refined = original_response
        
        # Si falta relevancia, agregar contexto
        if "relevancia" in evaluation["needs_improvement"]:
            q_keywords = self.intelligence.extract_keywords(question)
            for kw in q_keywords[:2]:
                knowledge = self.intelligence.get_knowledge_response(kw)
                if knowledge:
                    refined = f"{refined} {knowledge[:100]}..."
                    break
        
        # Si falta completitud, expandir
        if "completitud" in evaluation["needs_improvement"]:
            refined = f"{refined} ¿Te gustaría saber más sobre esto?"
        
        return refined


# =============================================================================
# DEMO
# =============================================================================

def demo():
    """Demo del sistema de inteligencia."""
    print("\n" + "=" * 60)
    print("   🧠 Demo de Inteligencia Avanzada")
    print("   Metal-Dead Intelligence Engine")
    print("=" * 60)
    
    engine = IntelligenceEngine()
    critical = CriticalThinking(engine)
    
    test_inputs = [
        "Hola, ¿cómo estás?",
        "¿Qué es una GPU y para qué sirve?",
        "Me llamo Developer y me gusta la IA",
        "Estoy frustrado porque no funciona mi código",
        "Busca información sobre transformers",
        "¿Qué sabes sobre la RTX 3060?",
    ]
    
    for text in test_inputs:
        print(f"\n{'='*60}")
        print(f"📝 Input: {text}")
        print("-" * 60)
        
        thought = engine.think(text, {"user_name": "Developer", "interests": ["programación", "ia"]})
        
        print(f"🎯 Intención: {thought.intent.value}")
        print(f"💭 Sentimiento: {thought.sentiment.value}")
        print(f"🔑 Keywords: {', '.join(thought.keywords[:5])}")
        print(f"📊 Confianza: {thought.confidence:.1%}")
        print(f"⏱️ Tiempo: {thought.processing_time_ms:.2f} ms")
        print(f"\n🧠 Razonamiento:")
        for i, step in enumerate(thought.reasoning_steps, 1):
            print(f"   {i}. {step}")
        print(f"\n✅ Conclusión: {thought.conclusion}")
    
    print(f"\n{'='*60}")
    print("📊 Estadísticas del Motor:")
    stats = engine.get_stats()
    for key, value in stats.items():
        print(f"   {key}: {value}")
    print("=" * 60)


if __name__ == "__main__":
    demo()
