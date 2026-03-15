"""
Gestor de Archivos para Metal-Dead
===================================
Author: Eddi Andreé Salazar Matos
Email: eddi.salazar.dev@gmail.com
Made with ❤️ in Peru 🇵🇪

Gestión inteligente de archivos y carpetas:
- Crear proyectos automáticamente
- Generar estructuras de datos
- Templates para Data Analysis
- Jupyter notebooks
"""

import os
import json
import time
from pathlib import Path
from typing import List, Dict, Optional, Any
from dataclasses import dataclass
from datetime import datetime


@dataclass
class ProjectTemplate:
    """Template de proyecto."""
    name: str
    description: str
    structure: Dict[str, Any]
    files: Dict[str, str]


class FileManager:
    """
    Gestor de archivos inteligente para Metal-Dead.
    Crea proyectos, carpetas y archivos automáticamente.
    """
    
    def __init__(self, base_path: str = None):
        self.base_path = Path(base_path) if base_path else Path.home() / "Metal_Dead_Projects"
        self.base_path.mkdir(parents=True, exist_ok=True)
        
        # Templates predefinidos
        self.templates = self._init_templates()
        
        print(f"📁 FileManager inicializado")
        print(f"   Base: {self.base_path}")
    
    def _init_templates(self) -> Dict[str, ProjectTemplate]:
        """Inicializa templates de proyectos."""
        return {
            "data_analyst": ProjectTemplate(
                name="Data Analyst Project",
                description="Proyecto completo para análisis de datos",
                structure={
                    "data": {"raw": {}, "processed": {}, "external": {}},
                    "notebooks": {},
                    "src": {"utils": {}},
                    "reports": {"figures": {}},
                    "sql": {},
                    "config": {},
                },
                files={
                    "README.md": self._template_readme_data_analyst(),
                    "requirements.txt": self._template_requirements_data_analyst(),
                    "notebooks/01_exploracion.ipynb": self._template_notebook_exploration(),
                    "notebooks/02_limpieza.ipynb": self._template_notebook_cleaning(),
                    "notebooks/03_analisis.ipynb": self._template_notebook_analysis(),
                    "src/__init__.py": "",
                    "src/utils/__init__.py": "",
                    "src/utils/data_loader.py": self._template_data_loader(),
                    "src/utils/visualizations.py": self._template_visualizations(),
                    "sql/queries.sql": self._template_sql_queries(),
                    "config/config.yaml": self._template_config(),
                    ".gitignore": self._template_gitignore(),
                }
            ),
            "python_project": ProjectTemplate(
                name="Python Project",
                description="Proyecto Python estándar",
                structure={
                    "src": {"utils": {}},
                    "tests": {},
                    "docs": {},
                    "data": {},
                },
                files={
                    "README.md": "# Python Project\n\nDescripción del proyecto.",
                    "requirements.txt": "numpy\npandas\n",
                    "src/__init__.py": "",
                    "src/main.py": 'if __name__ == "__main__":\n    print("Hello, World!")\n',
                    "tests/__init__.py": "",
                    ".gitignore": self._template_gitignore(),
                }
            ),
            "ml_project": ProjectTemplate(
                name="Machine Learning Project",
                description="Proyecto de Machine Learning",
                structure={
                    "data": {"raw": {}, "processed": {}, "features": {}},
                    "models": {"trained": {}, "checkpoints": {}},
                    "notebooks": {},
                    "src": {"data": {}, "features": {}, "models": {}, "visualization": {}},
                    "reports": {},
                    "config": {},
                },
                files={
                    "README.md": self._template_readme_ml(),
                    "requirements.txt": self._template_requirements_ml(),
                    "notebooks/01_eda.ipynb": self._template_notebook_exploration(),
                    "notebooks/02_feature_engineering.ipynb": self._template_notebook_features(),
                    "notebooks/03_modeling.ipynb": self._template_notebook_modeling(),
                    "src/__init__.py": "",
                    "src/data/__init__.py": "",
                    "src/models/__init__.py": "",
                    "config/config.yaml": self._template_config_ml(),
                    ".gitignore": self._template_gitignore_ml(),
                }
            ),
        }
    
    # =========================================================================
    # TEMPLATES
    # =========================================================================
    
    def _template_readme_data_analyst(self) -> str:
        return f'''# 📊 Data Analysis Project

**Creado con Metal-Dead** - {datetime.now().strftime("%Y-%m-%d")}

## 📁 Estructura

```
├── data/
│   ├── raw/          # Datos originales
│   ├── processed/    # Datos procesados
│   └── external/     # Datos externos
├── notebooks/
│   ├── 01_exploracion.ipynb
│   ├── 02_limpieza.ipynb
│   └── 03_analisis.ipynb
├── src/
│   └── utils/        # Funciones auxiliares
├── reports/
│   └── figures/      # Gráficos
├── sql/              # Queries SQL
└── config/           # Configuración
```

## 🚀 Inicio Rápido

```bash
# Crear entorno virtual
python -m venv venv
venv\\Scripts\\activate  # Windows

# Instalar dependencias
pip install -r requirements.txt

# Iniciar Jupyter
jupyter notebook
```

## 🛠️ Herramientas

- **Python**: pandas, numpy, matplotlib, seaborn
- **SQL**: PostgreSQL queries
- **Jupyter**: Notebooks interactivos
- **Git**: Control de versiones

## 📈 Workflow

1. **Exploración**: Entender los datos
2. **Limpieza**: Preparar datos
3. **Análisis**: Extraer insights
4. **Visualización**: Comunicar resultados

---
*Generado por Metal-Dead para ADead-BIB*
'''
    
    def _template_requirements_data_analyst(self) -> str:
        return '''# Data Analysis Requirements
pandas>=2.0.0
numpy>=1.24.0
matplotlib>=3.7.0
seaborn>=0.12.0
jupyter>=1.0.0
jupyterlab>=4.0.0
openpyxl>=3.1.0
xlrd>=2.0.0
sqlalchemy>=2.0.0
psycopg2-binary>=2.9.0
python-dotenv>=1.0.0
pyyaml>=6.0.0
scikit-learn>=1.3.0
scipy>=1.11.0
plotly>=5.15.0
'''
    
    def _template_notebook_exploration(self) -> str:
        return json.dumps({
            "cells": [
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["# 📊 Exploración de Datos\n", "\n", "Análisis exploratorio inicial."]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Imports\n",
                        "import pandas as pd\n",
                        "import numpy as np\n",
                        "import matplotlib.pyplot as plt\n",
                        "import seaborn as sns\n",
                        "\n",
                        "# Configuración\n",
                        "pd.set_option('display.max_columns', None)\n",
                        "plt.style.use('seaborn-v0_8-whitegrid')\n",
                        "%matplotlib inline"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 1. Cargar Datos"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Cargar datos\n",
                        "# df = pd.read_csv('../data/raw/datos.csv')\n",
                        "# df.head()"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 2. Información General"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Información del dataset\n",
                        "# print(f'Filas: {len(df)}')\n",
                        "# print(f'Columnas: {len(df.columns)}')\n",
                        "# df.info()"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 3. Estadísticas Descriptivas"]
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
                    "source": ["## 4. Valores Nulos"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Valores nulos\n",
                        "# df.isnull().sum()"
                    ]
                },
            ],
            "metadata": {
                "kernelspec": {
                    "display_name": "Python 3",
                    "language": "python",
                    "name": "python3"
                },
                "language_info": {
                    "name": "python",
                    "version": "3.12.0"
                }
            },
            "nbformat": 4,
            "nbformat_minor": 4
        }, indent=2)
    
    def _template_notebook_cleaning(self) -> str:
        return json.dumps({
            "cells": [
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["# 🧹 Limpieza de Datos\n", "\n", "Preparación y limpieza del dataset."]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "import pandas as pd\n",
                        "import numpy as np\n",
                        "\n",
                        "# Cargar datos\n",
                        "# df = pd.read_csv('../data/raw/datos.csv')"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 1. Manejo de Valores Nulos"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Estrategias para valores nulos\n",
                        "# df['columna'].fillna(df['columna'].mean(), inplace=True)\n",
                        "# df.dropna(subset=['columna_importante'], inplace=True)"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 2. Conversión de Tipos"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Convertir tipos\n",
                        "# df['fecha'] = pd.to_datetime(df['fecha'])\n",
                        "# df['categoria'] = df['categoria'].astype('category')"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 3. Guardar Datos Limpios"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": ["# df.to_csv('../data/processed/datos_limpios.csv', index=False)"]
                },
            ],
            "metadata": {"kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"}},
            "nbformat": 4,
            "nbformat_minor": 4
        }, indent=2)
    
    def _template_notebook_analysis(self) -> str:
        return json.dumps({
            "cells": [
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["# 📈 Análisis de Datos\n", "\n", "Análisis profundo y visualizaciones."]
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
                        "# df = pd.read_csv('../data/processed/datos_limpios.csv')"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 1. Análisis Univariado"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# fig, axes = plt.subplots(2, 2, figsize=(12, 10))\n",
                        "# # Histogramas, boxplots, etc."
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 2. Análisis Bivariado"]
                },
                {
                    "cell_type": "code",
                    "execution_count": None,
                    "metadata": {},
                    "outputs": [],
                    "source": [
                        "# Correlaciones\n",
                        "# sns.heatmap(df.corr(), annot=True, cmap='coolwarm')"
                    ]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["## 3. Conclusiones"]
                },
                {
                    "cell_type": "markdown",
                    "metadata": {},
                    "source": ["- Insight 1\n", "- Insight 2\n", "- Insight 3"]
                },
            ],
            "metadata": {"kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"}},
            "nbformat": 4,
            "nbformat_minor": 4
        }, indent=2)
    
    def _template_notebook_features(self) -> str:
        return json.dumps({
            "cells": [
                {"cell_type": "markdown", "metadata": {}, "source": ["# 🔧 Feature Engineering"]},
                {"cell_type": "code", "execution_count": None, "metadata": {}, "outputs": [], "source": ["import pandas as pd\nimport numpy as np"]},
            ],
            "metadata": {"kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"}},
            "nbformat": 4,
            "nbformat_minor": 4
        }, indent=2)
    
    def _template_notebook_modeling(self) -> str:
        return json.dumps({
            "cells": [
                {"cell_type": "markdown", "metadata": {}, "source": ["# 🤖 Modelado"]},
                {"cell_type": "code", "execution_count": None, "metadata": {}, "outputs": [], "source": ["from sklearn.model_selection import train_test_split\nfrom sklearn.ensemble import RandomForestClassifier"]},
            ],
            "metadata": {"kernelspec": {"display_name": "Python 3", "language": "python", "name": "python3"}},
            "nbformat": 4,
            "nbformat_minor": 4
        }, indent=2)
    
    def _template_data_loader(self) -> str:
        return '''"""
Data Loader Utilities
"""

import pandas as pd
from pathlib import Path


def load_csv(filename: str, data_dir: str = "data/raw") -> pd.DataFrame:
    """Carga un archivo CSV."""
    path = Path(data_dir) / filename
    return pd.read_csv(path)


def load_excel(filename: str, data_dir: str = "data/raw", sheet_name: str = 0) -> pd.DataFrame:
    """Carga un archivo Excel."""
    path = Path(data_dir) / filename
    return pd.read_excel(path, sheet_name=sheet_name)


def save_processed(df: pd.DataFrame, filename: str, data_dir: str = "data/processed"):
    """Guarda datos procesados."""
    path = Path(data_dir) / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    df.to_csv(path, index=False)
    print(f"✅ Guardado: {path}")
'''
    
    def _template_visualizations(self) -> str:
        return '''"""
Visualization Utilities
"""

import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path


def setup_style():
    """Configura estilo de gráficos."""
    plt.style.use('seaborn-v0_8-whitegrid')
    plt.rcParams['figure.figsize'] = (10, 6)
    plt.rcParams['font.size'] = 12


def save_figure(fig, filename: str, dpi: int = 300):
    """Guarda figura."""
    path = Path("reports/figures") / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(path, dpi=dpi, bbox_inches='tight')
    print(f"✅ Figura guardada: {path}")


def plot_distribution(df, column: str, title: str = None):
    """Grafica distribución de una columna."""
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    
    # Histograma
    axes[0].hist(df[column], bins=30, edgecolor='black')
    axes[0].set_title(f'Distribución de {column}')
    axes[0].set_xlabel(column)
    axes[0].set_ylabel('Frecuencia')
    
    # Boxplot
    axes[1].boxplot(df[column].dropna())
    axes[1].set_title(f'Boxplot de {column}')
    
    plt.tight_layout()
    return fig
'''
    
    def _template_sql_queries(self) -> str:
        return '''-- SQL Queries para Data Analysis
-- Generado por Metal-Dead

-- ============================================
-- CONSULTAS BÁSICAS
-- ============================================

-- Seleccionar todos los registros
SELECT * FROM tabla LIMIT 100;

-- Contar registros
SELECT COUNT(*) as total FROM tabla;

-- Agrupar y contar
SELECT columna, COUNT(*) as cantidad
FROM tabla
GROUP BY columna
ORDER BY cantidad DESC;

-- ============================================
-- ANÁLISIS ESTADÍSTICO
-- ============================================

-- Estadísticas básicas
SELECT 
    COUNT(*) as total,
    AVG(valor) as promedio,
    MIN(valor) as minimo,
    MAX(valor) as maximo,
    STDDEV(valor) as desviacion
FROM tabla;

-- Percentiles
SELECT 
    PERCENTILE_CONT(0.25) WITHIN GROUP (ORDER BY valor) as p25,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY valor) as mediana,
    PERCENTILE_CONT(0.75) WITHIN GROUP (ORDER BY valor) as p75
FROM tabla;

-- ============================================
-- JOINS
-- ============================================

-- Inner Join
SELECT a.*, b.columna
FROM tabla_a a
INNER JOIN tabla_b b ON a.id = b.id_a;

-- Left Join
SELECT a.*, b.columna
FROM tabla_a a
LEFT JOIN tabla_b b ON a.id = b.id_a;
'''
    
    def _template_config(self) -> str:
        return '''# Configuración del Proyecto
# Generado por Metal-Dead

database:
  host: localhost
  port: 5432
  name: mi_base_datos
  user: usuario
  # password: en variable de entorno DB_PASSWORD

paths:
  data_raw: data/raw
  data_processed: data/processed
  reports: reports

analysis:
  random_seed: 42
  test_size: 0.2
  
visualization:
  style: seaborn-v0_8-whitegrid
  figsize: [10, 6]
  dpi: 300
'''
    
    def _template_config_ml(self) -> str:
        return '''# ML Project Configuration

model:
  name: random_forest
  params:
    n_estimators: 100
    max_depth: 10
    random_state: 42

training:
  test_size: 0.2
  cv_folds: 5
  
paths:
  data: data
  models: models/trained
  checkpoints: models/checkpoints
'''
    
    def _template_gitignore(self) -> str:
        return '''# Python
__pycache__/
*.py[cod]
*.so
.Python
venv/
.env

# Jupyter
.ipynb_checkpoints/

# Data
*.csv
*.xlsx
*.xls
*.parquet
data/raw/*
data/processed/*
!data/raw/.gitkeep
!data/processed/.gitkeep

# IDE
.vscode/
.idea/

# OS
.DS_Store
Thumbs.db
'''
    
    def _template_gitignore_ml(self) -> str:
        return self._template_gitignore() + '''
# Models
models/trained/*
models/checkpoints/*
!models/trained/.gitkeep
!models/checkpoints/.gitkeep
*.pkl
*.h5
*.pt
*.pth
'''
    
    def _template_readme_ml(self) -> str:
        return f'''# 🤖 Machine Learning Project

**Creado con Metal-Dead** - {datetime.now().strftime("%Y-%m-%d")}

## 📁 Estructura

```
├── data/
│   ├── raw/
│   ├── processed/
│   └── features/
├── models/
│   ├── trained/
│   └── checkpoints/
├── notebooks/
├── src/
└── config/
```

## 🚀 Workflow

1. EDA (Exploratory Data Analysis)
2. Feature Engineering
3. Model Training
4. Evaluation
5. Deployment

---
*Generado por Metal-Dead*
'''
    
    def _template_requirements_ml(self) -> str:
        return '''# ML Requirements
pandas>=2.0.0
numpy>=1.24.0
scikit-learn>=1.3.0
matplotlib>=3.7.0
seaborn>=0.12.0
jupyter>=1.0.0
xgboost>=2.0.0
lightgbm>=4.0.0
optuna>=3.3.0
mlflow>=2.7.0
'''
    
    # =========================================================================
    # MÉTODOS PRINCIPALES
    # =========================================================================
    
    def create_project(self, name: str, template: str = "data_analyst", path: str = None) -> Path:
        """
        Crea un proyecto completo desde template.
        
        Args:
            name: Nombre del proyecto
            template: Tipo de template
            path: Ruta base (opcional)
        """
        if template not in self.templates:
            raise ValueError(f"Template '{template}' no existe. Disponibles: {list(self.templates.keys())}")
        
        tmpl = self.templates[template]
        base = Path(path) if path else self.base_path
        project_path = base / name
        
        print(f"\n📁 Creando proyecto: {name}")
        print(f"   Template: {tmpl.name}")
        print(f"   Ruta: {project_path}")
        
        # Crear estructura de carpetas
        self._create_structure(project_path, tmpl.structure)
        
        # Crear archivos
        for file_path, content in tmpl.files.items():
            full_path = project_path / file_path
            full_path.parent.mkdir(parents=True, exist_ok=True)
            full_path.write_text(content, encoding='utf-8')
            print(f"   ✅ {file_path}")
        
        # Crear .gitkeep en carpetas vacías
        self._add_gitkeep(project_path)
        
        print(f"\n✅ Proyecto '{name}' creado exitosamente!")
        return project_path
    
    def _create_structure(self, base: Path, structure: Dict, indent: int = 0):
        """Crea estructura de carpetas recursivamente."""
        for name, substructure in structure.items():
            path = base / name
            path.mkdir(parents=True, exist_ok=True)
            if substructure:
                self._create_structure(path, substructure, indent + 1)
    
    def _add_gitkeep(self, path: Path):
        """Agrega .gitkeep a carpetas vacías."""
        for dir_path in path.rglob('*'):
            if dir_path.is_dir() and not any(dir_path.iterdir()):
                (dir_path / '.gitkeep').touch()
    
    def create_folder(self, name: str, path: str = None) -> Path:
        """Crea una carpeta."""
        base = Path(path) if path else self.base_path
        folder = base / name
        folder.mkdir(parents=True, exist_ok=True)
        print(f"📁 Carpeta creada: {folder}")
        return folder
    
    def create_file(self, filename: str, content: str = "", path: str = None) -> Path:
        """Crea un archivo."""
        base = Path(path) if path else self.base_path
        file_path = base / filename
        file_path.parent.mkdir(parents=True, exist_ok=True)
        file_path.write_text(content, encoding='utf-8')
        print(f"📄 Archivo creado: {file_path}")
        return file_path
    
    def list_templates(self) -> List[str]:
        """Lista templates disponibles."""
        return list(self.templates.keys())
    
    def get_template_info(self, template: str) -> str:
        """Obtiene información de un template."""
        if template not in self.templates:
            return f"Template '{template}' no existe."
        
        tmpl = self.templates[template]
        return f"📋 **{tmpl.name}**\n{tmpl.description}\n\nArchivos: {len(tmpl.files)}"


# =============================================================================
# DEMO
# =============================================================================

def demo():
    """Demo del gestor de archivos."""
    print("\n" + "=" * 60)
    print("   📁 Demo de File Manager")
    print("   Metal-Dead")
    print("=" * 60)
    
    fm = FileManager()
    
    print("\n📋 Templates disponibles:")
    for name in fm.list_templates():
        print(f"   • {name}")
    
    print("\n" + fm.get_template_info("data_analyst"))


if __name__ == "__main__":
    demo()
