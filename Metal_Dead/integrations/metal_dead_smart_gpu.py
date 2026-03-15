"""
Metal-Dead Smart GPU - IA Inteligente con GPU MAX
==================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Combina:
- Pensamiento crítico
- Razonamiento lógico
- GPU MAX (Flash Attention + BF16)
- Base de conocimiento
"""

import sys
import time
from pathlib import Path
from typing import Dict, List

sys.path.insert(0, str(Path(__file__).parent.parent))

from Metal_Dead.core.metal_dead_smart import MetalDeadSmart
from Metal_Dead.core.metal_dead import MetalDeadConfig
from Metal_Dead.core.intelligence import IntelligenceEngine, CriticalThinking

# GPU imports
try:
    from Metal_Dead.integrations.gpu_advanced import GPUAdvanced, GPUConfig, PrecisionMode, AdvancedGPUTransformer
    HAS_GPU = True
except:
    HAS_GPU = False


class MetalDeadSmartGPU(MetalDeadSmart):
    """
    Metal-Dead con inteligencia avanzada + GPU MAX.
    El más potente e inteligente.
    """
    
    def __init__(self, config: MetalDeadConfig = None):
        # Inicializar base inteligente
        super().__init__(config)
        
        # Configurar GPU MAX
        if HAS_GPU:
            self.gpu_config = GPUConfig(
                precision=PrecisionMode.AUTO,
                use_flash_attention=True,
                persistent_weights=True,
                benchmark_cudnn=True,
            )
            
            # Reemplazar modelo con versión GPU
            self.model = AdvancedGPUTransformer(
                vocab_size=len(self.tokenizer),
                embed_dim=self.config.embed_dim,
                num_heads=self.config.num_heads,
                hidden_dim=self.config.hidden_dim,
                num_layers=self.config.num_layers,
                config=self.gpu_config
            )
            
            print(f"\n🔥 Metal-Dead Smart GPU MAX")
            print(f"   Precisión: {self.model.gpu.precision_name}")
            print(f"   Flash Attention: ✅")
            print(f"   Pensamiento Crítico: ✅")
        else:
            print("\n⚠️ GPU no disponible, usando CPU con inteligencia avanzada")
    
    def chat(self, message: str) -> str:
        """Chat inteligente con GPU."""
        start = time.perf_counter()
        
        # Usar el chat inteligente del padre
        response = super().chat(message)
        
        elapsed = (time.perf_counter() - start) * 1000
        
        # Agregar métricas si está en modo verbose
        if hasattr(self, 'verbose') and self.verbose:
            response += f"\n\n[⚡ {elapsed:.1f}ms | GPU: {self.model.gpu.precision_name if HAS_GPU else 'CPU'}]"
        
        return response
    
    def benchmark_intelligence(self) -> Dict:
        """Benchmark del sistema de inteligencia + GPU."""
        print("\n" + "=" * 60)
        print("   🧠 Benchmark de Inteligencia + GPU")
        print("=" * 60)
        
        results = {}
        
        # Test de pensamiento
        print("\n📊 Test de Pensamiento Crítico:")
        print("-" * 40)
        
        test_messages = [
            "¿Qué es una GPU y cómo funciona?",
            "Explícame sobre inteligencia artificial",
            "Me siento frustrado con mi código",
            "Busca información sobre transformers",
        ]
        
        times = []
        for msg in test_messages:
            start = time.perf_counter()
            thought = self.think(msg)
            elapsed = (time.perf_counter() - start) * 1000
            times.append(elapsed)
            print(f"   '{msg[:30]}...' -> {elapsed:.2f}ms (conf: {thought['confidence']:.1%})")
        
        results["thinking_avg_ms"] = sum(times) / len(times)
        print(f"\n   Promedio: {results['thinking_avg_ms']:.2f} ms")
        
        # Test de chat completo
        print("\n📊 Test de Chat Inteligente:")
        print("-" * 40)
        
        chat_times = []
        for msg in test_messages:
            start = time.perf_counter()
            _ = self.chat(msg)
            elapsed = (time.perf_counter() - start) * 1000
            chat_times.append(elapsed)
        
        results["chat_avg_ms"] = sum(chat_times) / len(chat_times)
        print(f"   Promedio: {results['chat_avg_ms']:.2f} ms")
        
        # Estadísticas de inteligencia
        print("\n📊 Estadísticas de Inteligencia:")
        print("-" * 40)
        intel_stats = self.intelligence.get_stats()
        for key, value in intel_stats.items():
            print(f"   {key}: {value}")
        
        results["intelligence_stats"] = intel_stats
        
        print("\n" + "=" * 60)
        print("   ✅ Benchmark completado")
        print("=" * 60)
        
        return results


def demo():
    """Demo de Metal-Dead Smart GPU."""
    print("\n" + "=" * 70)
    print("   🧠 Metal-Dead Smart GPU - Demo")
    print("   IA Inteligente con Pensamiento Crítico + GPU MAX")
    print("=" * 70)
    
    config = MetalDeadConfig(
        vocab_size=10000,
        embed_dim=256,
        num_heads=8,
        hidden_dim=1024,
        num_layers=4,
    )
    
    metal = MetalDeadSmartGPU(config)
    
    print("\n📝 Conversación Inteligente:")
    print("-" * 60)
    
    messages = [
        "Hola",
        "¿Qué es una GPU y para qué sirve mi RTX 3060?",
        "Me llamo Developer y me interesa la IA",
        "¿Qué sabes sobre transformers y attention?",
        "pensamiento",  # Ver último pensamiento
        "estadísticas",  # Ver stats de inteligencia
    ]
    
    for msg in messages:
        print(f"\n👤 Tú: {msg}")
        start = time.perf_counter()
        response = metal.chat(msg)
        elapsed = (time.perf_counter() - start) * 1000
        print(f"🧠 Metal-Dead: {response}")
        print(f"   ⏱️ {elapsed:.1f} ms")
    
    # Benchmark
    print("\n")
    metal.benchmark_intelligence()


if __name__ == "__main__":
    demo()
