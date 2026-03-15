"""
Metal-Dead JARVIS - Asistente Inteligente Completo
===================================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Sistema tipo JARVIS que integra:
- Reconocimiento de voz
- Control de mouse/teclado
- Búsqueda en internet
- Análisis de datos
- Creación de proyectos
- Pensamiento crítico
- GPU acelerado
"""

import sys
import time
import threading
import re
from pathlib import Path
from typing import Optional, Dict, List, Tuple, Any
from dataclasses import dataclass

# Agregar paths
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

# Core
from Metal_Dead.core.metal_dead import MetalDeadConfig
from Metal_Dead.core.metal_dead_smart import MetalDeadSmart
from Metal_Dead.core.intelligence import IntelligenceEngine, IntentType

# Tools
from Metal_Dead.tools.web_search import WebSearch, HAS_REQUESTS
from Metal_Dead.tools.file_manager import FileManager
from Metal_Dead.tools.data_analyst import DataAnalyst, HAS_PANDAS

# Voice (importar con cuidado)
try:
    sys.path.insert(0, str(Path(__file__).parent.parent.parent / "IA_Personal"))
    from IA_Personal.voice.system_control import SystemControl, Direction, HAS_PYAUTOGUI
    from IA_Personal.voice.speech_recognition import VoiceRecognizer, VoiceConfig, HAS_SPEECH
    HAS_VOICE = HAS_PYAUTOGUI
except ImportError:
    HAS_VOICE = False
    HAS_SPEECH = False
    HAS_PYAUTOGUI = False

# TTS
try:
    import pyttsx3
    HAS_TTS = True
except ImportError:
    HAS_TTS = False

# GPU
try:
    from Metal_Dead.integrations.gpu_advanced import GPUAdvanced, GPUConfig, HAS_TORCH, TORCH_CUDA
    HAS_GPU = HAS_TORCH and TORCH_CUDA
except:
    HAS_GPU = False


@dataclass
class JarvisConfig:
    """Configuración de JARVIS."""
    use_voice: bool = True
    use_tts: bool = True
    use_gpu: bool = True
    wake_word: str = "jarvis"
    language: str = "es-ES"
    projects_dir: str = ""
    
    def __post_init__(self):
        if not self.projects_dir:
            self.projects_dir = str(Path.home() / "Metal_Dead_Projects")


class MetalJarvis:
    """
    Metal-Dead JARVIS - Tu asistente personal inteligente.
    
    Capacidades:
    - 🎤 Control por voz
    - 🖱️ Control de mouse y teclado
    - 🌐 Búsqueda en internet
    - 📊 Análisis de datos
    - 📁 Creación de proyectos
    - 🧠 Pensamiento crítico
    - 🚀 Aceleración GPU
    """
    
    def __init__(self, config: JarvisConfig = None):
        self.config = config or JarvisConfig()
        
        print("\n" + "=" * 70)
        print("   🤖 Metal-Dead JARVIS")
        print("   Tu Asistente Personal Inteligente")
        print("=" * 70)
        
        # IA Core con pensamiento crítico
        ia_config = MetalDeadConfig(
            vocab_size=15000,
            embed_dim=256,
            num_heads=8,
            hidden_dim=1024,
            num_layers=4,
        )
        self.brain = MetalDeadSmart(ia_config)
        
        # Herramientas
        self.web_search = WebSearch() if HAS_REQUESTS else None
        self.file_manager = FileManager(self.config.projects_dir)
        self.data_analyst = DataAnalyst() if HAS_PANDAS else None
        
        # Control de sistema
        self.system_control = SystemControl() if HAS_PYAUTOGUI else None
        
        # Voz
        self.voice_recognizer = None
        self.tts_engine = None
        
        if self.config.use_voice and HAS_SPEECH:
            voice_config = VoiceConfig(
                language=self.config.language,
                wake_word=self.config.wake_word,
                wake_word_enabled=True,
            )
            self.voice_recognizer = VoiceRecognizer(voice_config)
        
        if self.config.use_tts and HAS_TTS:
            self.tts_engine = pyttsx3.init()
            self.tts_engine.setProperty('rate', 180)
            # Buscar voz en español
            for voice in self.tts_engine.getProperty('voices'):
                if 'spanish' in voice.name.lower() or 'español' in voice.name.lower():
                    self.tts_engine.setProperty('voice', voice.id)
                    break
        
        # Estado
        self.is_running = False
        self.is_listening = False
        self.command_history: List[Dict] = []
        
        # Comandos especiales de JARVIS
        self._init_jarvis_commands()
        
        self._print_status()
    
    def _init_jarvis_commands(self):
        """Inicializa comandos especiales de JARVIS."""
        self.jarvis_patterns = {
            # Búsqueda web
            r"(busca|buscar|search)\s+en\s+(internet|web|google)\s+(.+)": self._cmd_web_search,
            r"(qué|que)\s+es\s+(.+)": self._cmd_quick_answer,
            r"(investiga|investigar)\s+sobre\s+(.+)": self._cmd_research,
            
            # Proyectos
            r"(crea|crear)\s+(un\s*)?(proyecto|carpeta)\s+(de\s*)?(data\s*analyst|análisis|python|ml)?\s*(.*)": self._cmd_create_project,
            r"(genera|generar)\s+(un\s*)?(reporte|informe)": self._cmd_generate_report,
            
            # Mouse
            r"(mueve|mover)\s+(el\s*)?(mouse|ratón|cursor)\s+(hacia\s*)?(arriba|abajo|izquierda|derecha|centro)": self._cmd_move_mouse,
            r"^(arriba|abajo|izquierda|derecha|centro)$": self._cmd_move_mouse_simple,
            r"(click|clic)(\s+derecho)?": self._cmd_click,
            r"(doble\s*click|doble\s*clic)": self._cmd_double_click,
            
            # Aplicaciones
            r"(abre|abrir)\s+(el\s*)?(navegador|chrome|firefox|notepad|calculadora|terminal|vscode)": self._cmd_open_app,
            r"(abre|abrir)\s+(.+)": self._cmd_open_app_generic,
            
            # Sistema
            r"(sube|subir|aumenta)\s+(el\s*)?(volumen)": self._cmd_volume_up,
            r"(baja|bajar|reduce)\s+(el\s*)?(volumen)": self._cmd_volume_down,
            r"(silencia|silenciar|mute)": self._cmd_mute,
            r"(captura|screenshot|pantallazo)": self._cmd_screenshot,
            
            # Datos
            r"(analiza|analizar)\s+(los\s*)?(datos|data)": self._cmd_analyze_data,
            r"(carga|cargar)\s+(archivo|datos)\s+(.+)": self._cmd_load_data,
            
            # Control
            r"(estado|status)": self._cmd_status,
            r"(ayuda|help|comandos)": self._cmd_help,
        }
    
    def _print_status(self):
        """Imprime estado del sistema."""
        print(f"\n📊 Estado del Sistema:")
        print(f"   🧠 IA Smart: ✅")
        print(f"   🎤 Voz: {'✅' if self.voice_recognizer else '❌'}")
        print(f"   🔊 TTS: {'✅' if self.tts_engine else '❌'}")
        print(f"   🖱️ Control Sistema: {'✅' if self.system_control else '❌'}")
        print(f"   🌐 Web Search: {'✅' if self.web_search else '❌'}")
        print(f"   📊 Data Analyst: {'✅' if self.data_analyst else '❌'}")
        print(f"   🚀 GPU: {'✅' if HAS_GPU else '❌'}")
        print(f"\n💡 Wake word: '{self.config.wake_word}'")
        print("=" * 70)
    
    # =========================================================================
    # COMANDOS
    # =========================================================================
    
    def _cmd_web_search(self, match: re.Match) -> str:
        """Busca en internet."""
        query = match.group(3).strip()
        if not self.web_search:
            return "Búsqueda web no disponible. Instala: pip install requests beautifulsoup4"
        
        self.speak(f"Buscando {query}")
        results = self.web_search.search(query)
        
        if results:
            response = f"Encontré {len(results)} resultados para '{query}':\n\n"
            for i, r in enumerate(results[:3], 1):
                response += f"{i}. **{r.title}**\n   {r.snippet[:100]}...\n\n"
            return response
        return f"No encontré resultados para '{query}'"
    
    def _cmd_quick_answer(self, match: re.Match) -> str:
        """Respuesta rápida de Wikipedia."""
        topic = match.group(2).strip()
        if not self.web_search:
            return "Búsqueda no disponible"
        
        self.speak(f"Buscando información sobre {topic}")
        answer = self.web_search.quick_answer(topic)
        
        if answer:
            return f"📚 **{topic.title()}**\n\n{answer[:500]}..."
        return f"No encontré información sobre '{topic}'"
    
    def _cmd_research(self, match: re.Match) -> str:
        """Investigación profunda."""
        topic = match.group(2).strip()
        if not self.web_search:
            return "Búsqueda no disponible"
        
        self.speak(f"Investigando sobre {topic}")
        
        # Buscar en múltiples fuentes
        results = self.web_search.search(topic)
        wiki_answer = self.web_search.quick_answer(topic)
        
        response = f"🔍 **Investigación: {topic}**\n\n"
        
        if wiki_answer:
            response += f"📚 **Resumen:**\n{wiki_answer[:300]}...\n\n"
        
        if results:
            response += "🌐 **Fuentes encontradas:**\n"
            for i, r in enumerate(results[:5], 1):
                response += f"{i}. {r.title} ({r.source})\n"
        
        return response
    
    def _cmd_create_project(self, match: re.Match) -> str:
        """Crea un proyecto."""
        project_type = match.group(5) or "data_analyst"
        project_name = match.group(6).strip() or f"proyecto_{int(time.time())}"
        
        # Mapear tipos
        type_map = {
            "data analyst": "data_analyst",
            "análisis": "data_analyst",
            "python": "python_project",
            "ml": "ml_project",
        }
        template = type_map.get(project_type.lower().strip(), "data_analyst")
        
        self.speak(f"Creando proyecto {project_name}")
        
        try:
            path = self.file_manager.create_project(project_name, template)
            return f"✅ Proyecto '{project_name}' creado en:\n{path}"
        except Exception as e:
            return f"❌ Error creando proyecto: {e}"
    
    def _cmd_generate_report(self, match: re.Match) -> str:
        """Genera reporte de datos."""
        if not self.data_analyst:
            return "Data Analyst no disponible. Instala: pip install pandas"
        
        if self.data_analyst.current_df is None:
            return "No hay datos cargados. Usa 'carga archivo [ruta]' primero."
        
        self.speak("Generando reporte")
        report = self.data_analyst.generate_report()
        return f"✅ Reporte generado:\n{report[:500]}..."
    
    def _cmd_move_mouse(self, match: re.Match) -> str:
        """Mueve el mouse."""
        if not self.system_control:
            return "Control de sistema no disponible. Instala: pip install pyautogui"
        
        direction_str = match.group(5).lower()
        direction_map = {
            "arriba": Direction.UP,
            "abajo": Direction.DOWN,
            "izquierda": Direction.LEFT,
            "derecha": Direction.RIGHT,
            "centro": Direction.CENTER,
        }
        
        direction = direction_map.get(direction_str)
        if direction:
            self.system_control.move_mouse(direction)
            return f"Mouse movido hacia {direction_str}"
        return "Dirección no reconocida"
    
    def _cmd_move_mouse_simple(self, match: re.Match) -> str:
        """Mueve el mouse (comando simple)."""
        if not self.system_control:
            return "Control no disponible"
        
        direction_str = match.group(1).lower()
        direction_map = {
            "arriba": Direction.UP,
            "abajo": Direction.DOWN,
            "izquierda": Direction.LEFT,
            "derecha": Direction.RIGHT,
            "centro": Direction.CENTER,
        }
        
        direction = direction_map.get(direction_str)
        if direction:
            self.system_control.move_mouse(direction)
            return f"🖱️ {direction_str}"
        return ""
    
    def _cmd_click(self, match: re.Match) -> str:
        """Click del mouse."""
        if not self.system_control:
            return "Control no disponible"
        
        if match.group(2):  # click derecho
            self.system_control.right_click()
            return "Click derecho"
        else:
            self.system_control.click()
            return "Click"
    
    def _cmd_double_click(self, match: re.Match) -> str:
        """Doble click."""
        if not self.system_control:
            return "Control no disponible"
        
        self.system_control.double_click()
        return "Doble click"
    
    def _cmd_open_app(self, match: re.Match) -> str:
        """Abre una aplicación conocida."""
        if not self.system_control:
            return "Control no disponible"
        
        app = match.group(3).lower()
        self.speak(f"Abriendo {app}")
        
        if self.system_control.open_app(app):
            return f"Abriendo {app}"
        return f"No pude abrir {app}"
    
    def _cmd_open_app_generic(self, match: re.Match) -> str:
        """Abre una aplicación genérica."""
        if not self.system_control:
            return "Control no disponible"
        
        app = match.group(2).strip()
        self.speak(f"Abriendo {app}")
        
        if self.system_control.open_app(app):
            return f"Abriendo {app}"
        return f"No pude abrir {app}"
    
    def _cmd_volume_up(self, match: re.Match) -> str:
        """Sube volumen."""
        if not self.system_control:
            return "Control no disponible"
        self.system_control.volume_up()
        return "🔊 Volumen aumentado"
    
    def _cmd_volume_down(self, match: re.Match) -> str:
        """Baja volumen."""
        if not self.system_control:
            return "Control no disponible"
        self.system_control.volume_down()
        return "🔉 Volumen reducido"
    
    def _cmd_mute(self, match: re.Match) -> str:
        """Silencia."""
        if not self.system_control:
            return "Control no disponible"
        self.system_control.volume_mute()
        return "🔇 Mute"
    
    def _cmd_screenshot(self, match: re.Match) -> str:
        """Toma screenshot."""
        if not self.system_control:
            return "Control no disponible"
        filename = self.system_control.screenshot()
        return f"📸 Screenshot guardado: {filename}"
    
    def _cmd_analyze_data(self, match: re.Match) -> str:
        """Analiza datos cargados."""
        if not self.data_analyst:
            return "Data Analyst no disponible"
        
        if self.data_analyst.current_df is None:
            # Crear datos de ejemplo
            self.data_analyst.create_sample_data()
        
        return self.data_analyst.describe()
    
    def _cmd_load_data(self, match: re.Match) -> str:
        """Carga archivo de datos."""
        if not self.data_analyst:
            return "Data Analyst no disponible"
        
        filepath = match.group(3).strip()
        
        try:
            if filepath.endswith('.csv'):
                self.data_analyst.load_csv(filepath)
            elif filepath.endswith(('.xlsx', '.xls')):
                self.data_analyst.load_excel(filepath)
            elif filepath.endswith('.json'):
                self.data_analyst.load_json(filepath)
            else:
                return f"Formato no soportado: {filepath}"
            
            return f"✅ Datos cargados: {filepath}\n\n{self.data_analyst.describe()}"
        except Exception as e:
            return f"❌ Error cargando datos: {e}"
    
    def _cmd_status(self, match: re.Match) -> str:
        """Muestra estado del sistema."""
        stats = self.brain.get_stats()
        
        lines = [
            "🤖 **Estado de JARVIS:**",
            f"• Interacciones: {stats.get('interaction_count', 0)}",
            f"• Memorias: {stats.get('memory_count', 0)}",
            f"• Pensamientos: {stats.get('thought_count', 0)}",
            f"• RAM IA: {stats.get('ram_mb', 0):.2f} MB",
            "",
            "📊 **Módulos:**",
            f"• Voz: {'✅' if self.voice_recognizer else '❌'}",
            f"• Control: {'✅' if self.system_control else '❌'}",
            f"• Web: {'✅' if self.web_search else '❌'}",
            f"• Data: {'✅' if self.data_analyst else '❌'}",
            f"• GPU: {'✅' if HAS_GPU else '❌'}",
        ]
        
        return "\n".join(lines)
    
    def _cmd_help(self, match: re.Match) -> str:
        """Muestra ayuda."""
        return """🤖 **JARVIS - Comandos Disponibles:**

**🌐 Búsqueda:**
• "busca en internet [tema]"
• "qué es [tema]"
• "investiga sobre [tema]"

**📁 Proyectos:**
• "crea proyecto data analyst [nombre]"
• "crea proyecto python [nombre]"
• "genera reporte"

**🖱️ Mouse:**
• "mueve el mouse arriba/abajo/izquierda/derecha"
• "click" / "click derecho" / "doble click"

**📱 Aplicaciones:**
• "abre chrome/notepad/calculadora/vscode"

**🔊 Sistema:**
• "sube/baja el volumen"
• "silenciar"
• "captura" (screenshot)

**📊 Datos:**
• "analiza los datos"
• "carga archivo [ruta]"

**💬 Chat:**
• Cualquier otra cosa → conversación inteligente
"""
    
    # =========================================================================
    # PROCESAMIENTO
    # =========================================================================
    
    def process(self, text: str) -> str:
        """
        Procesa un comando o mensaje.
        Primero intenta comandos especiales, luego usa IA.
        """
        text = text.strip()
        if not text:
            return ""
        
        # Guardar en historial
        self.command_history.append({
            "input": text,
            "timestamp": time.time(),
        })
        
        # Intentar comandos especiales de JARVIS
        for pattern, handler in self.jarvis_patterns.items():
            match = re.search(pattern, text.lower())
            if match:
                try:
                    response = handler(match)
                    if response:
                        return response
                except Exception as e:
                    return f"Error ejecutando comando: {e}"
        
        # Si no es comando especial, usar IA inteligente
        return self.brain.chat(text)
    
    def speak(self, text: str):
        """Habla el texto (TTS)."""
        if self.tts_engine and text:
            # Hablar en background
            def _speak():
                try:
                    self.tts_engine.say(text)
                    self.tts_engine.runAndWait()
                except:
                    pass
            threading.Thread(target=_speak, daemon=True).start()
    
    # =========================================================================
    # MODOS DE EJECUCIÓN
    # =========================================================================
    
    def interactive(self):
        """Modo interactivo por texto."""
        print("\n" + "=" * 70)
        print("   🤖 JARVIS - Modo Interactivo")
        print("   Escribe 'ayuda' para ver comandos")
        print("   Escribe 'salir' para terminar")
        print("=" * 70)
        
        greeting = self.brain.context.get_greeting()
        print(f"\n🤖 JARVIS: {greeting}\n")
        
        while True:
            try:
                user_input = input("Tú: ").strip()
                
                if not user_input:
                    continue
                
                if user_input.lower() in ["salir", "exit", "quit", "q"]:
                    self.speak("Hasta luego")
                    print("\n🤖 JARVIS: ¡Hasta luego! 👋")
                    break
                
                start = time.perf_counter()
                response = self.process(user_input)
                elapsed = (time.perf_counter() - start) * 1000
                
                print(f"\n🤖 JARVIS: {response}")
                print(f"   [{elapsed:.1f}ms]\n")
                
            except KeyboardInterrupt:
                self.speak("Hasta luego")
                print("\n\n🤖 JARVIS: ¡Hasta luego! 👋")
                break
    
    def voice_mode(self):
        """Modo de voz (como JARVIS real)."""
        if not self.voice_recognizer:
            print("❌ Reconocimiento de voz no disponible")
            print("   Instala: pip install SpeechRecognition pyaudio")
            return
        
        print("\n" + "=" * 70)
        print("   🤖 JARVIS - Modo Voz")
        print(f"   Di '{self.config.wake_word}' para activar")
        print("   Presiona Ctrl+C para salir")
        print("=" * 70)
        
        self.speak("JARVIS activado. Di mi nombre cuando me necesites.")
        
        # Configurar callbacks
        def on_speech(text):
            print(f"\n🗣️ Tú: {text}")
            response = self.process(text)
            print(f"🤖 JARVIS: {response}")
            self.speak(response[:200])  # Limitar TTS
        
        def on_wake():
            print("\n🔔 ¡JARVIS activado!")
            self.speak("¿Sí? ¿En qué puedo ayudarte?")
        
        self.voice_recognizer.on_speech_detected = on_speech
        self.voice_recognizer.on_wake_word = on_wake
        
        # Calibrar y escuchar
        self.voice_recognizer.calibrate(duration=2)
        self.voice_recognizer.start_listening()
        
        try:
            while True:
                time.sleep(0.1)
        except KeyboardInterrupt:
            self.voice_recognizer.stop_listening()
            self.speak("Hasta luego")
            print("\n\n🤖 JARVIS: ¡Hasta luego! 👋")


# =============================================================================
# CLI
# =============================================================================

def main():
    """Punto de entrada principal."""
    import argparse
    
    parser = argparse.ArgumentParser(description="🤖 Metal-Dead JARVIS")
    parser.add_argument("--voice", action="store_true", help="Modo voz")
    parser.add_argument("--no-tts", action="store_true", help="Desactivar síntesis de voz")
    parser.add_argument("--wake-word", type=str, default="jarvis", help="Palabra de activación")
    args = parser.parse_args()
    
    config = JarvisConfig(
        use_voice=args.voice,
        use_tts=not args.no_tts,
        wake_word=args.wake_word,
    )
    
    jarvis = MetalJarvis(config)
    
    if args.voice:
        jarvis.voice_mode()
    else:
        jarvis.interactive()


if __name__ == "__main__":
    main()
