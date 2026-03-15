"""
Búsqueda Web para Metal-Dead
=============================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Búsqueda inteligente en internet:
- Google Search
- DuckDuckGo
- Wikipedia
- Extracción de contenido
"""

import re
import json
import time
from typing import List, Dict, Optional, Any
from dataclasses import dataclass
from urllib.parse import quote_plus, urljoin
from pathlib import Path

# Intentar importar requests
try:
    import requests
    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False
    print("⚠️ requests no instalado: pip install requests")

# Intentar importar BeautifulSoup
try:
    from bs4 import BeautifulSoup
    HAS_BS4 = True
except ImportError:
    HAS_BS4 = False
    print("⚠️ beautifulsoup4 no instalado: pip install beautifulsoup4")


@dataclass
class SearchResult:
    """Resultado de búsqueda."""
    title: str
    url: str
    snippet: str
    source: str
    timestamp: float = 0.0
    
    def to_dict(self) -> Dict:
        return {
            "title": self.title,
            "url": self.url,
            "snippet": self.snippet,
            "source": self.source,
        }


class WebSearch:
    """
    Motor de búsqueda web para Metal-Dead.
    Busca en múltiples fuentes y extrae información.
    """
    
    def __init__(self):
        self.session = requests.Session() if HAS_REQUESTS else None
        self.headers = {
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            "Accept-Language": "es-ES,es;q=0.9,en;q=0.8",
        }
        if self.session:
            self.session.headers.update(self.headers)
        
        # Cache de búsquedas
        self.cache: Dict[str, List[SearchResult]] = {}
        self.cache_ttl = 3600  # 1 hora
        
        print("🌐 WebSearch inicializado")
    
    def search_duckduckgo(self, query: str, max_results: int = 5) -> List[SearchResult]:
        """Busca en DuckDuckGo (no requiere API key)."""
        if not HAS_REQUESTS or not HAS_BS4:
            return []
        
        results = []
        try:
            url = f"https://html.duckduckgo.com/html/?q={quote_plus(query)}"
            response = self.session.get(url, timeout=10)
            
            if response.status_code == 200:
                soup = BeautifulSoup(response.text, 'html.parser')
                
                for result in soup.select('.result')[:max_results]:
                    title_elem = result.select_one('.result__title')
                    snippet_elem = result.select_one('.result__snippet')
                    link_elem = result.select_one('.result__url')
                    
                    if title_elem:
                        title = title_elem.get_text(strip=True)
                        snippet = snippet_elem.get_text(strip=True) if snippet_elem else ""
                        url = link_elem.get_text(strip=True) if link_elem else ""
                        
                        results.append(SearchResult(
                            title=title,
                            url=url,
                            snippet=snippet,
                            source="duckduckgo",
                            timestamp=time.time()
                        ))
        except Exception as e:
            print(f"⚠️ Error en DuckDuckGo: {e}")
        
        return results
    
    def search_wikipedia(self, query: str, lang: str = "es") -> List[SearchResult]:
        """Busca en Wikipedia."""
        if not HAS_REQUESTS:
            return []
        
        results = []
        try:
            # API de Wikipedia
            url = f"https://{lang}.wikipedia.org/w/api.php"
            params = {
                "action": "query",
                "list": "search",
                "srsearch": query,
                "format": "json",
                "srlimit": 5,
            }
            
            response = self.session.get(url, params=params, timeout=10)
            
            if response.status_code == 200:
                data = response.json()
                for item in data.get("query", {}).get("search", []):
                    # Limpiar snippet de HTML
                    snippet = re.sub(r'<[^>]+>', '', item.get("snippet", ""))
                    
                    results.append(SearchResult(
                        title=item.get("title", ""),
                        url=f"https://{lang}.wikipedia.org/wiki/{quote_plus(item.get('title', '').replace(' ', '_'))}",
                        snippet=snippet,
                        source="wikipedia",
                        timestamp=time.time()
                    ))
        except Exception as e:
            print(f"⚠️ Error en Wikipedia: {e}")
        
        return results
    
    def get_wikipedia_summary(self, title: str, lang: str = "es") -> Optional[str]:
        """Obtiene resumen de Wikipedia."""
        if not HAS_REQUESTS:
            return None
        
        try:
            url = f"https://{lang}.wikipedia.org/api/rest_v1/page/summary/{quote_plus(title)}"
            response = self.session.get(url, timeout=10)
            
            if response.status_code == 200:
                data = response.json()
                return data.get("extract", "")
        except Exception as e:
            print(f"⚠️ Error obteniendo resumen: {e}")
        
        return None
    
    def extract_page_content(self, url: str) -> Optional[Dict]:
        """Extrae contenido de una página web."""
        if not HAS_REQUESTS or not HAS_BS4:
            return None
        
        try:
            response = self.session.get(url, timeout=15)
            
            if response.status_code == 200:
                soup = BeautifulSoup(response.text, 'html.parser')
                
                # Remover scripts y estilos
                for tag in soup(['script', 'style', 'nav', 'footer', 'header']):
                    tag.decompose()
                
                # Extraer título
                title = soup.title.string if soup.title else ""
                
                # Extraer texto principal
                main_content = soup.find('main') or soup.find('article') or soup.find('body')
                text = main_content.get_text(separator='\n', strip=True) if main_content else ""
                
                # Limpiar texto
                lines = [line.strip() for line in text.split('\n') if line.strip()]
                text = '\n'.join(lines[:100])  # Limitar a 100 líneas
                
                # Extraer links
                links = []
                for a in soup.find_all('a', href=True)[:20]:
                    href = a['href']
                    if href.startswith('http'):
                        links.append({
                            "text": a.get_text(strip=True)[:50],
                            "url": href
                        })
                
                return {
                    "title": title,
                    "url": url,
                    "text": text[:5000],  # Limitar texto
                    "links": links,
                }
        except Exception as e:
            print(f"⚠️ Error extrayendo contenido: {e}")
        
        return None
    
    def search(self, query: str, sources: List[str] = None) -> List[SearchResult]:
        """
        Búsqueda unificada en múltiples fuentes.
        
        Args:
            query: Término de búsqueda
            sources: Lista de fuentes ["duckduckgo", "wikipedia"]
        """
        if sources is None:
            sources = ["duckduckgo", "wikipedia"]
        
        # Verificar cache
        cache_key = f"{query}:{','.join(sources)}"
        if cache_key in self.cache:
            cached = self.cache[cache_key]
            if cached and (time.time() - cached[0].timestamp) < self.cache_ttl:
                print(f"📦 Usando cache para '{query}'")
                return cached
        
        all_results = []
        
        if "duckduckgo" in sources:
            results = self.search_duckduckgo(query)
            all_results.extend(results)
        
        if "wikipedia" in sources:
            results = self.search_wikipedia(query)
            all_results.extend(results)
        
        # Guardar en cache
        if all_results:
            self.cache[cache_key] = all_results
        
        return all_results
    
    def quick_answer(self, query: str) -> Optional[str]:
        """Obtiene una respuesta rápida de Wikipedia."""
        # Buscar en Wikipedia
        results = self.search_wikipedia(query)
        
        if results:
            # Obtener resumen del primer resultado
            summary = self.get_wikipedia_summary(results[0].title)
            if summary:
                return summary
        
        return None
    
    def format_results(self, results: List[SearchResult]) -> str:
        """Formatea resultados para mostrar."""
        if not results:
            return "No se encontraron resultados."
        
        lines = [f"🔍 **{len(results)} resultados encontrados:**\n"]
        
        for i, r in enumerate(results, 1):
            lines.append(f"**{i}. {r.title}**")
            lines.append(f"   {r.snippet[:150]}...")
            lines.append(f"   🔗 {r.url[:60]}...")
            lines.append(f"   📌 Fuente: {r.source}\n")
        
        return "\n".join(lines)


# =============================================================================
# DEMO
# =============================================================================

def demo():
    """Demo de búsqueda web."""
    print("\n" + "=" * 60)
    print("   🌐 Demo de Búsqueda Web")
    print("   Metal-Dead WebSearch")
    print("=" * 60)
    
    if not HAS_REQUESTS or not HAS_BS4:
        print("\n❌ Instala las dependencias:")
        print("   pip install requests beautifulsoup4")
        return
    
    search = WebSearch()
    
    queries = [
        "Python programación",
        "Inteligencia Artificial",
        "GPU NVIDIA RTX",
    ]
    
    for query in queries:
        print(f"\n🔍 Buscando: '{query}'")
        print("-" * 40)
        
        results = search.search(query)
        print(search.format_results(results[:3]))
    
    # Quick answer
    print("\n📚 Quick Answer: 'Machine Learning'")
    print("-" * 40)
    answer = search.quick_answer("Machine Learning")
    if answer:
        print(answer[:500] + "...")
    
    print("\n" + "=" * 60)


if __name__ == "__main__":
    demo()
