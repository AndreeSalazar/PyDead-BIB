"""
IA-Personal Chat - Interfaz de Chat Mejorada
=============================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Interfaz de chat interactiva con:
- Colores y formato mejorado
- Comandos avanzados
- Historial de sesión
- Exportación de conversaciones
- Integración completa con ADead-BIB

Uso:
    python ia_personal_chat.py              # Chat normal
    python ia_personal_chat.py --turbo      # Modo turbo (más rápido)
    python ia_personal_chat.py --export     # Exportar historial
"""

import os
import sys
import time
import json
from pathlib import Path
from datetime import datetime
from typing import Optional, List, Dict

sys.path.insert(0, str(Path(__file__).parent))

# Intentar importar colorama para colores en Windows
try:
    from colorama import init, Fore, Back, Style
    init()
    HAS_COLORS = True
except ImportError:
    HAS_COLORS = False
    class Fore:
        RED = GREEN = YELLOW = BLUE = MAGENTA = CYAN = WHITE = RESET = ""
    class Style:
        BRIGHT = DIM = RESET_ALL = ""

from ia_personal import IAPersonal, IAPersonalConfig
from ia_personal_adead import IAPersonalADead


# =============================================================================
# UTILIDADES DE FORMATO
# =============================================================================

def clear_screen():
    """Limpia la pantalla."""
    os.system('cls' if os.name == 'nt' else 'clear')


def print_header():
    """Imprime el encabezado del chat."""
    print(f"""
{Fore.CYAN}╔══════════════════════════════════════════════════════════════╗
║{Fore.WHITE}                    🤖 IA-Personal Chat                        {Fore.CYAN}║
║{Fore.YELLOW}              Sistema de IA Personal para ADead-BIB            {Fore.CYAN}║
║{Fore.GREEN}                   Made with ❤️ in Peru 🇵🇪                      {Fore.CYAN}║
╚══════════════════════════════════════════════════════════════╝{Style.RESET_ALL}
""")


def print_help():
    """Imprime ayuda de comandos."""
    print(f"""
{Fore.CYAN}╔══════════════════════════════════════════════════════════════╗
║{Fore.WHITE}                      📚 COMANDOS                              {Fore.CYAN}║
╠══════════════════════════════════════════════════════════════╣
║{Fore.GREEN} /ayuda, /help     {Fore.WHITE}│ Muestra este mensaje                    {Fore.CYAN}║
║{Fore.GREEN} /limpiar, /clear  {Fore.WHITE}│ Limpia la pantalla                      {Fore.CYAN}║
║{Fore.GREEN} /memoria          {Fore.WHITE}│ Muestra estadísticas de memoria         {Fore.CYAN}║
║{Fore.GREEN} /perfil           {Fore.WHITE}│ Muestra tu perfil                       {Fore.CYAN}║
║{Fore.GREEN} /buscar [texto]   {Fore.WHITE}│ Busca en las memorias                   {Fore.CYAN}║
║{Fore.GREEN} /exportar         {Fore.WHITE}│ Exporta la conversación                 {Fore.CYAN}║
║{Fore.GREEN} /stats            {Fore.WHITE}│ Estadísticas del sistema                {Fore.CYAN}║
║{Fore.GREEN} /reset            {Fore.WHITE}│ Reinicia la conversación                {Fore.CYAN}║
║{Fore.GREEN} /salir, /exit     {Fore.WHITE}│ Termina el chat                         {Fore.CYAN}║
╠══════════════════════════════════════════════════════════════╣
║{Fore.YELLOW} 💡 Tips:                                                      {Fore.CYAN}║
║{Fore.WHITE}  • "me llamo [nombre]" - Aprendo tu nombre                    {Fore.CYAN}║
║{Fore.WHITE}  • "me gusta [algo]" - Aprendo tus intereses                  {Fore.CYAN}║
║{Fore.WHITE}  • "recuerda que [algo]" - Guardo información                 {Fore.CYAN}║
╚══════════════════════════════════════════════════════════════╝{Style.RESET_ALL}
""")


def format_user_message(msg: str) -> str:
    """Formatea mensaje del usuario."""
    return f"{Fore.GREEN}👤 Tú:{Style.RESET_ALL} {msg}"


def format_ai_message(msg: str) -> str:
    """Formatea mensaje de la IA."""
    return f"{Fore.CYAN}🤖 IA:{Style.RESET_ALL} {msg}"


def format_system_message(msg: str) -> str:
    """Formatea mensaje del sistema."""
    return f"{Fore.YELLOW}⚙️  {msg}{Style.RESET_ALL}"


def format_error_message(msg: str) -> str:
    """Formatea mensaje de error."""
    return f"{Fore.RED}❌ {msg}{Style.RESET_ALL}"


def format_success_message(msg: str) -> str:
    """Formatea mensaje de éxito."""
    return f"{Fore.GREEN}✅ {msg}{Style.RESET_ALL}"


# =============================================================================
# CLASE PRINCIPAL DEL CHAT
# =============================================================================

class IAPersonalChat:
    """Interfaz de chat mejorada para IA-Personal."""
    
    def __init__(self, use_acceleration: bool = True, turbo_mode: bool = False):
        self.turbo_mode = turbo_mode
        self.session_start = datetime.now()
        self.message_count = 0
        self.session_history: List[Dict] = []
        
        # Configuración
        if turbo_mode:
            config = IAPersonalConfig(
                vocab_size=5000,
                embed_dim=64,
                num_layers=1,
                hidden_dim=128,
                temperature=0.9,
            )
        else:
            config = IAPersonalConfig(
                vocab_size=10000,
                embed_dim=128,
                num_layers=2,
                hidden_dim=256,
                temperature=0.7,
            )
        
        # Crear IA
        print(format_system_message("Inicializando IA-Personal..."))
        
        if use_acceleration:
            try:
                self.ia = IAPersonalADead(config)
            except Exception as e:
                print(format_system_message(f"Aceleración no disponible: {e}"))
                self.ia = IAPersonal(config)
        else:
            self.ia = IAPersonal(config)
        
        # Directorio de exportación
        self.export_dir = Path(__file__).parent / "ia_personal_data" / "exports"
        self.export_dir.mkdir(parents=True, exist_ok=True)
    
    def process_command(self, command: str) -> Optional[str]:
        """Procesa un comando especial."""
        cmd = command.lower().strip()
        
        if cmd in ["/ayuda", "/help", "/?", "/h"]:
            print_help()
            return None
        
        elif cmd in ["/limpiar", "/clear", "/cls"]:
            clear_screen()
            print_header()
            return None
        
        elif cmd in ["/memoria", "/memory", "/mem"]:
            stats = self.ia.memory.stats()
            output = [
                f"\n{Fore.CYAN}📚 Estadísticas de Memoria:{Style.RESET_ALL}",
                f"  Total: {stats['total_memories']} memorias",
                f"  Accesos: {stats['total_accesses']}",
                "  Por categoría:"
            ]
            for cat, count in stats['categories'].items():
                if count > 0:
                    output.append(f"    • {cat}: {count}")
            return "\n".join(output)
        
        elif cmd in ["/perfil", "/profile"]:
            p = self.ia.context.profile
            output = [
                f"\n{Fore.CYAN}👤 Tu Perfil:{Style.RESET_ALL}",
                f"  Nombre: {p.name}",
                f"  Interacciones: {p.interaction_count}",
            ]
            if p.interests:
                output.append(f"  Intereses: {', '.join(p.interests)}")
            return "\n".join(output)
        
        elif cmd.startswith("/buscar ") or cmd.startswith("/search "):
            query = command[8:].strip()
            results = self.ia.memory.search(query, top_k=5)
            if not results:
                return f"No encontré nada sobre '{query}'"
            output = [f"\n{Fore.CYAN}🔍 Resultados para '{query}':{Style.RESET_ALL}"]
            for i, mem in enumerate(results, 1):
                output.append(f"  {i}. {mem.content[:80]}...")
            return "\n".join(output)
        
        elif cmd in ["/exportar", "/export"]:
            return self.export_conversation()
        
        elif cmd in ["/stats", "/estadisticas"]:
            return self.get_session_stats()
        
        elif cmd in ["/reset", "/reiniciar"]:
            self.ia.conversation_history.clear()
            self.session_history.clear()
            self.message_count = 0
            return format_success_message("Conversación reiniciada")
        
        elif cmd in ["/salir", "/exit", "/quit", "/q"]:
            return "EXIT"
        
        return None  # No es un comando
    
    def export_conversation(self) -> str:
        """Exporta la conversación actual."""
        if not self.session_history:
            return format_error_message("No hay conversación para exportar")
        
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = self.export_dir / f"chat_{timestamp}.json"
        
        export_data = {
            "session_start": self.session_start.isoformat(),
            "session_end": datetime.now().isoformat(),
            "message_count": self.message_count,
            "user_profile": self.ia.context.profile.to_dict(),
            "messages": self.session_history,
        }
        
        with open(filename, 'w', encoding='utf-8') as f:
            json.dump(export_data, f, ensure_ascii=False, indent=2)
        
        return format_success_message(f"Conversación exportada a: {filename}")
    
    def get_session_stats(self) -> str:
        """Obtiene estadísticas de la sesión."""
        duration = datetime.now() - self.session_start
        minutes = duration.total_seconds() / 60
        
        output = [
            f"\n{Fore.CYAN}📊 Estadísticas de Sesión:{Style.RESET_ALL}",
            f"  Duración: {minutes:.1f} minutos",
            f"  Mensajes: {self.message_count}",
            f"  RAM modelo: {self.ia.model.ram_mb:.2f} MB",
            f"  Vocabulario: {len(self.ia.tokenizer)} tokens",
            f"  Memorias: {len(self.ia.memory.memories)}",
        ]
        
        if hasattr(self.ia, 'accelerator'):
            stats = self.ia.get_acceleration_stats()
            output.append(f"  Acelerador: {'Activo' if stats['compiler_available'] else 'Inactivo'}")
            output.append(f"  Cache hits: {stats['cache_hits']}")
        
        return "\n".join(output)
    
    def chat(self, message: str) -> str:
        """Procesa un mensaje y retorna la respuesta."""
        # Verificar si es un comando
        if message.startswith("/"):
            result = self.process_command(message)
            if result == "EXIT":
                return "EXIT"
            if result is not None:
                return result
        
        # Procesar mensaje normal
        self.message_count += 1
        
        start_time = time.time()
        response = self.ia.chat(message)
        elapsed = (time.time() - start_time) * 1000
        
        # Guardar en historial de sesión
        self.session_history.append({
            "timestamp": datetime.now().isoformat(),
            "user": message,
            "ai": response,
            "time_ms": elapsed,
        })
        
        return response
    
    def run(self):
        """Ejecuta el chat interactivo."""
        clear_screen()
        print_header()
        
        # Saludo inicial
        greeting = self.ia.context.get_greeting()
        print(format_ai_message(greeting))
        print()
        
        print(format_system_message("Escribe /ayuda para ver los comandos disponibles"))
        print()
        
        while True:
            try:
                # Prompt con color
                user_input = input(f"{Fore.GREEN}👤 Tú:{Style.RESET_ALL} ").strip()
                
                if not user_input:
                    continue
                
                # Procesar mensaje
                response = self.chat(user_input)
                
                if response == "EXIT":
                    print()
                    print(format_ai_message(f"¡Hasta luego, {self.ia.context.profile.name}! Fue un placer conversar contigo. 👋"))
                    print()
                    
                    # Mostrar estadísticas finales
                    print(self.get_session_stats())
                    break
                
                # Mostrar respuesta
                print()
                print(format_ai_message(response))
                print()
                
            except KeyboardInterrupt:
                print()
                print(format_system_message("Interrupción detectada"))
                print(format_ai_message("¡Hasta luego! 👋"))
                break
            
            except Exception as e:
                print(format_error_message(f"Error: {e}"))


# =============================================================================
# FUNCIONES DE ENTRADA
# =============================================================================

def main():
    """Función principal."""
    import argparse
    
    parser = argparse.ArgumentParser(description="IA-Personal Chat")
    parser.add_argument("--turbo", action="store_true", help="Modo turbo (más rápido, menos preciso)")
    parser.add_argument("--no-accel", action="store_true", help="Desactivar aceleración ADead-BIB")
    parser.add_argument("--export", action="store_true", help="Exportar último historial")
    args = parser.parse_args()
    
    if args.export:
        # Solo exportar
        export_dir = Path(__file__).parent / "ia_personal_data" / "exports"
        if export_dir.exists():
            files = list(export_dir.glob("*.json"))
            if files:
                print(f"Archivos exportados en: {export_dir}")
                for f in sorted(files)[-5:]:
                    print(f"  • {f.name}")
            else:
                print("No hay conversaciones exportadas")
        return
    
    # Iniciar chat
    chat = IAPersonalChat(
        use_acceleration=not args.no_accel,
        turbo_mode=args.turbo,
    )
    chat.run()


if __name__ == "__main__":
    main()
