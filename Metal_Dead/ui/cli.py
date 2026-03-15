"""
CLI para Metal-Dead
====================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪
"""

import argparse
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent.parent))


def main():
    parser = argparse.ArgumentParser(
        description="⚡ Metal-Dead - IA Personal para ADead-BIB",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Ejemplos:
  python -m Metal_Dead              # Chat estándar
  python -m Metal_Dead --gpu        # Con GPU
  python -m Metal_Dead --gpu-max    # GPU MAX (Flash Attention + BF16)
  python -m Metal_Dead --demo       # Demo del sistema
  python -m Metal_Dead --benchmark  # Benchmark de rendimiento
        """
    )
    
    parser.add_argument("--gpu", action="store_true", help="Habilitar GPU (CUDA)")
    parser.add_argument("--gpu-max", action="store_true", help="GPU MAX: Flash Attention + BF16")
    parser.add_argument("--smart", action="store_true", help="Modo inteligente (pensamiento crítico)")
    parser.add_argument("--smart-gpu", action="store_true", help="Inteligente + GPU MAX (máximo poder)")
    parser.add_argument("--jarvis", action="store_true", help="🤖 Modo JARVIS (asistente completo)")
    parser.add_argument("--jarvis-voice", action="store_true", help="🎤 JARVIS con control por voz")
    parser.add_argument("--adead", action="store_true", help="Aceleración ADead-BIB")
    parser.add_argument("--demo", action="store_true", help="Ejecutar demo")
    parser.add_argument("--benchmark", action="store_true", help="Ejecutar benchmark")
    parser.add_argument("--info", action="store_true", help="Mostrar información")
    
    args = parser.parse_args()
    
    if getattr(args, 'jarvis_voice', False):
        mode = "jarvis_voice"
    elif getattr(args, 'jarvis', False):
        mode = "jarvis"
    elif getattr(args, 'smart_gpu', False):
        mode = "smart_gpu"
    elif getattr(args, 'smart', False):
        mode = "smart"
    elif getattr(args, 'gpu_max', False):
        mode = "gpu_max"
    elif args.gpu:
        mode = "gpu"
    elif args.adead:
        mode = "adead"
    else:
        mode = "standard"
    
    if args.demo:
        run_demo(mode)
    elif args.benchmark:
        run_benchmark(mode)
    elif args.info:
        show_info()
    elif mode == "jarvis":
        from Metal_Dead.jarvis.jarvis import MetalJarvis, JarvisConfig
        config = JarvisConfig(use_voice=False)
        jarvis = MetalJarvis(config)
        jarvis.interactive()
    elif mode == "jarvis_voice":
        from Metal_Dead.jarvis.jarvis import MetalJarvis, JarvisConfig
        config = JarvisConfig(use_voice=True)
        jarvis = MetalJarvis(config)
        jarvis.voice_mode()
    else:
        from Metal_Dead.ui.chat import MetalDeadChat
        chat = MetalDeadChat(mode=mode)
        chat.run()


def run_demo(mode: str = "standard"):
    print("\n" + "=" * 60)
    print("   ⚡ DEMO: Metal-Dead para ADead-BIB")
    print("=" * 60)
    
    from Metal_Dead.core.metal_dead import MetalDead, MetalDeadConfig
    
    if mode == "smart_gpu":
        from Metal_Dead.integrations.metal_dead_smart_gpu import MetalDeadSmartGPU
        metal = MetalDeadSmartGPU()
    elif mode == "smart":
        from Metal_Dead.core.metal_dead_smart import MetalDeadSmart
        metal = MetalDeadSmart()
    elif mode == "gpu_max":
        from Metal_Dead.integrations.gpu_advanced import MetalDeadGPUMax
        metal = MetalDeadGPUMax()
    elif mode == "gpu":
        from Metal_Dead.integrations.gpu_compute import MetalDeadGPU
        metal = MetalDeadGPU()
    else:
        metal = MetalDead()
    
    print("\n📝 Simulación de Conversación:")
    print("-" * 40)
    
    messages = [
        "Hola",
        "Me llamo Developer",
        "Me gusta la programación y la IA",
        "Recuerda que estoy trabajando en ADead-BIB",
        "¿Qué sabes de mí?",
        "perfil",
        "memoria",
    ]
    
    for msg in messages:
        print(f"\n👤 Usuario: {msg}")
        start = time.perf_counter()
        response = metal.chat(msg)
        elapsed = (time.perf_counter() - start) * 1000
        print(f"⚡ Metal-Dead: {response}")
        print(f"   ⏱️ {elapsed:.1f} ms")
    
    stats = metal.get_stats()
    print("\n" + "=" * 60)
    print(f"   ✅ Demo Completada")
    print(f"   💾 RAM: {stats['ram_mb']:.2f} MB")
    print(f"   📚 Memorias: {stats['memory_count']}")
    print("=" * 60)


def run_benchmark(mode: str = "standard"):
    print("\n" + "=" * 60)
    print("   ⚡ BENCHMARK: Metal-Dead")
    print("=" * 60)
    
    from Metal_Dead.core.metal_dead import MetalDead, MetalDeadConfig
    
    config = MetalDeadConfig(vocab_size=10000, embed_dim=128, num_layers=2)
    
    if mode == "gpu_max":
        from Metal_Dead.integrations.gpu_advanced import MetalDeadGPUMax
        metal = MetalDeadGPUMax(config)
    elif mode == "gpu":
        from Metal_Dead.integrations.gpu_compute import MetalDeadGPU
        metal = MetalDeadGPU(config)
    else:
        metal = MetalDead(config)
    
    print("\n💬 Benchmark de Chat:")
    print("-" * 40)
    
    prompts = ["Hola", "¿Cómo estás?", "¿Qué puedes hacer?", "Cuéntame algo"]
    times = []
    
    for _ in range(20):
        prompt = prompts[_ % len(prompts)]
        start = time.perf_counter()
        _ = metal.chat(prompt)
        times.append((time.perf_counter() - start) * 1000)
    
    print(f"  Tiempo promedio: {np.mean(times):.1f} ms")
    print(f"  Tiempo mínimo:   {np.min(times):.1f} ms")
    print(f"  Tiempo máximo:   {np.max(times):.1f} ms")
    
    print("\n📚 Benchmark de Memoria:")
    print("-" * 40)
    
    start = time.time()
    for i in range(100):
        metal.memory.add(f"Test memory item {i}", category="general")
    add_time = time.time() - start
    
    start = time.time()
    for i in range(100):
        metal.memory.search(f"memory {i}", top_k=5)
    search_time = time.time() - start
    
    print(f"  Agregar 100 items: {add_time*1000:.1f} ms")
    print(f"  Buscar 100 veces:  {search_time*1000:.1f} ms")
    
    print("\n" + "=" * 60)
    print(f"   ✅ Benchmark Completado")
    print("=" * 60)


def show_info():
    print("""
╔══════════════════════════════════════════════════════════════╗
║                    ⚡ Metal-Dead v1.0                         ║
║              IA Personal para ADead-BIB                      ║
╠══════════════════════════════════════════════════════════════╣
║                                                              ║
║  Autor: Eddi Andreé Salazar Matos                           ║
║  Email: eddi.salazar.dev@gmail.com                          ║
║  Made with ❤️ in Peru 🇵🇪                                    ║
║                                                              ║
╠══════════════════════════════════════════════════════════════╣
║  Modos de ejecución:                                         ║
║    --gpu        GPU con CUDA                                 ║
║    --gpu-max    GPU MAX (Flash Attention + BF16)             ║
║    --adead      Aceleración ADead-BIB                        ║
║                                                              ║
║  Acciones:                                                   ║
║    --demo       Ejecutar demo                                ║
║    --benchmark  Ejecutar benchmark                           ║
║    --info       Mostrar esta información                     ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
""")


if __name__ == "__main__":
    main()
