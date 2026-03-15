"""
Metal-Dead - IA Personal para ADead-BIB
========================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Metal-Dead: Tu asistente de IA personal ultra-eficiente.
Diseñado para funcionar con ADead-BIB - Sin runtime, máximo rendimiento.

Uso rápido:
    from Metal_Dead import MetalDead
    
    metal = MetalDead()
    metal.chat("Hola")
    
    # O ejecutar directamente:
    # python -m Metal_Dead
"""

__version__ = "1.0.0"
__author__ = "Eddi Andreé Salazar Matos"
__email__ = "eddi.salazar.dev@gmail.com"

# Core
from .core.metal_dead import MetalDead, MetalDeadConfig
from .core.memory import PersistentMemory, MemoryItem
from .core.context import PersonalContext, UserProfile
from .core.tokenizer import SmartTokenizer
from .core.model import LightTransformer

# Integrations
from .integrations.gpu_compute import GPUCompute, MetalDeadGPU
from .integrations.gpu_advanced import GPUAdvanced, MetalDeadGPUMax
from .integrations.adead_accelerator import ADeadAccelerator, MetalDeadADead

# UI
from .ui.chat import MetalDeadChat
from .ui.cli import main as cli_main

__all__ = [
    # Core
    "MetalDead",
    "MetalDeadConfig",
    "PersistentMemory",
    "MemoryItem",
    "PersonalContext",
    "UserProfile",
    "SmartTokenizer",
    "LightTransformer",
    # Integrations
    "GPUCompute",
    "MetalDeadGPU",
    "GPUAdvanced",
    "MetalDeadGPUMax",
    "ADeadAccelerator",
    "MetalDeadADead",
    # UI
    "MetalDeadChat",
    "cli_main",
]


def quick_start():
    """Inicio rápido de Metal-Dead."""
    metal = MetalDead()
    metal.interactive()


def chat():
    """Inicia el chat con interfaz mejorada."""
    chat_ui = MetalDeadChat()
    chat_ui.run()
