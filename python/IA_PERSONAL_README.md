# 🤖 IA-Personal para ADead-BIB

**Sistema de IA Personal Ultra-Ligero**

> Tu asistente personal que aprende de ti, recuerda tus conversaciones y se integra con ADead-BIB para máximo rendimiento.

---

## 🇵🇪 Made with ❤️ in Peru

**Author:** Eddi Andreé Salazar Matos  
**Email:** eddi.salazar.dev@gmail.com

---

## ✨ Características

| Característica | Descripción |
|----------------|-------------|
| **Memoria Persistente** | Recuerda conversaciones entre sesiones |
| **Contexto Personal** | Aprende tu nombre, intereses y preferencias |
| **Aprendizaje Continuo** | Mejora con cada interacción |
| **Ultra-Ligero** | Solo ~0.5 MB de RAM |
| **100% Privado** | Todo se procesa localmente |
| **Integración ADead-BIB** | Operaciones aceleradas sin runtime |

---

## 🚀 Inicio Rápido

### Modo Interactivo (Chat)
```powershell
cd python
python ia_personal.py
```

### Demo Completa
```powershell
python ia_personal.py --demo
```

### Benchmark de Rendimiento
```powershell
python ia_personal.py --benchmark
```

### Chat con Interfaz Mejorada
```powershell
python ia_personal_chat.py
```

### Modo Turbo (Más Rápido)
```powershell
python ia_personal_chat.py --turbo
```

---

## 📁 Archivos del Sistema

```
python/
├── ia_personal.py          # Sistema principal de IA Personal
├── ia_personal_adead.py    # Integración con ADead-BIB
├── ia_personal_chat.py     # Interfaz de chat mejorada
├── IA_PERSONAL_README.md   # Esta documentación
└── ia_personal_data/       # Datos persistentes (auto-generado)
    ├── memories.json       # Memorias guardadas
    ├── profile.json        # Perfil del usuario
    ├── adead_cache/        # Cache de binarios compilados
    └── exports/            # Conversaciones exportadas
```

---

## 💬 Comandos de Chat

### Comandos Especiales
| Comando | Descripción |
|---------|-------------|
| `/ayuda` o `/help` | Muestra ayuda |
| `/memoria` | Estadísticas de memoria |
| `/perfil` | Tu perfil personal |
| `/buscar [texto]` | Busca en memorias |
| `/exportar` | Exporta la conversación |
| `/stats` | Estadísticas del sistema |
| `/limpiar` | Limpia la pantalla |
| `/salir` | Termina el chat |

### Frases de Aprendizaje
| Frase | Acción |
|-------|--------|
| "Me llamo [nombre]" | Aprende tu nombre |
| "Mi nombre es [nombre]" | Aprende tu nombre |
| "Me gusta [algo]" | Aprende tus intereses |
| "Me interesa [algo]" | Aprende tus intereses |
| "Recuerda que [algo]" | Guarda información |
| "No olvides que [algo]" | Guarda información |

---

## 📊 Rendimiento

### Especificaciones
| Métrica | Valor |
|---------|-------|
| **RAM Total** | ~0.5-0.7 MB |
| **Vocabulario** | 289+ tokens |
| **Embeddings** | 128 dimensiones |
| **Capas Transformer** | 2 |
| **Tiempo de Respuesta** | <100 ms (reglas) |

### Benchmark de Memoria
| Operación | Tiempo |
|-----------|--------|
| Agregar 100 items | ~97 ms |
| Buscar 100 veces | ~11 ms |

---

## 🔧 Configuración Avanzada

### Personalizar Configuración
```python
from ia_personal import IAPersonal, IAPersonalConfig

config = IAPersonalConfig(
    vocab_size=15000,      # Tamaño del vocabulario
    embed_dim=128,         # Dimensión de embeddings
    num_heads=8,           # Cabezas de atención
    hidden_dim=256,        # Dimensión oculta FFN
    num_layers=2,          # Capas transformer
    temperature=0.7,       # Temperatura de generación
    max_memory_items=1000, # Máximo de memorias
)

ia = IAPersonal(config)
ia.interactive()
```

### Usar con Aceleración ADead-BIB
```python
from ia_personal_adead import IAPersonalADead

ia = IAPersonalADead()
ia.chat("Hola, soy tu asistente")
```

---

## 🧠 Arquitectura

```
┌─────────────────────────────────────────────────────────────┐
│                     IA-Personal                              │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ Tokenizer   │  │  Memory     │  │  Context    │         │
│  │ (Smart)     │  │ (Persistent)│  │ (Personal)  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│         │                │                │                 │
│         └────────────────┼────────────────┘                 │
│                          │                                  │
│                  ┌───────▼───────┐                         │
│                  │  Transformer  │                         │
│                  │   (Light)     │                         │
│                  └───────────────┘                         │
│                          │                                  │
│              ┌───────────┼───────────┐                     │
│              │                       │                      │
│      ┌───────▼───────┐      ┌───────▼───────┐             │
│      │ Rule-Based    │      │ ADead-BIB     │             │
│      │ Responses     │      │ Accelerator   │             │
│      └───────────────┘      └───────────────┘             │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔒 Privacidad

- **100% Local**: Todos los datos se almacenan en tu máquina
- **Sin Internet**: No requiere conexión para funcionar
- **Sin Telemetría**: No se envían datos a ningún servidor
- **Datos Tuyos**: Puedes ver, editar o eliminar tus datos en cualquier momento

### Ubicación de Datos
```
python/ia_personal_data/
├── memories.json    # Tus memorias (editable)
├── profile.json     # Tu perfil (editable)
└── exports/         # Conversaciones exportadas
```

---

## 🛠️ Integración con ADead-BIB

IA-Personal se integra con el compilador ADead-BIB para:

1. **Operaciones Matemáticas Rápidas**: Producto punto, softmax, GELU
2. **Compilación a Binarios**: Funciones críticas compiladas a código nativo
3. **Sin Runtime**: Ejecución directa sin overhead
4. **Cache de Binarios**: Reutilización de compilaciones

### Ejemplo de Aceleración
```python
from ia_personal_adead import IAPersonalADead

ia = IAPersonalADead()

# Benchmark de aceleración
ia.benchmark_acceleration()

# Ver estadísticas
stats = ia.get_acceleration_stats()
print(f"Compilaciones: {stats['compilations']}")
print(f"Cache hits: {stats['cache_hits']}")
```

---

## 📈 Roadmap

- [x] Memoria persistente
- [x] Contexto personal
- [x] Aprendizaje de patrones
- [x] Integración ADead-BIB
- [x] Interfaz de chat mejorada
- [ ] Entrenamiento del modelo transformer
- [ ] Integración con Ollama para respuestas avanzadas
- [ ] Soporte multi-idioma mejorado
- [ ] API REST para integración externa

---

## 📝 Licencia

Apache 2.0 - Libre para uso personal y comercial.

---

## 🤝 Contribuir

¡Las contribuciones son bienvenidas! Este proyecto es parte de ADead-BIB.

```bash
git clone https://github.com/yourusername/ADead-BIB.git
cd ADead-BIB/python
python ia_personal.py --demo
```

---

**¡Disfruta tu IA Personal!** 🚀
