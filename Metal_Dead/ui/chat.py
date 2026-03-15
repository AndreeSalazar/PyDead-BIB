"""
Chat UI para Metal-Dead
========================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪
"""

import sys
import time
from pathlib import Path
from typing import Optional

try:
    from colorama import init, Fore, Style
    init()
    HAS_COLOR = True
except ImportError:
    HAS_COLOR = False
    class Fore:
        CYAN = YELLOW = GREEN = RED = MAGENTA = BLUE = WHITE = ""
    class Style:
        RESET_ALL = BRIGHT = ""

sys.path.insert(0, str(Path(__file__).parent.parent))
from Metal_Dead.core.metal_dead import MetalDead, MetalDeadConfig


class MetalDeadChat:
    """Interfaz de chat mejorada para Metal-Dead."""
    
    def __init__(self, mode: str = "standard", config: MetalDeadConfig = None):
        self.mode = mode
        self.config = config or MetalDeadConfig(
            vocab_size=10000,
            embed_dim=128,
            num_layers=2,
            hidden_dim=256,
            temperature=0.7,
        )
        
        print(f"{Fore.YELLOW}⚡ Inicializando Metal-Dead (modo: {mode})...{Style.RESET_ALL}")
        
        if mode == "smart_gpu":
            from Metal_Dead.integrations.metal_dead_smart_gpu import MetalDeadSmartGPU
            self.metal = MetalDeadSmartGPU(self.config)
        elif mode == "smart":
            from Metal_Dead.core.metal_dead_smart import MetalDeadSmart
            self.metal = MetalDeadSmart(self.config)
        elif mode == "gpu_max":
            from Metal_Dead.integrations.gpu_advanced import MetalDeadGPUMax
            self.metal = MetalDeadGPUMax(self.config)
        elif mode == "gpu":
            from Metal_Dead.integrations.gpu_compute import MetalDeadGPU
            self.metal = MetalDeadGPU(self.config)
        elif mode == "adead":
            from Metal_Dead.integrations.adead_accelerator import MetalDeadADead
            self.metal = MetalDeadADead(self.config)
        else:
            self.metal = MetalDead(self.config)
    
    def _print_header(self):
        print(f"\n{Fore.CYAN}{'='*60}{Style.RESET_ALL}")
        print(f"{Fore.CYAN}   ⚡ Metal-Dead - Chat Interactivo{Style.RESET_ALL}")
        print(f"{Fore.CYAN}   Tu IA Personal para ADead-BIB{Style.RESET_ALL}")
        print(f"{Fore.CYAN}{'='*60}{Style.RESET_ALL}")
        print(f"\n{Fore.YELLOW}Comandos:{Style.RESET_ALL}")
        print(f"  {Fore.GREEN}/ayuda{Style.RESET_ALL}   - Mostrar ayuda")
        print(f"  {Fore.GREEN}/memoria{Style.RESET_ALL} - Ver memorias")
        print(f"  {Fore.GREEN}/perfil{Style.RESET_ALL}  - Ver tu perfil")
        print(f"  {Fore.GREEN}/salir{Style.RESET_ALL}   - Salir")
        print(f"\n{Fore.CYAN}{'='*60}{Style.RESET_ALL}\n")
    
    def run(self):
        self._print_header()
        
        greeting = self.metal.context.get_greeting()
        print(f"{Fore.CYAN}⚡ Metal-Dead:{Style.RESET_ALL} {greeting}\n")
        
        while True:
            try:
                user_input = input(f"{Fore.GREEN}Tú:{Style.RESET_ALL} ").strip()
                
                if not user_input:
                    continue
                
                if user_input.startswith("/"):
                    cmd = user_input[1:].lower()
                    if cmd in ["salir", "exit", "quit", "q"]:
                        print(f"\n{Fore.CYAN}⚡:{Style.RESET_ALL} ¡Hasta luego! 👋")
                        break
                    elif cmd in ["ayuda", "help", "?"]:
                        print(f"\n{Fore.CYAN}⚡:{Style.RESET_ALL} {self.metal._get_help()}\n")
                        continue
                    elif cmd in ["memoria", "memorias", "memory"]:
                        print(f"\n{Fore.CYAN}⚡:{Style.RESET_ALL} {self.metal._get_memory_stats()}\n")
                        continue
                    elif cmd in ["perfil", "profile"]:
                        print(f"\n{Fore.CYAN}⚡:{Style.RESET_ALL} {self.metal.context.get_summary()}\n")
                        continue
                
                start = time.perf_counter()
                response = self.metal.chat(user_input)
                elapsed = (time.perf_counter() - start) * 1000
                
                print(f"\n{Fore.CYAN}⚡ Metal-Dead:{Style.RESET_ALL} {response}")
                print(f"{Fore.MAGENTA}   [{elapsed:.1f}ms]{Style.RESET_ALL}\n")
                
            except KeyboardInterrupt:
                print(f"\n\n{Fore.CYAN}⚡:{Style.RESET_ALL} ¡Hasta luego! 👋")
                break
            except Exception as e:
                print(f"\n{Fore.RED}⚠️ Error: {e}{Style.RESET_ALL}\n")
