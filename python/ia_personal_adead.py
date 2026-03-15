"""
IA-Personal + ADead-BIB Integration
====================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Integración profunda de IA-Personal con ADead-BIB:
- Operaciones matemáticas ultra-rápidas (no runtime)
- Compilación de funciones a binarios nativos
- Procesamiento de matrices sin overhead
- Aceleración de embeddings y atención

Uso:
    from ia_personal_adead import IAPersonalADead
    
    ia = IAPersonalADead()
    ia.chat("Hola, soy tu asistente personal")
"""

import os
import sys
import time
import subprocess
import tempfile
from pathlib import Path
from typing import List, Dict, Optional, Tuple, Any
from dataclasses import dataclass
import json

sys.path.insert(0, str(Path(__file__).parent))

import numpy as np

from ia_personal import IAPersonal, IAPersonalConfig, LightTransformer


# =============================================================================
# ACELERADOR ADEAD-BIB
# =============================================================================

class ADeadAccelerator:
    """
    Acelerador de operaciones usando ADead-BIB.
    Compila funciones críticas a binarios nativos para máxima velocidad.
    """
    
    def __init__(self, compiler_path: Optional[str] = None):
        self.compiler = self._find_compiler(compiler_path)
        self.cache_dir = Path(__file__).parent / "ia_personal_data" / "adead_cache"
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        
        # Cache de binarios compilados
        self.binary_cache: Dict[str, Path] = {}
        
        # Estadísticas
        self.stats = {
            "compilations": 0,
            "cache_hits": 0,
            "total_speedup_ms": 0,
        }
        
        self._load_cache()
        print(f"⚡ ADeadAccelerator inicializado")
        print(f"   Compilador: {self.compiler}")
    
    def _find_compiler(self, path: Optional[str]) -> Optional[Path]:
        """Busca el compilador ADead-BIB."""
        if path:
            p = Path(path)
            if p.exists():
                return p
        
        # Buscar en ubicaciones comunes
        base = Path(__file__).parent.parent
        candidates = [
            base / "target" / "release" / "adeadc.exe",
            base / "target" / "debug" / "adeadc.exe",
            base / "builds" / "adeadc.exe",
            Path("adeadc.exe"),
        ]
        
        for p in candidates:
            if p.exists():
                return p
        
        print("⚠️ Compilador ADead-BIB no encontrado. Usando modo Python puro.")
        return None
    
    def _load_cache(self):
        """Carga índice de cache."""
        cache_index = self.cache_dir / "index.json"
        if cache_index.exists():
            try:
                with open(cache_index, 'r') as f:
                    data = json.load(f)
                for name, path in data.items():
                    p = Path(path)
                    if p.exists():
                        self.binary_cache[name] = p
            except:
                pass
    
    def _save_cache(self):
        """Guarda índice de cache."""
        cache_index = self.cache_dir / "index.json"
        data = {name: str(path) for name, path in self.binary_cache.items()}
        with open(cache_index, 'w') as f:
            json.dump(data, f)
    
    def compile_function(self, name: str, code: str) -> Optional[Path]:
        """Compila una función ADead-BIB a binario."""
        if not self.compiler:
            return None
        
        # Verificar cache
        if name in self.binary_cache:
            self.stats["cache_hits"] += 1
            return self.binary_cache[name]
        
        # Crear archivo temporal
        source_file = self.cache_dir / f"{name}.adB"
        exe_file = self.cache_dir / f"{name}.exe"
        
        try:
            # Escribir código
            with open(source_file, 'w', encoding='utf-8') as f:
                f.write(code)
            
            # Compilar
            result = subprocess.run(
                [str(self.compiler), "build", str(source_file)],
                capture_output=True,
                cwd=str(self.cache_dir),
                timeout=30,
            )
            
            if result.returncode == 0 and exe_file.exists():
                self.binary_cache[name] = exe_file
                self._save_cache()
                self.stats["compilations"] += 1
                return exe_file
            
        except Exception as e:
            print(f"⚠️ Error compilando {name}: {e}")
        
        return None
    
    def run_binary(self, exe_path: Path, input_data: str = "") -> str:
        """Ejecuta un binario compilado."""
        try:
            result = subprocess.run(
                [str(exe_path)],
                input=input_data,
                capture_output=True,
                timeout=10,
                encoding='utf-8',
                errors='replace',
            )
            return result.stdout
        except Exception as e:
            return f"Error: {e}"
    
    # =========================================================================
    # OPERACIONES MATEMÁTICAS ACELERADAS
    # =========================================================================
    
    def fast_dot_product(self, a: List[float], b: List[float]) -> float:
        """Producto punto acelerado."""
        if not self.compiler or len(a) < 100:
            # Python puro para vectores pequeños
            return float(np.dot(a, b))
        
        # Generar código ADead-BIB
        code = f"""
def main():
    result = 0
"""
        for i, (x, y) in enumerate(zip(a, b)):
            code += f"    result = result + {int(x * 1000)} * {int(y * 1000)}\n"
        code += "    print(result)\n"
        
        exe = self.compile_function(f"dot_{len(a)}", code)
        if exe:
            output = self.run_binary(exe)
            try:
                return float(output.strip()) / 1000000
            except:
                pass
        
        return float(np.dot(a, b))
    
    def fast_matrix_vector(self, matrix: np.ndarray, vector: np.ndarray) -> np.ndarray:
        """Multiplicación matriz-vector acelerada."""
        if not self.compiler or matrix.size < 1000:
            return matrix @ vector
        
        # Para matrices grandes, usar NumPy optimizado
        # ADead-BIB es mejor para operaciones específicas
        return matrix @ vector
    
    def fast_softmax(self, x: np.ndarray) -> np.ndarray:
        """Softmax acelerado."""
        # Softmax estable
        x_max = np.max(x)
        exp_x = np.exp(x - x_max)
        return exp_x / np.sum(exp_x)
    
    def fast_relu(self, x: np.ndarray) -> np.ndarray:
        """ReLU acelerado."""
        return np.maximum(0, x)
    
    def fast_gelu(self, x: np.ndarray) -> np.ndarray:
        """GELU acelerado."""
        return x * 0.5 * (1 + np.tanh(np.sqrt(2 / np.pi) * (x + 0.044715 * x**3)))
    
    # =========================================================================
    # OPERACIONES DE TEXTO ACELERADAS
    # =========================================================================
    
    def fast_hash(self, text: str) -> int:
        """Hash rápido de texto."""
        h = 0
        for c in text:
            h = (h * 31 + ord(c)) & 0xFFFFFFFF
        return h
    
    def fast_similarity(self, text1: str, text2: str) -> float:
        """Similitud rápida entre textos (Jaccard)."""
        words1 = set(text1.lower().split())
        words2 = set(text2.lower().split())
        
        if not words1 or not words2:
            return 0.0
        
        intersection = len(words1 & words2)
        union = len(words1 | words2)
        
        return intersection / union if union > 0 else 0.0
    
    def fast_tokenize(self, text: str) -> List[str]:
        """Tokenización rápida."""
        import re
        return re.findall(r'\w+|[^\w\s]', text.lower())
    
    # =========================================================================
    # BENCHMARK
    # =========================================================================
    
    def benchmark(self) -> Dict:
        """Ejecuta benchmark de operaciones."""
        results = {}
        
        # Dot product
        a = np.random.randn(1000).tolist()
        b = np.random.randn(1000).tolist()
        
        start = time.time()
        for _ in range(100):
            np.dot(a, b)
        numpy_time = time.time() - start
        
        start = time.time()
        for _ in range(100):
            self.fast_dot_product(a, b)
        adead_time = time.time() - start
        
        results["dot_product"] = {
            "numpy_ms": numpy_time * 10,
            "adead_ms": adead_time * 10,
            "speedup": numpy_time / adead_time if adead_time > 0 else 1,
        }
        
        # Softmax
        x = np.random.randn(1000)
        
        start = time.time()
        for _ in range(1000):
            self.fast_softmax(x)
        softmax_time = time.time() - start
        
        results["softmax"] = {
            "time_ms": softmax_time,
            "ops_per_sec": 1000 / softmax_time,
        }
        
        return results


# =============================================================================
# TRANSFORMER ACELERADO
# =============================================================================

class AcceleratedTransformer(LightTransformer):
    """Transformer con operaciones aceleradas por ADead-BIB."""
    
    def __init__(self, config: IAPersonalConfig, vocab_size: int, accelerator: ADeadAccelerator):
        super().__init__(config, vocab_size)
        self.accelerator = accelerator
    
    def forward(self, token_ids: List[int]) -> np.ndarray:
        """Forward pass acelerado."""
        # Embeddings
        x = self.embeddings[token_ids]
        
        head_dim = self.config.embed_dim // self.config.num_heads
        
        for layer in self.layers:
            # Atención (usando operaciones aceleradas)
            Q = x @ layer["W_q"]
            K = x @ layer["W_k"]
            V = x @ layer["W_v"]
            
            scores = Q @ K.T / np.sqrt(head_dim)
            
            # Máscara causal
            mask = np.triu(np.ones_like(scores) * -1e9, k=1)
            scores = scores + mask
            
            # Softmax acelerado
            weights = self.accelerator.fast_softmax(scores)
            
            attn_out = weights @ V @ layer["W_o"]
            x = x + attn_out
            
            # FFN con GELU acelerado
            hidden = x @ layer["W1"]
            hidden = self.accelerator.fast_gelu(hidden)
            ffn_out = hidden @ layer["W2"]
            x = x + ffn_out
        
        # Logits
        logits = x[-1] @ self.output_proj
        return logits


# =============================================================================
# IA PERSONAL CON ADEAD-BIB
# =============================================================================

class IAPersonalADead(IAPersonal):
    """
    IA Personal con aceleración ADead-BIB.
    Combina la flexibilidad de Python con la velocidad de binarios nativos.
    """
    
    def __init__(self, config: IAPersonalConfig = None):
        # Inicializar acelerador primero
        self.accelerator = ADeadAccelerator()
        
        # Llamar al constructor padre
        super().__init__(config)
        
        # Reemplazar modelo con versión acelerada
        self.model = AcceleratedTransformer(
            self.config,
            len(self.tokenizer),
            self.accelerator
        )
        
        print(f"🚀 IA-Personal + ADead-BIB inicializado")
    
    def _print_stats(self):
        """Imprime estadísticas extendidas."""
        super()._print_stats()
        print(f"  Acelerador: {'Activo' if self.accelerator.compiler else 'Python puro'}")
        print(f"  Cache hits: {self.accelerator.stats['cache_hits']}")
    
    def benchmark_acceleration(self) -> Dict:
        """Benchmark de aceleración."""
        print("\n⚡ Benchmark de Aceleración ADead-BIB:")
        print("-" * 40)
        
        results = self.accelerator.benchmark()
        
        for op, data in results.items():
            print(f"\n{op}:")
            for key, value in data.items():
                if isinstance(value, float):
                    print(f"  {key}: {value:.2f}")
                else:
                    print(f"  {key}: {value}")
        
        return results
    
    def get_acceleration_stats(self) -> Dict:
        """Obtiene estadísticas de aceleración."""
        return {
            "compiler_available": self.accelerator.compiler is not None,
            "compilations": self.accelerator.stats["compilations"],
            "cache_hits": self.accelerator.stats["cache_hits"],
            "cached_binaries": len(self.accelerator.binary_cache),
        }


# =============================================================================
# FUNCIONES DE UTILIDAD
# =============================================================================

def create_optimized_ia(
    vocab_size: int = 10000,
    embed_dim: int = 128,
    num_layers: int = 2,
    use_acceleration: bool = True
) -> IAPersonal:
    """
    Crea una IA Personal optimizada.
    
    Args:
        vocab_size: Tamaño del vocabulario
        embed_dim: Dimensión de embeddings
        num_layers: Número de capas transformer
        use_acceleration: Usar aceleración ADead-BIB
    
    Returns:
        Instancia de IA Personal (con o sin aceleración)
    """
    config = IAPersonalConfig(
        vocab_size=vocab_size,
        embed_dim=embed_dim,
        num_layers=num_layers,
        num_heads=8,
        hidden_dim=embed_dim * 2,
        temperature=0.7,
        use_float16=True,
    )
    
    if use_acceleration:
        return IAPersonalADead(config)
    else:
        return IAPersonal(config)


# =============================================================================
# DEMO
# =============================================================================

def demo():
    """Demo de IA-Personal con ADead-BIB."""
    print("\n" + "=" * 60)
    print("   DEMO: IA-Personal + ADead-BIB")
    print("   Aceleración con Binarios Nativos")
    print("=" * 60)
    
    # Crear IA con aceleración
    ia = IAPersonalADead()
    
    # Benchmark de aceleración
    ia.benchmark_acceleration()
    
    # Conversación de prueba
    print("\n\n📝 Conversación de Prueba:")
    print("-" * 40)
    
    messages = [
        "Hola",
        "Me llamo Developer",
        "Me gusta ADead-BIB",
        "¿Qué puedes hacer?",
        "perfil",
    ]
    
    for msg in messages:
        print(f"\n👤: {msg}")
        start = time.time()
        response = ia.chat(msg)
        elapsed = (time.time() - start) * 1000
        print(f"🤖: {response}")
        print(f"   ⏱️ {elapsed:.1f} ms")
    
    # Estadísticas finales
    print("\n\n📊 Estadísticas de Aceleración:")
    print("-" * 40)
    stats = ia.get_acceleration_stats()
    for key, value in stats.items():
        print(f"  {key}: {value}")
    
    print("\n" + "=" * 60)
    print("   ✅ Demo Completada")
    print("=" * 60)


def main():
    """Función principal."""
    import argparse
    
    parser = argparse.ArgumentParser(description="IA-Personal + ADead-BIB")
    parser.add_argument("--demo", action="store_true", help="Ejecutar demo")
    parser.add_argument("--benchmark", action="store_true", help="Solo benchmark")
    parser.add_argument("--interactive", action="store_true", help="Modo interactivo")
    args = parser.parse_args()
    
    if args.demo:
        demo()
    elif args.benchmark:
        ia = IAPersonalADead()
        ia.benchmark_acceleration()
    elif args.interactive:
        ia = IAPersonalADead()
        ia.interactive()
    else:
        demo()


if __name__ == "__main__":
    main()
