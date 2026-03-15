"""
ADead-BIB Full Demo - Demostración Completa
============================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Demuestra todas las capacidades del sistema:
1. Compilador ADead-BIB (binarios mínimos)
2. IA local (0.19 MB RAM)
3. IA escalable con BPE (0.82 MB RAM)
4. Integración con Ollama (modelo real)
"""

import os
import sys
import time
import json
import psutil
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

# Imports del proyecto
from adead_ffi import ADeadBIB
from ai_complete import ADeadAI as BasicAI, demo as basic_demo
from ai_scalable import ScalableAI, ScalableConfig, demo as scalable_demo

# Ollama
try:
    import urllib.request
    HAS_URLLIB = True
except:
    HAS_URLLIB = False


def get_memory_mb():
    """Obtiene uso de memoria del proceso actual."""
    process = psutil.Process(os.getpid())
    return process.memory_info().rss / (1024 * 1024)


def print_header(title):
    print("\n" + "=" * 70)
    print(f"   {title}")
    print("=" * 70)


def demo_compiler():
    """Demo del compilador ADead-BIB."""
    print_header("1. COMPILADOR ADead-BIB")
    
    try:
        adead = ADeadBIB()
        print("✅ Compilador encontrado")
        
        # Compilar hello world
        print("\n📝 Compilando hello_world.adB...")
        start = time.time()
        
        examples_dir = Path(__file__).parent.parent / "examples"
        hello_file = examples_dir / "hello_world.adB"
        
        if hello_file.exists():
            exe_path = adead.compile(str(hello_file))
            compile_time = (time.time() - start) * 1000
            
            # Obtener tamaño del binario
            exe_size = os.path.getsize(exe_path)
            
            print(f"   Tiempo de compilación: {compile_time:.1f} ms")
            print(f"   Tamaño del binario: {exe_size} bytes ({exe_size/1024:.2f} KB)")
            
            # Ejecutar
            output = adead.run(exe_path)
            print(f"   Salida: {output.strip()}")
            
            return True
        else:
            print("⚠️ Archivo hello_world.adB no encontrado")
            return False
            
    except Exception as e:
        print(f"❌ Error: {e}")
        return False


def demo_ai_basic():
    """Demo de IA básica."""
    print_header("2. IA BÁSICA (0.19 MB RAM)")
    
    mem_before = get_memory_mb()
    
    try:
        from ai_complete import Tokenizer, Embeddings, MultiHeadAttention, FeedForward
        
        # Crear componentes
        tokenizer = Tokenizer(vocab_size=1000)
        embeddings = Embeddings(len(tokenizer), 64, use_float16=True)
        attention = MultiHeadAttention(64, 4, use_float16=True)
        ffn = FeedForward(64, 128, use_float16=True)
        
        mem_after = get_memory_mb()
        mem_used = mem_after - mem_before
        
        print(f"\n📊 Componentes creados:")
        print(f"   Vocabulario: {len(tokenizer)} tokens")
        print(f"   Embeddings: 64 dim (float16)")
        print(f"   Atención: 4 heads")
        print(f"   FFN: 128 hidden")
        print(f"\n💾 RAM usada: {mem_used:.2f} MB")
        
        # Tokenizar texto
        texts = [
            "Hello world",
            "Python programming",
            "Artificial intelligence",
        ]
        
        print("\n🔤 Tokenización:")
        for text in texts:
            tokens = tokenizer.encode(text)
            unk_count = tokens.count(tokenizer.UNK)
            print(f"   '{text}' → {len(tokens)} tokens, UNK: {unk_count}")
        
        return True
        
    except Exception as e:
        print(f"❌ Error: {e}")
        return False


def demo_ai_scalable():
    """Demo de IA escalable."""
    print_header("3. IA ESCALABLE CON BPE (0.82 MB RAM)")
    
    mem_before = get_memory_mb()
    
    try:
        config = ScalableConfig(
            vocab_size=5000,
            embed_dim=128,
            num_heads=8,
            hidden_dim=256,
            num_layers=2,
            temperature=0.8
        )
        
        ai = ScalableAI(config)
        
        mem_after = get_memory_mb()
        mem_used = mem_after - mem_before
        
        print(f"\n💾 RAM usada: {mem_used:.2f} MB")
        
        # Análisis
        texts = [
            "Machine learning is transforming the world",
            "Python programming for data science",
            "Inteligencia artificial y aprendizaje automático",
        ]
        
        print("\n📝 Análisis de texto:")
        for text in texts:
            stats = ai.analyze(text)
            print(f"   '{text[:40]}...'")
            print(f"      Tokens: {stats['num_tokens']}, UNK: {stats['unk_count']}")
        
        # Benchmark
        print("\n⚡ Benchmark:")
        bench = ai.benchmark(20)
        print(f"   Tiempo promedio: {bench['avg_time_ms']:.1f} ms")
        print(f"   Tokens/segundo: {bench['tokens_per_sec']:.1f}")
        
        return True
        
    except Exception as e:
        print(f"❌ Error: {e}")
        return False


def demo_ollama():
    """Demo de integración con Ollama."""
    print_header("4. INTEGRACIÓN CON OLLAMA (Modelo Real)")
    
    if not HAS_URLLIB:
        print("⚠️ urllib no disponible")
        return False
    
    try:
        # Verificar Ollama
        url = "http://localhost:11434/api/tags"
        req = urllib.request.Request(url, method='GET')
        
        with urllib.request.urlopen(req, timeout=5) as response:
            data = json.loads(response.read().decode())
            models = [m["name"] for m in data.get("models", [])]
            print(f"✅ Ollama disponible")
            print(f"   Modelos: {models}")
        
        # Generar texto
        print("\n🤖 Generando texto con TinyLlama...")
        
        prompts = [
            ("What is Python in one sentence?", 30),
            ("Explain AI in 10 words", 20),
        ]
        
        for prompt, max_tokens in prompts:
            print(f"\n   Prompt: '{prompt}'")
            
            payload = {
                "model": "tinyllama",
                "prompt": prompt,
                "stream": False,
                "options": {
                    "temperature": 0.7,
                    "num_predict": max_tokens,
                }
            }
            
            start = time.time()
            
            data = json.dumps(payload).encode('utf-8')
            req = urllib.request.Request(
                "http://localhost:11434/api/generate",
                data=data,
                headers={'Content-Type': 'application/json'},
                method='POST'
            )
            
            with urllib.request.urlopen(req, timeout=60) as response:
                result = json.loads(response.read().decode())
                text = result.get("response", "").strip()
                elapsed = time.time() - start
                
                print(f"   Respuesta: {text[:80]}...")
                print(f"   Tiempo: {elapsed:.1f}s")
        
        return True
        
    except urllib.error.URLError as e:
        print(f"⚠️ Ollama no disponible: {e}")
        return False
    except Exception as e:
        print(f"❌ Error: {e}")
        return False


def demo_comparison():
    """Comparación de rendimiento."""
    print_header("5. COMPARACIÓN DE RENDIMIENTO")
    
    print("\n📊 Resumen de capacidades:")
    print()
    print("| Componente          | RAM      | Velocidad     | Uso                    |")
    print("|---------------------|----------|---------------|------------------------|")
    print("| ADead-BIB Compiler  | ~5 MB    | <100 ms       | Binarios mínimos       |")
    print("| IA Básica           | 0.19 MB  | 15 ms/token   | Análisis rápido        |")
    print("| IA Escalable (BPE)  | 0.82 MB  | 37 ms/token   | 0% UNK, caché 93%      |")
    print("| Ollama (TinyLlama)  | ~700 MB  | 1-2 s/resp    | Generación coherente   |")
    print()
    
    print("🎯 Casos de uso recomendados:")
    print()
    print("   1. Procesamiento masivo → IA Básica (0.19 MB)")
    print("   2. Tokenización precisa → IA Escalable (0% UNK)")
    print("   3. Generación de texto → Ollama (calidad alta)")
    print("   4. Binarios pequeños → ADead-BIB (1.5 KB)")
    print()
    
    print("💡 Combinación óptima:")
    print("   ADead-BIB (pre-procesamiento) + Ollama (inferencia) + Python (orquestación)")


def main():
    """Ejecuta todas las demos."""
    print("\n" + "=" * 70)
    print("   🔥 ADead-BIB FULL DEMO")
    print("   Author: Eddi Andreé Salazar Matos")
    print("   Made with ❤️ in Peru 🇵🇪")
    print("=" * 70)
    
    mem_start = get_memory_mb()
    print(f"\n💾 RAM inicial: {mem_start:.1f} MB")
    
    results = {}
    
    # Ejecutar demos
    results["compiler"] = demo_compiler()
    results["ai_basic"] = demo_ai_basic()
    results["ai_scalable"] = demo_ai_scalable()
    results["ollama"] = demo_ollama()
    
    # Comparación
    demo_comparison()
    
    # Resumen
    print_header("RESUMEN FINAL")
    
    mem_end = get_memory_mb()
    
    print(f"\n✅ Demos completadas:")
    for name, success in results.items():
        status = "✅" if success else "❌"
        print(f"   {status} {name}")
    
    print(f"\n💾 RAM total usada: {mem_end - mem_start:.1f} MB")
    print(f"💾 RAM final del proceso: {mem_end:.1f} MB")
    
    print("\n" + "=" * 70)
    print("   Demo completada - ADead-BIB + Python + Ollama")
    print("=" * 70)


if __name__ == "__main__":
    main()
