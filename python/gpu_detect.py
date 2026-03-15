"""
ADead-BIB GPU Detection & Benchmark
====================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Detecta hardware disponible y compara rendimiento:
- CPU: NumPy
- GPU: CuPy/PyTorch (si disponible)
- Híbrido: CPU + GPU combinado
"""

import os
import sys
import time
import platform
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass

# NumPy siempre disponible
import numpy as np

# Intentar importar psutil para info de sistema
try:
    import psutil
    HAS_PSUTIL = True
except ImportError:
    HAS_PSUTIL = False

# Intentar importar GPUtil para info de GPU
try:
    import GPUtil
    HAS_GPUTIL = True
except ImportError:
    HAS_GPUTIL = False

# Intentar importar CuPy para GPU NVIDIA
try:
    import cupy as cp
    HAS_CUPY = True
    CUPY_VERSION = cp.__version__
except ImportError:
    HAS_CUPY = False
    CUPY_VERSION = None

# Intentar importar PyTorch
try:
    import torch
    HAS_TORCH = True
    TORCH_VERSION = torch.__version__
    TORCH_CUDA = torch.cuda.is_available()
    TORCH_CUDA_VERSION = torch.version.cuda if TORCH_CUDA else None
except ImportError:
    HAS_TORCH = False
    TORCH_VERSION = None
    TORCH_CUDA = False
    TORCH_CUDA_VERSION = None


@dataclass
class CPUInfo:
    """Información del CPU."""
    name: str
    cores_physical: int
    cores_logical: int
    frequency_mhz: float
    architecture: str


@dataclass
class RAMInfo:
    """Información de RAM."""
    total_gb: float
    available_gb: float
    used_percent: float


@dataclass
class GPUInfo:
    """Información de GPU."""
    available: bool
    name: str
    vram_total_gb: float
    vram_free_gb: float
    driver_version: str
    cuda_version: str
    backend: str  # "cupy", "torch", "none"


@dataclass
class HardwareInfo:
    """Información completa del hardware."""
    cpu: CPUInfo
    ram: RAMInfo
    gpu: GPUInfo
    recommended_mode: str  # "cpu", "gpu", "hybrid"


def get_cpu_info() -> CPUInfo:
    """Obtiene información del CPU."""
    try:
        import cpuinfo
        info = cpuinfo.get_cpu_info()
        name = info.get('brand_raw', 'Unknown CPU')
    except:
        name = platform.processor() or 'Unknown CPU'
    
    cores_physical = psutil.cpu_count(logical=False) if HAS_PSUTIL else os.cpu_count() or 1
    cores_logical = psutil.cpu_count(logical=True) if HAS_PSUTIL else os.cpu_count() or 1
    
    try:
        freq = psutil.cpu_freq().current if HAS_PSUTIL else 0
    except:
        freq = 0
    
    return CPUInfo(
        name=name,
        cores_physical=cores_physical,
        cores_logical=cores_logical,
        frequency_mhz=freq,
        architecture=platform.machine()
    )


def get_ram_info() -> RAMInfo:
    """Obtiene información de RAM."""
    if HAS_PSUTIL:
        mem = psutil.virtual_memory()
        return RAMInfo(
            total_gb=mem.total / (1024**3),
            available_gb=mem.available / (1024**3),
            used_percent=mem.percent
        )
    return RAMInfo(total_gb=0, available_gb=0, used_percent=0)


def get_gpu_info() -> GPUInfo:
    """Obtiene información de GPU."""
    # Intentar con PyTorch CUDA
    if HAS_TORCH and TORCH_CUDA:
        try:
            gpu_name = torch.cuda.get_device_name(0)
            props = torch.cuda.get_device_properties(0)
            vram_total = props.total_memory / (1024**3)
            vram_free = (props.total_memory - torch.cuda.memory_allocated(0)) / (1024**3)
            
            return GPUInfo(
                available=True,
                name=gpu_name,
                vram_total_gb=vram_total,
                vram_free_gb=vram_free,
                driver_version="N/A",
                cuda_version=TORCH_CUDA_VERSION or "N/A",
                backend="torch"
            )
        except Exception as e:
            pass
    
    # Intentar con CuPy
    if HAS_CUPY:
        try:
            device = cp.cuda.Device(0)
            mem_info = device.mem_info
            vram_free = mem_info[0] / (1024**3)
            vram_total = mem_info[1] / (1024**3)
            
            return GPUInfo(
                available=True,
                name=f"CUDA Device {device.id}",
                vram_total_gb=vram_total,
                vram_free_gb=vram_free,
                driver_version="N/A",
                cuda_version=str(cp.cuda.runtime.runtimeGetVersion()),
                backend="cupy"
            )
        except Exception as e:
            pass
    
    # Intentar con GPUtil
    if HAS_GPUTIL:
        try:
            gpus = GPUtil.getGPUs()
            if gpus:
                gpu = gpus[0]
                return GPUInfo(
                    available=True,
                    name=gpu.name,
                    vram_total_gb=gpu.memoryTotal / 1024,
                    vram_free_gb=gpu.memoryFree / 1024,
                    driver_version=gpu.driver,
                    cuda_version="N/A",
                    backend="gputil"
                )
        except:
            pass
    
    # No GPU disponible
    return GPUInfo(
        available=False,
        name="No GPU detected",
        vram_total_gb=0,
        vram_free_gb=0,
        driver_version="N/A",
        cuda_version="N/A",
        backend="none"
    )


def detect_hardware() -> HardwareInfo:
    """Detecta todo el hardware disponible."""
    cpu = get_cpu_info()
    ram = get_ram_info()
    gpu = get_gpu_info()
    
    # Determinar modo recomendado
    if gpu.available:
        if gpu.vram_free_gb >= 4:
            recommended = "gpu"
        elif gpu.vram_free_gb >= 2:
            recommended = "hybrid"
        else:
            recommended = "cpu"
    else:
        recommended = "cpu"
    
    return HardwareInfo(
        cpu=cpu,
        ram=ram,
        gpu=gpu,
        recommended_mode=recommended
    )


def print_hardware_info(info: HardwareInfo):
    """Imprime información del hardware."""
    print("\n" + "=" * 70)
    print("   🖥️  DETECCIÓN DE HARDWARE")
    print("=" * 70)
    
    print("\n📊 CPU:")
    print(f"   Modelo: {info.cpu.name}")
    print(f"   Cores: {info.cpu.cores_physical} físicos, {info.cpu.cores_logical} lógicos")
    print(f"   Frecuencia: {info.cpu.frequency_mhz:.0f} MHz")
    print(f"   Arquitectura: {info.cpu.architecture}")
    
    print("\n💾 RAM:")
    print(f"   Total: {info.ram.total_gb:.1f} GB")
    print(f"   Disponible: {info.ram.available_gb:.1f} GB")
    print(f"   En uso: {info.ram.used_percent:.1f}%")
    
    print("\n🎮 GPU:")
    if info.gpu.available:
        print(f"   ✅ GPU detectada")
        print(f"   Modelo: {info.gpu.name}")
        print(f"   VRAM Total: {info.gpu.vram_total_gb:.1f} GB")
        print(f"   VRAM Libre: {info.gpu.vram_free_gb:.1f} GB")
        print(f"   CUDA: {info.gpu.cuda_version}")
        print(f"   Backend: {info.gpu.backend}")
    else:
        print(f"   ❌ No se detectó GPU compatible")
        print(f"   Instalar: pip install torch cupy-cuda12x GPUtil")
    
    print("\n🎯 Modo Recomendado:")
    modes = {
        "cpu": "CPU (NumPy) - Compatible con todo",
        "gpu": "GPU (CUDA) - Máximo rendimiento",
        "hybrid": "Híbrido (CPU + GPU) - Balanceado"
    }
    print(f"   {modes.get(info.recommended_mode, 'cpu')}")


# =============================================================================
# BENCHMARK
# =============================================================================

def benchmark_matmul_cpu(size: int, iterations: int = 10) -> float:
    """Benchmark de multiplicación de matrices en CPU."""
    a = np.random.randn(size, size).astype(np.float32)
    b = np.random.randn(size, size).astype(np.float32)
    
    # Warmup
    _ = np.matmul(a, b)
    
    # Benchmark
    start = time.perf_counter()
    for _ in range(iterations):
        _ = np.matmul(a, b)
    elapsed = time.perf_counter() - start
    
    return (elapsed / iterations) * 1000  # ms


def benchmark_matmul_gpu_torch(size: int, iterations: int = 10) -> Optional[float]:
    """Benchmark de multiplicación de matrices en GPU con PyTorch."""
    if not (HAS_TORCH and TORCH_CUDA):
        return None
    
    device = torch.device("cuda")
    a = torch.randn(size, size, device=device, dtype=torch.float32)
    b = torch.randn(size, size, device=device, dtype=torch.float32)
    
    # Warmup
    torch.cuda.synchronize()
    _ = torch.matmul(a, b)
    torch.cuda.synchronize()
    
    # Benchmark
    start = time.perf_counter()
    for _ in range(iterations):
        _ = torch.matmul(a, b)
    torch.cuda.synchronize()
    elapsed = time.perf_counter() - start
    
    return (elapsed / iterations) * 1000  # ms


def benchmark_matmul_gpu_cupy(size: int, iterations: int = 10) -> Optional[float]:
    """Benchmark de multiplicación de matrices en GPU con CuPy."""
    if not HAS_CUPY:
        return None
    
    a = cp.random.randn(size, size, dtype=cp.float32)
    b = cp.random.randn(size, size, dtype=cp.float32)
    
    # Warmup
    cp.cuda.Stream.null.synchronize()
    _ = cp.matmul(a, b)
    cp.cuda.Stream.null.synchronize()
    
    # Benchmark
    start = time.perf_counter()
    for _ in range(iterations):
        _ = cp.matmul(a, b)
    cp.cuda.Stream.null.synchronize()
    elapsed = time.perf_counter() - start
    
    return (elapsed / iterations) * 1000  # ms


def run_benchmark(sizes: List[int] = None) -> Dict:
    """Ejecuta benchmark completo."""
    if sizes is None:
        sizes = [256, 512, 1024, 2048]
    
    results = {
        "sizes": sizes,
        "cpu": [],
        "gpu_torch": [],
        "gpu_cupy": [],
        "speedup_torch": [],
        "speedup_cupy": [],
    }
    
    print("\n" + "=" * 70)
    print("   ⚡ BENCHMARK: CPU vs GPU")
    print("=" * 70)
    
    print("\n📊 Multiplicación de Matrices (MatMul)")
    print("-" * 70)
    print(f"{'Tamaño':<12} {'CPU (ms)':<12} {'GPU Torch':<12} {'GPU CuPy':<12} {'Speedup':<12}")
    print("-" * 70)
    
    for size in sizes:
        # CPU
        cpu_time = benchmark_matmul_cpu(size)
        results["cpu"].append(cpu_time)
        
        # GPU PyTorch
        gpu_torch_time = benchmark_matmul_gpu_torch(size)
        results["gpu_torch"].append(gpu_torch_time)
        
        # GPU CuPy
        gpu_cupy_time = benchmark_matmul_gpu_cupy(size)
        results["gpu_cupy"].append(gpu_cupy_time)
        
        # Speedup
        if gpu_torch_time:
            speedup_torch = cpu_time / gpu_torch_time
            results["speedup_torch"].append(speedup_torch)
        else:
            speedup_torch = None
            results["speedup_torch"].append(None)
        
        if gpu_cupy_time:
            speedup_cupy = cpu_time / gpu_cupy_time
            results["speedup_cupy"].append(speedup_cupy)
        else:
            speedup_cupy = None
            results["speedup_cupy"].append(None)
        
        # Imprimir
        gpu_torch_str = f"{gpu_torch_time:.2f}" if gpu_torch_time else "N/A"
        gpu_cupy_str = f"{gpu_cupy_time:.2f}" if gpu_cupy_time else "N/A"
        
        best_speedup = max(filter(None, [speedup_torch, speedup_cupy]), default=1)
        speedup_str = f"{best_speedup:.1f}x" if best_speedup > 1 else "1x"
        
        print(f"{size}x{size:<6} {cpu_time:<12.2f} {gpu_torch_str:<12} {gpu_cupy_str:<12} {speedup_str:<12}")
    
    print("-" * 70)
    
    return results


def print_recommendations(info: HardwareInfo, benchmark: Dict):
    """Imprime recomendaciones basadas en hardware y benchmark."""
    print("\n" + "=" * 70)
    print("   💡 RECOMENDACIONES")
    print("=" * 70)
    
    has_gpu = info.gpu.available
    
    if has_gpu:
        avg_speedup = np.mean([s for s in benchmark["speedup_torch"] + benchmark["speedup_cupy"] if s])
        
        print(f"\n✅ GPU detectada: {info.gpu.name}")
        print(f"   VRAM: {info.gpu.vram_total_gb:.1f} GB")
        print(f"   Speedup promedio: {avg_speedup:.1f}x")
        
        print("\n🎯 Configuración óptima para tu sistema:")
        
        if info.gpu.vram_free_gb >= 8:
            print("   • Modo: GPU (máximo rendimiento)")
            print("   • Batch size: Grande (512-1024)")
            print("   • Modelo: Puedes usar modelos grandes")
        elif info.gpu.vram_free_gb >= 4:
            print("   • Modo: GPU o Híbrido")
            print("   • Batch size: Medio (128-256)")
            print("   • Modelo: Modelos medianos")
        else:
            print("   • Modo: Híbrido (CPU + GPU)")
            print("   • Batch size: Pequeño (32-64)")
            print("   • Modelo: Modelos pequeños")
    else:
        print("\n⚠️ No se detectó GPU compatible")
        print("\n🎯 Para habilitar GPU:")
        print("   1. NVIDIA: pip install torch cupy-cuda12x")
        print("   2. AMD: pip install torch-rocm")
        print("   3. Intel: pip install intel-extension-for-pytorch")
        
        print("\n📊 Rendimiento actual (CPU only):")
        print(f"   • RAM: {info.ram.total_gb:.1f} GB")
        print(f"   • Cores: {info.cpu.cores_logical}")
        print("   • Modo: CPU con NumPy")


def demo():
    """Demo completa de detección y benchmark."""
    print("\n" + "=" * 70)
    print("   🔥 ADead-BIB GPU Detection & Benchmark")
    print("   Author: Eddi Andreé Salazar Matos")
    print("   Made with ❤️ in Peru 🇵🇪")
    print("=" * 70)
    
    # Detectar hardware
    info = detect_hardware()
    print_hardware_info(info)
    
    # Ejecutar benchmark
    benchmark = run_benchmark([256, 512, 1024])
    
    # Recomendaciones
    print_recommendations(info, benchmark)
    
    print("\n" + "=" * 70)
    print("   Demo completada")
    print("=" * 70)
    
    return info, benchmark


if __name__ == "__main__":
    demo()
