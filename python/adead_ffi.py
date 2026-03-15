"""
ADead-BIB FFI - Foreign Function Interface
==========================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪
Permite usar funciones ADead-BIB desde Python para procesamiento rápido.

Uso:
    from adead_ffi import ADeadBIB
    
    adead = ADeadBIB()
    result = adead.compile_and_run("examples/hello_world.adB")
    print(result)
"""

import subprocess
import os
import tempfile
from pathlib import Path
from typing import Optional, List, Dict, Any


class ADeadBIB:
    """Wrapper para integrar ADead-BIB con Python."""
    
    def __init__(self, compiler_path: Optional[str] = None):
        """
        Inicializa el wrapper de ADead-BIB.
        
        Args:
            compiler_path: Ruta al compilador adeadc.exe
        """
        if compiler_path:
            self.compiler = Path(compiler_path)
        else:
            # Buscar en ubicaciones comunes
            possible_paths = [
                Path(__file__).parent.parent / "target" / "release" / "adeadc.exe",
                Path(__file__).parent.parent / "target" / "debug" / "adeadc.exe",
                Path("adeadc.exe"),
            ]
            for p in possible_paths:
                if p.exists():
                    self.compiler = p
                    break
            else:
                raise FileNotFoundError("No se encontró adeadc.exe")
        
        self.base_dir = self.compiler.parent.parent.parent
    
    def compile(self, source_file: str) -> str:
        """
        Compila un archivo .adB a ejecutable.
        
        Args:
            source_file: Ruta al archivo .adB
            
        Returns:
            Ruta al ejecutable generado
        """
        source = Path(source_file).resolve()
        if not source.exists():
            raise FileNotFoundError(f"Archivo no encontrado: {source_file}")
        
        result = subprocess.run(
            [str(self.compiler), str(source)],
            capture_output=True,
            cwd=str(source.parent),
            encoding='utf-8',
            errors='replace'
        )
        
        if result.returncode != 0:
            raise RuntimeError(f"Error de compilación:\n{result.stderr}")
        
        exe_name = source.stem + ".exe"
        exe_path = source.parent / exe_name
        
        if not exe_path.exists():
            raise RuntimeError(f"No se generó el ejecutable: {exe_path}")
        
        return str(exe_path)
    
    def run(self, exe_path: str, input_data: str = "") -> str:
        """
        Ejecuta un binario ADead-BIB.
        
        Args:
            exe_path: Ruta al ejecutable
            input_data: Datos de entrada (stdin)
            
        Returns:
            Salida del programa (stdout)
        """
        exe = Path(exe_path).resolve()
        result = subprocess.run(
            [str(exe)],
            input=input_data,
            capture_output=True,
            cwd=str(exe.parent),
            encoding='utf-8',
            errors='replace'
        )
        
        return result.stdout
    
    def compile_and_run(self, source_file: str, input_data: str = "") -> str:
        """
        Compila y ejecuta un archivo .adB.
        
        Args:
            source_file: Ruta al archivo .adB
            input_data: Datos de entrada
            
        Returns:
            Salida del programa
        """
        exe_path = self.compile(source_file)
        return self.run(exe_path, input_data)
    
    def compile_code(self, code: str) -> str:
        """
        Compila código ADead-BIB desde un string.
        
        Args:
            code: Código fuente ADead-BIB
            
        Returns:
            Ruta al ejecutable generado
        """
        # Crear archivo temporal en el directorio de ejemplos
        temp_dir = self.base_dir / "examples"
        temp_file = temp_dir / f"_temp_{os.getpid()}.adB"
        
        with open(temp_file, 'w', encoding='utf-8') as f:
            f.write(code)
        
        try:
            exe_path = self.compile(str(temp_file))
            return exe_path
        finally:
            try:
                os.unlink(temp_file)
            except:
                pass
    
    def run_code(self, code: str, input_data: str = "") -> str:
        """
        Compila y ejecuta código ADead-BIB desde un string.
        
        Args:
            code: Código fuente ADead-BIB
            input_data: Datos de entrada
            
        Returns:
            Salida del programa
        """
        exe_path = self.compile_code(code)
        try:
            return self.run(exe_path, input_data)
        finally:
            os.unlink(exe_path)
    
    # Funciones de utilidad para procesamiento rápido
    def fast_sum(self, numbers: List[int]) -> int:
        """Suma rápida usando ADead-BIB."""
        code = f"""
def main():
    total = 0
    {chr(10).join(f'    total = total + {n}' for n in numbers)}
    print(total)
"""
        result = self.run_code(code)
        return int(result.strip())
    
    def fast_max(self, numbers: List[int]) -> int:
        """Máximo rápido usando ADead-BIB."""
        if not numbers:
            return 0
        code = f"""
def main():
    m = {numbers[0]}
    {chr(10).join(f'    m = max(m, {n})' for n in numbers[1:])}
    print(m)
"""
        result = self.run_code(code)
        return int(result.strip())
    
    def fast_min(self, numbers: List[int]) -> int:
        """Mínimo rápido usando ADead-BIB."""
        if not numbers:
            return 0
        code = f"""
def main():
    m = {numbers[0]}
    {chr(10).join(f'    m = min(m, {n})' for n in numbers[1:])}
    print(m)
"""
        result = self.run_code(code)
        return int(result.strip())


class ADeadAI:
    """
    IA básica usando ADead-BIB para procesamiento rápido.
    Combina Python (flexibilidad) con ADead-BIB (velocidad).
    """
    
    def __init__(self):
        self.adead = ADeadBIB()
        self.vocabulary: Dict[str, int] = {}
        self.word_count = 0
    
    def load_vocabulary(self, words: List[str]):
        """Carga vocabulario para la IA."""
        for i, word in enumerate(words):
            self.vocabulary[word.lower()] = i
        self.word_count = len(words)
        print(f"Vocabulario cargado: {self.word_count} palabras")
    
    def tokenize(self, text: str) -> List[int]:
        """Tokeniza texto usando el vocabulario."""
        tokens = []
        for word in text.lower().split():
            word = ''.join(c for c in word if c.isalnum())
            if word in self.vocabulary:
                tokens.append(self.vocabulary[word])
            else:
                tokens.append(-1)  # Unknown token
        return tokens
    
    def count_known_words(self, text: str) -> int:
        """Cuenta palabras conocidas en el texto."""
        tokens = self.tokenize(text)
        return sum(1 for t in tokens if t >= 0)
    
    def analyze_text(self, text: str) -> Dict[str, Any]:
        """
        Analiza texto usando ADead-BIB para cálculos rápidos.
        """
        words = text.lower().split()
        word_lengths = [len(w) for w in words]
        
        # Usar ADead-BIB para cálculos
        total_chars = self.adead.fast_sum(word_lengths) if word_lengths else 0
        max_len = self.adead.fast_max(word_lengths) if word_lengths else 0
        min_len = self.adead.fast_min(word_lengths) if word_lengths else 0
        
        return {
            "total_words": len(words),
            "total_chars": total_chars,
            "avg_word_length": total_chars / len(words) if words else 0,
            "max_word_length": max_len,
            "min_word_length": min_len,
            "known_words": self.count_known_words(text),
        }
    
    def similarity_score(self, text1: str, text2: str) -> float:
        """
        Calcula similitud entre dos textos.
        """
        tokens1 = set(self.tokenize(text1))
        tokens2 = set(self.tokenize(text2))
        
        # Jaccard similarity
        intersection = len(tokens1 & tokens2)
        union = len(tokens1 | tokens2)
        
        return intersection / union if union > 0 else 0.0


# Ejemplo de uso
if __name__ == "__main__":
    print("=== ADead-BIB FFI Demo ===\n")
    
    # Test básico
    try:
        adead = ADeadBIB()
        print("✓ Compilador encontrado")
        
        # Compilar y ejecutar hello_world
        result = adead.compile_and_run("../examples/hello_world.adB")
        print(f"✓ Hello World: {result.strip()}")
        
        # Test de funciones rápidas
        numbers = [10, 20, 30, 40, 50]
        print(f"\n✓ fast_sum({numbers}) = {adead.fast_sum(numbers)}")
        print(f"✓ fast_max({numbers}) = {adead.fast_max(numbers)}")
        print(f"✓ fast_min({numbers}) = {adead.fast_min(numbers)}")
        
        print("\n=== FFI Funcionando ===")
        
    except Exception as e:
        print(f"Error: {e}")
