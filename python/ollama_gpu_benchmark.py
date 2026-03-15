"""
ADead-BIB Ollama GPU Benchmark
==============================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with love in Peru

Benchmark de Ollama con diferentes modos:
- CPU Solo (100% CPU, 0% GPU)
- GPU Solo (10% CPU, 90% GPU)
- CPU + GPU (50% / 50%)
- Hibrido Optimo (10% CPU, 90% GPU)
"""

import os
import sys
import time
import json
import urllib.request
import urllib.error
from pathlib import Path
from dataclasses import dataclass
from typing import Optional, List, Dict
from enum import Enum

# Detectar hardware
try:
    import psutil
    HAS_PSUTIL = True
except ImportError:
    HAS_PSUTIL = False

try:
    import GPUtil
    HAS_GPUTIL = True
except ImportError:
    HAS_GPUTIL = False

try:
    import torch
    HAS_TORCH = True
    TORCH_CUDA = torch.cuda.is_available()
except ImportError:
    HAS_TORCH = False
    TORCH_CUDA = False


class ComputeMode(Enum):
    CPU_ONLY = "cpu_only"           # 100% CPU, 0% GPU
    GPU_ONLY = "gpu_only"           # 10% CPU, 90% GPU
    CPU_GPU_BALANCED = "balanced"   # 50% CPU, 50% GPU
    HYBRID_OPTIMAL = "hybrid"       # 10% CPU, 90% GPU optimizado


@dataclass
class BenchmarkResult:
    mode: str
    prompt: str
    response: str
    time_seconds: float
    tokens_generated: int
    tokens_per_second: float
    cpu_percent: float
    gpu_percent: float
    ram_mb: float
    vram_mb: float


class OllamaGPUBenchmark:
    """Benchmark de Ollama con diferentes modos CPU/GPU."""
    
    def __init__(self, model: str = "tinyllama"):
        self.model = model
        self.base_url = "http://localhost:11434"
        self.results: List[BenchmarkResult] = []
        
        # Detectar hardware
        self.gpu_available = self._detect_gpu()
        self.gpu_name = self._get_gpu_name()
        self.gpu_vram = self._get_gpu_vram()
    
    def _detect_gpu(self) -> bool:
        if HAS_GPUTIL:
            gpus = GPUtil.getGPUs()
            return len(gpus) > 0
        return TORCH_CUDA
    
    def _get_gpu_name(self) -> str:
        if HAS_GPUTIL:
            gpus = GPUtil.getGPUs()
            if gpus:
                return gpus[0].name
        if TORCH_CUDA:
            return torch.cuda.get_device_name(0)
        return "No GPU"
    
    def _get_gpu_vram(self) -> float:
        if HAS_GPUTIL:
            gpus = GPUtil.getGPUs()
            if gpus:
                return gpus[0].memoryTotal / 1024  # GB
        if TORCH_CUDA:
            return torch.cuda.get_device_properties(0).total_memory / 1024**3
        return 0
    
    def _get_cpu_usage(self) -> float:
        if HAS_PSUTIL:
            return psutil.cpu_percent(interval=0.1)
        return 0
    
    def _get_ram_usage(self) -> float:
        if HAS_PSUTIL:
            return psutil.Process().memory_info().rss / 1024**2
        return 0
    
    def _get_gpu_usage(self) -> tuple:
        if HAS_GPUTIL:
            gpus = GPUtil.getGPUs()
            if gpus:
                return gpus[0].load * 100, gpus[0].memoryUsed
        return 0, 0
    
    def check_ollama(self) -> bool:
        """Verifica si Ollama esta disponible."""
        try:
            req = urllib.request.Request(f"{self.base_url}/api/tags", method='GET')
            with urllib.request.urlopen(req, timeout=5) as response:
                data = json.loads(response.read().decode())
                models = [m["name"] for m in data.get("models", [])]
                return self.model in models or f"{self.model}:latest" in models
        except:
            return False
    
    def generate(self, prompt: str, mode: ComputeMode, max_tokens: int = 50) -> Optional[BenchmarkResult]:
        """Genera texto con Ollama en el modo especificado."""
        
        # Configurar opciones segun modo
        options = self._get_options_for_mode(mode)
        
        payload = {
            "model": self.model,
            "prompt": prompt,
            "stream": False,
            "options": options
        }
        
        # Medir recursos antes
        cpu_before = self._get_cpu_usage()
        ram_before = self._get_ram_usage()
        gpu_load_before, vram_before = self._get_gpu_usage()
        
        try:
            data = json.dumps(payload).encode('utf-8')
            req = urllib.request.Request(
                f"{self.base_url}/api/generate",
                data=data,
                headers={'Content-Type': 'application/json'},
                method='POST'
            )
            
            start = time.perf_counter()
            
            with urllib.request.urlopen(req, timeout=120) as response:
                result = json.loads(response.read().decode())
            
            elapsed = time.perf_counter() - start
            
            # Medir recursos despues
            cpu_after = self._get_cpu_usage()
            ram_after = self._get_ram_usage()
            gpu_load_after, vram_after = self._get_gpu_usage()
            
            response_text = result.get("response", "").strip()
            
            # Estimar tokens (aproximado)
            tokens = len(response_text.split())
            tokens_per_sec = tokens / elapsed if elapsed > 0 else 0
            
            benchmark_result = BenchmarkResult(
                mode=mode.value,
                prompt=prompt,
                response=response_text[:100] + "..." if len(response_text) > 100 else response_text,
                time_seconds=elapsed,
                tokens_generated=tokens,
                tokens_per_second=tokens_per_sec,
                cpu_percent=(cpu_before + cpu_after) / 2,
                gpu_percent=(gpu_load_before + gpu_load_after) / 2,
                ram_mb=(ram_before + ram_after) / 2,
                vram_mb=(vram_before + vram_after) / 2
            )
            
            self.results.append(benchmark_result)
            return benchmark_result
            
        except Exception as e:
            print(f"Error: {e}")
            return None
    
    def _get_options_for_mode(self, mode: ComputeMode) -> dict:
        """Obtiene opciones de Ollama segun el modo."""
        base_options = {
            "temperature": 0.7,
            "num_predict": 50,
        }
        
        if mode == ComputeMode.CPU_ONLY:
            # Forzar CPU: num_gpu = 0
            base_options["num_gpu"] = 0
            base_options["num_thread"] = os.cpu_count() or 4
        
        elif mode == ComputeMode.GPU_ONLY:
            # Maximo GPU: todas las capas en GPU
            base_options["num_gpu"] = 999  # Todas las capas
            base_options["num_thread"] = 2  # Minimo CPU
        
        elif mode == ComputeMode.CPU_GPU_BALANCED:
            # Balanceado: mitad de capas en GPU
            base_options["num_gpu"] = 20  # Algunas capas
            base_options["num_thread"] = (os.cpu_count() or 4) // 2
        
        elif mode == ComputeMode.HYBRID_OPTIMAL:
            # Hibrido optimo: 90% GPU, 10% CPU
            base_options["num_gpu"] = 999
            base_options["num_thread"] = max(2, (os.cpu_count() or 4) // 4)
        
        return base_options
    
    def run_benchmark(self, prompts: List[str] = None):
        """Ejecuta benchmark completo."""
        if prompts is None:
            prompts = [
                "What is Python?",
                "Explain AI in simple terms",
                "Write a haiku about coding",
            ]
        
        print("=" * 80)
        print("   ADead-BIB Ollama GPU Benchmark")
        print("   Author: Eddi Andree Salazar Matos")
        print("=" * 80)
        
        print(f"\nHardware:")
        print(f"  GPU: {self.gpu_name}")
        print(f"  VRAM: {self.gpu_vram:.1f} GB")
        print(f"  GPU disponible: {'Si' if self.gpu_available else 'No'}")
        print(f"  Modelo: {self.model}")
        
        if not self.check_ollama():
            print("\nERROR: Ollama no disponible o modelo no encontrado")
            print("Ejecuta: ollama serve")
            print(f"Y luego: ollama pull {self.model}")
            return
        
        print("\n" + "=" * 80)
        print("   Ejecutando benchmarks...")
        print("=" * 80)
        
        modes = [
            ComputeMode.CPU_ONLY,
            ComputeMode.GPU_ONLY,
            ComputeMode.CPU_GPU_BALANCED,
            ComputeMode.HYBRID_OPTIMAL,
        ]
        
        for mode in modes:
            print(f"\n--- Modo: {mode.value.upper()} ---")
            
            mode_results = []
            for prompt in prompts:
                print(f"  Prompt: '{prompt[:30]}...'", end=" ")
                result = self.generate(prompt, mode)
                if result:
                    print(f"-> {result.time_seconds:.2f}s, {result.tokens_per_second:.1f} tok/s")
                    mode_results.append(result)
                else:
                    print("-> ERROR")
            
            if mode_results:
                avg_time = sum(r.time_seconds for r in mode_results) / len(mode_results)
                avg_tps = sum(r.tokens_per_second for r in mode_results) / len(mode_results)
                print(f"  Promedio: {avg_time:.2f}s, {avg_tps:.1f} tokens/s")
        
        self.print_summary()
    
    def print_summary(self):
        """Imprime resumen de resultados."""
        print("\n" + "=" * 80)
        print("   RESUMEN DE RESULTADOS")
        print("=" * 80)
        
        # Agrupar por modo
        modes_data = {}
        for r in self.results:
            if r.mode not in modes_data:
                modes_data[r.mode] = []
            modes_data[r.mode].append(r)
        
        print(f"\n{'Modo':<20} {'Tiempo (s)':<12} {'Tokens/s':<12} {'CPU %':<10} {'GPU %':<10}")
        print("-" * 70)
        
        for mode, results in modes_data.items():
            avg_time = sum(r.time_seconds for r in results) / len(results)
            avg_tps = sum(r.tokens_per_second for r in results) / len(results)
            avg_cpu = sum(r.cpu_percent for r in results) / len(results)
            avg_gpu = sum(r.gpu_percent for r in results) / len(results)
            
            print(f"{mode:<20} {avg_time:<12.2f} {avg_tps:<12.1f} {avg_cpu:<10.1f} {avg_gpu:<10.1f}")
        
        print("-" * 70)
        
        # Comparacion
        if "cpu_only" in modes_data and "gpu_only" in modes_data:
            cpu_time = sum(r.time_seconds for r in modes_data["cpu_only"]) / len(modes_data["cpu_only"])
            gpu_time = sum(r.time_seconds for r in modes_data["gpu_only"]) / len(modes_data["gpu_only"])
            speedup = cpu_time / gpu_time if gpu_time > 0 else 1
            
            print(f"\nSpeedup GPU vs CPU: {speedup:.1f}x")
        
        print("\n" + "=" * 80)
        print("   RECOMENDACIONES")
        print("=" * 80)
        print("""
  CPU Solo (100% CPU):
    - Usar cuando: No hay GPU o GPU ocupada
    - Velocidad: Lenta (10-20 tokens/s)
    
  GPU Solo (90% GPU):
    - Usar cuando: Maximo rendimiento necesario
    - Velocidad: Rapida (50-100 tokens/s)
    - Requiere: VRAM suficiente
    
  CPU + GPU (50/50):
    - Usar cuando: VRAM limitada
    - Velocidad: Media (30-50 tokens/s)
    
  Hibrido Optimo (10% CPU, 90% GPU):
    - Usar cuando: Produccion
    - Velocidad: Optima (40-80 tokens/s)
    - Mejor balance recursos/velocidad
""")
        
        print("=" * 80)
        print("   Benchmark completado")
        print("=" * 80)


def demo():
    """Demo del benchmark."""
    benchmark = OllamaGPUBenchmark(model="tinyllama")
    benchmark.run_benchmark()


if __name__ == "__main__":
    demo()
