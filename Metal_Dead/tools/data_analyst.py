"""
Data Analyst para Metal-Dead
=============================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Herramientas de análisis de datos:
- Carga y exploración de datos
- Estadísticas descriptivas
- Visualizaciones automáticas
- Generación de reportes
"""

import os
import json
from pathlib import Path
from typing import List, Dict, Optional, Any, Union
from dataclasses import dataclass
from datetime import datetime

# Intentar importar pandas
try:
    import pandas as pd
    HAS_PANDAS = True
except ImportError:
    HAS_PANDAS = False
    print("⚠️ pandas no instalado: pip install pandas")

# Intentar importar numpy
try:
    import numpy as np
    HAS_NUMPY = True
except ImportError:
    HAS_NUMPY = False

# Intentar importar matplotlib
try:
    import matplotlib.pyplot as plt
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False
    print("⚠️ matplotlib no instalado: pip install matplotlib")

# Intentar importar seaborn
try:
    import seaborn as sns
    HAS_SEABORN = True
except ImportError:
    HAS_SEABORN = False


@dataclass
class DatasetInfo:
    """Información de un dataset."""
    name: str
    rows: int
    columns: int
    memory_mb: float
    dtypes: Dict[str, str]
    null_counts: Dict[str, int]
    numeric_columns: List[str]
    categorical_columns: List[str]


class DataAnalyst:
    """
    Analista de datos inteligente para Metal-Dead.
    Automatiza tareas comunes de análisis de datos.
    """
    
    def __init__(self, output_dir: str = None):
        self.output_dir = Path(output_dir) if output_dir else Path.cwd() / "analysis_output"
        self.output_dir.mkdir(parents=True, exist_ok=True)
        
        self.current_df: Optional[pd.DataFrame] = None
        self.datasets: Dict[str, pd.DataFrame] = {}
        self.analysis_history: List[Dict] = []
        
        if HAS_MATPLOTLIB:
            plt.style.use('seaborn-v0_8-whitegrid' if 'seaborn-v0_8-whitegrid' in plt.style.available else 'ggplot')
        
        print("📊 DataAnalyst inicializado")
    
    # =========================================================================
    # CARGA DE DATOS
    # =========================================================================
    
    def load_csv(self, path: str, name: str = None, **kwargs) -> pd.DataFrame:
        """Carga un archivo CSV."""
        if not HAS_PANDAS:
            raise ImportError("pandas no instalado")
        
        df = pd.read_csv(path, **kwargs)
        name = name or Path(path).stem
        self.datasets[name] = df
        self.current_df = df
        
        print(f"✅ Cargado: {name} ({len(df)} filas, {len(df.columns)} columnas)")
        return df
    
    def load_excel(self, path: str, name: str = None, sheet_name: Union[str, int] = 0, **kwargs) -> pd.DataFrame:
        """Carga un archivo Excel."""
        if not HAS_PANDAS:
            raise ImportError("pandas no instalado")
        
        df = pd.read_excel(path, sheet_name=sheet_name, **kwargs)
        name = name or Path(path).stem
        self.datasets[name] = df
        self.current_df = df
        
        print(f"✅ Cargado: {name} ({len(df)} filas, {len(df.columns)} columnas)")
        return df
    
    def load_json(self, path: str, name: str = None, **kwargs) -> pd.DataFrame:
        """Carga un archivo JSON."""
        if not HAS_PANDAS:
            raise ImportError("pandas no instalado")
        
        df = pd.read_json(path, **kwargs)
        name = name or Path(path).stem
        self.datasets[name] = df
        self.current_df = df
        
        print(f"✅ Cargado: {name} ({len(df)} filas, {len(df.columns)} columnas)")
        return df
    
    def create_sample_data(self, name: str = "sample", rows: int = 1000) -> pd.DataFrame:
        """Crea datos de ejemplo para practicar."""
        if not HAS_PANDAS or not HAS_NUMPY:
            raise ImportError("pandas y numpy requeridos")
        
        np.random.seed(42)
        
        df = pd.DataFrame({
            'id': range(1, rows + 1),
            'fecha': pd.date_range('2023-01-01', periods=rows, freq='H'),
            'categoria': np.random.choice(['A', 'B', 'C', 'D'], rows),
            'region': np.random.choice(['Norte', 'Sur', 'Este', 'Oeste'], rows),
            'ventas': np.random.uniform(100, 10000, rows).round(2),
            'cantidad': np.random.randint(1, 100, rows),
            'precio_unitario': np.random.uniform(10, 500, rows).round(2),
            'descuento': np.random.uniform(0, 0.3, rows).round(2),
            'satisfaccion': np.random.randint(1, 6, rows),
            'es_premium': np.random.choice([True, False], rows, p=[0.3, 0.7]),
        })
        
        # Agregar algunos valores nulos
        null_indices = np.random.choice(rows, size=int(rows * 0.05), replace=False)
        df.loc[null_indices, 'descuento'] = np.nan
        
        self.datasets[name] = df
        self.current_df = df
        
        print(f"✅ Datos de ejemplo creados: {name} ({rows} filas)")
        return df
    
    # =========================================================================
    # ANÁLISIS
    # =========================================================================
    
    def get_info(self, df: pd.DataFrame = None) -> DatasetInfo:
        """Obtiene información detallada del dataset."""
        df = df if df is not None else self.current_df
        if df is None:
            raise ValueError("No hay dataset cargado")
        
        numeric_cols = df.select_dtypes(include=[np.number]).columns.tolist()
        categorical_cols = df.select_dtypes(include=['object', 'category']).columns.tolist()
        
        return DatasetInfo(
            name="current",
            rows=len(df),
            columns=len(df.columns),
            memory_mb=df.memory_usage(deep=True).sum() / (1024 * 1024),
            dtypes={col: str(dtype) for col, dtype in df.dtypes.items()},
            null_counts=df.isnull().sum().to_dict(),
            numeric_columns=numeric_cols,
            categorical_columns=categorical_cols,
        )
    
    def describe(self, df: pd.DataFrame = None) -> str:
        """Genera descripción estadística."""
        df = df if df is not None else self.current_df
        if df is None:
            return "No hay dataset cargado"
        
        info = self.get_info(df)
        
        lines = [
            "📊 **Resumen del Dataset**",
            f"• Filas: {info.rows:,}",
            f"• Columnas: {info.columns}",
            f"• Memoria: {info.memory_mb:.2f} MB",
            "",
            "📋 **Tipos de Datos:**",
        ]
        
        for col, dtype in info.dtypes.items():
            null_count = info.null_counts.get(col, 0)
            null_pct = (null_count / info.rows * 100) if info.rows > 0 else 0
            null_str = f" ({null_count} nulos, {null_pct:.1f}%)" if null_count > 0 else ""
            lines.append(f"  • {col}: {dtype}{null_str}")
        
        lines.extend([
            "",
            f"📈 **Columnas Numéricas:** {len(info.numeric_columns)}",
            f"📝 **Columnas Categóricas:** {len(info.categorical_columns)}",
        ])
        
        return "\n".join(lines)
    
    def quick_stats(self, df: pd.DataFrame = None) -> Dict:
        """Estadísticas rápidas."""
        df = df if df is not None else self.current_df
        if df is None:
            return {}
        
        stats = {
            "filas": len(df),
            "columnas": len(df.columns),
            "valores_nulos_total": int(df.isnull().sum().sum()),
            "duplicados": int(df.duplicated().sum()),
        }
        
        # Stats numéricas
        numeric_df = df.select_dtypes(include=[np.number])
        if not numeric_df.empty:
            stats["estadisticas_numericas"] = numeric_df.describe().to_dict()
        
        return stats
    
    def correlation_analysis(self, df: pd.DataFrame = None) -> pd.DataFrame:
        """Análisis de correlación."""
        df = df if df is not None else self.current_df
        if df is None:
            raise ValueError("No hay dataset cargado")
        
        numeric_df = df.select_dtypes(include=[np.number])
        return numeric_df.corr()
    
    def find_outliers(self, column: str, df: pd.DataFrame = None, method: str = "iqr") -> pd.DataFrame:
        """Encuentra outliers en una columna."""
        df = df if df is not None else self.current_df
        if df is None:
            raise ValueError("No hay dataset cargado")
        
        if column not in df.columns:
            raise ValueError(f"Columna '{column}' no existe")
        
        data = df[column].dropna()
        
        if method == "iqr":
            Q1 = data.quantile(0.25)
            Q3 = data.quantile(0.75)
            IQR = Q3 - Q1
            lower = Q1 - 1.5 * IQR
            upper = Q3 + 1.5 * IQR
            outliers = df[(df[column] < lower) | (df[column] > upper)]
        elif method == "zscore":
            mean = data.mean()
            std = data.std()
            outliers = df[abs((df[column] - mean) / std) > 3]
        else:
            raise ValueError(f"Método '{method}' no soportado")
        
        return outliers
    
    # =========================================================================
    # VISUALIZACIONES
    # =========================================================================
    
    def plot_distribution(self, column: str, df: pd.DataFrame = None, save: bool = True) -> Optional[str]:
        """Grafica distribución de una columna."""
        if not HAS_MATPLOTLIB:
            return None
        
        df = df if df is not None else self.current_df
        if df is None:
            return None
        
        fig, axes = plt.subplots(1, 2, figsize=(12, 5))
        
        # Histograma
        axes[0].hist(df[column].dropna(), bins=30, edgecolor='black', alpha=0.7)
        axes[0].set_title(f'Distribución de {column}')
        axes[0].set_xlabel(column)
        axes[0].set_ylabel('Frecuencia')
        
        # Boxplot
        axes[1].boxplot(df[column].dropna())
        axes[1].set_title(f'Boxplot de {column}')
        axes[1].set_ylabel(column)
        
        plt.tight_layout()
        
        if save:
            path = self.output_dir / f"dist_{column}.png"
            fig.savefig(path, dpi=150, bbox_inches='tight')
            plt.close(fig)
            return str(path)
        
        plt.show()
        return None
    
    def plot_correlation_heatmap(self, df: pd.DataFrame = None, save: bool = True) -> Optional[str]:
        """Grafica mapa de calor de correlaciones."""
        if not HAS_MATPLOTLIB or not HAS_SEABORN:
            return None
        
        df = df if df is not None else self.current_df
        if df is None:
            return None
        
        corr = self.correlation_analysis(df)
        
        fig, ax = plt.subplots(figsize=(10, 8))
        sns.heatmap(corr, annot=True, cmap='coolwarm', center=0, ax=ax, fmt='.2f')
        ax.set_title('Matriz de Correlación')
        
        plt.tight_layout()
        
        if save:
            path = self.output_dir / "correlation_heatmap.png"
            fig.savefig(path, dpi=150, bbox_inches='tight')
            plt.close(fig)
            return str(path)
        
        plt.show()
        return None
    
    def plot_categorical(self, column: str, df: pd.DataFrame = None, save: bool = True) -> Optional[str]:
        """Grafica distribución de variable categórica."""
        if not HAS_MATPLOTLIB:
            return None
        
        df = df if df is not None else self.current_df
        if df is None:
            return None
        
        fig, axes = plt.subplots(1, 2, figsize=(12, 5))
        
        # Barras
        counts = df[column].value_counts()
        axes[0].bar(counts.index.astype(str), counts.values, edgecolor='black', alpha=0.7)
        axes[0].set_title(f'Distribución de {column}')
        axes[0].set_xlabel(column)
        axes[0].set_ylabel('Frecuencia')
        axes[0].tick_params(axis='x', rotation=45)
        
        # Pie
        axes[1].pie(counts.values, labels=counts.index.astype(str), autopct='%1.1f%%')
        axes[1].set_title(f'Proporción de {column}')
        
        plt.tight_layout()
        
        if save:
            path = self.output_dir / f"cat_{column}.png"
            fig.savefig(path, dpi=150, bbox_inches='tight')
            plt.close(fig)
            return str(path)
        
        plt.show()
        return None
    
    # =========================================================================
    # REPORTES
    # =========================================================================
    
    def generate_report(self, df: pd.DataFrame = None, name: str = "analysis_report") -> str:
        """Genera un reporte completo de análisis."""
        df = df if df is not None else self.current_df
        if df is None:
            return "No hay dataset cargado"
        
        info = self.get_info(df)
        stats = self.quick_stats(df)
        
        report = f"""# 📊 Reporte de Análisis de Datos

**Generado por Metal-Dead** - {datetime.now().strftime("%Y-%m-%d %H:%M")}

---

## 1. Información General

| Métrica | Valor |
|---------|-------|
| Filas | {info.rows:,} |
| Columnas | {info.columns} |
| Memoria | {info.memory_mb:.2f} MB |
| Valores Nulos | {stats['valores_nulos_total']:,} |
| Duplicados | {stats['duplicados']:,} |

## 2. Tipos de Datos

| Columna | Tipo | Nulos |
|---------|------|-------|
"""
        
        for col, dtype in info.dtypes.items():
            null_count = info.null_counts.get(col, 0)
            report += f"| {col} | {dtype} | {null_count} |\n"
        
        report += f"""
## 3. Columnas Numéricas ({len(info.numeric_columns)})

{', '.join(info.numeric_columns) if info.numeric_columns else 'Ninguna'}

## 4. Columnas Categóricas ({len(info.categorical_columns)})

{', '.join(info.categorical_columns) if info.categorical_columns else 'Ninguna'}

## 5. Estadísticas Descriptivas

"""
        
        if info.numeric_columns:
            desc = df[info.numeric_columns].describe()
            report += desc.to_markdown() + "\n"
        
        report += """
---

*Reporte generado automáticamente por Metal-Dead DataAnalyst*
"""
        
        # Guardar reporte
        report_path = self.output_dir / f"{name}.md"
        report_path.write_text(report, encoding='utf-8')
        print(f"📄 Reporte guardado: {report_path}")
        
        return report
    
    def export_to_jupyter(self, df: pd.DataFrame = None, name: str = "analysis") -> str:
        """Exporta análisis a notebook Jupyter."""
        df = df if df is not None else self.current_df
        if df is None:
            return "No hay dataset cargado"
        
        info = self.get_info(df)
        
        notebook = {
            "cells": [
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": [f"# 📊 Análisis de Datos\n\n**Generado por Metal-Dead** - {datetime.now().strftime('%Y-%m-%d')}"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "import pandas as pd\n",
                        "import numpy as np\n",
                        "import matplotlib.pyplot as plt\n",
                        "import seaborn as sns\n",
                        "\n",
                        "pd.set_option('display.max_columns', None)\n",
                        "%matplotlib inline"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 1. Información del Dataset"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        f"# Dataset: {info.rows} filas, {info.columns} columnas\n",
                        "# Cargar tus datos aquí:\n",
                        "# df = pd.read_csv('tu_archivo.csv')\n",
                        "\n",
                        "# Info\n",
                        "# df.info()\n",
                        "# df.head()"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 2. Estadísticas Descriptivas"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": ["# df.describe()"]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 3. Valores Nulos"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Valores nulos por columna\n",
                        "# df.isnull().sum()\n",
                        "\n",
                        "# Visualizar\n",
                        "# sns.heatmap(df.isnull(), cbar=True, yticklabels=False)"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 4. Correlaciones"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Matriz de correlación\n",
                        "# plt.figure(figsize=(10, 8))\n",
                        "# sns.heatmap(df.corr(), annot=True, cmap='coolwarm', center=0)\n",
                        "# plt.title('Matriz de Correlación')\n",
                        "# plt.tight_layout()"
                    ]
                },
            ],
            "metadata": {
                "kernelspec": {
                    "display_name": "Python 3",
                    "language": "python",
                    "name": "python3"
                }
            },
            "nbformat": 4,
            "nbformat_minor": 4
        }
        
        notebook_path = self.output_dir / f"{name}.ipynb"
        notebook_path.write_text(json.dumps(notebook, indent=2), encoding='utf-8')
        print(f"📓 Notebook guardado: {notebook_path}")
        
        return str(notebook_path)


# =============================================================================
# DEMO
# =============================================================================

def demo():
    """Demo del analista de datos."""
    print("\n" + "=" * 60)
    print("   📊 Demo de Data Analyst")
    print("   Metal-Dead")
    print("=" * 60)
    
    if not HAS_PANDAS:
        print("\n❌ Instala pandas: pip install pandas")
        return
    
    analyst = DataAnalyst()
    
    # Crear datos de ejemplo
    print("\n📝 Creando datos de ejemplo...")
    df = analyst.create_sample_data("ventas", rows=500)
    
    # Mostrar descripción
    print("\n" + analyst.describe())
    
    # Generar reporte
    print("\n📄 Generando reporte...")
    analyst.generate_report(name="demo_report")
    
    # Exportar a Jupyter
    print("\n📓 Exportando a Jupyter...")
    analyst.export_to_jupyter(name="demo_analysis")
    
    print("\n" + "=" * 60)
    print("   ✅ Demo completada")
    print(f"   📁 Archivos en: {analyst.output_dir}")
    print("=" * 60)


if __name__ == "__main__":
    demo()
