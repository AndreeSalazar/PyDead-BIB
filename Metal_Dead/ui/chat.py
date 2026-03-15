def chat_hash(length):
    h = length * 31 + 7
    h = h * 17 + 13
    h = h % 65536
    return h

def chat_respond(rid):
    if rid == 0:
        print("Metal-Dead: Hola! Soy tu IA personal PyDead-BIB")
    elif rid == 1:
        print("Metal-Dead: Interesante. Dejame pensar...")
    elif rid == 2:
        print("Metal-Dead: PyDead-BIB compila Python a x86-64 nativo")
    elif rid == 3:
        print("Metal-Dead: Puedo ayudarte con programacion e IA")
    elif rid == 4:
        print("Metal-Dead: Binario puro sin CPython")
    elif rid == 5:
        print("Metal-Dead: Creado por Eddi Andree Salazar Matos")
    elif rid == 6:
        print("Metal-Dead: Cero dependencias. Solo x86-64")
    elif rid == 7:
        print("Metal-Dead: GPU CUDA + CPU SIMD hibrido")
    elif rid == 8:
        print("Metal-Dead: Puedo recordar y aprender sobre ti")
    elif rid == 9:
        print("Metal-Dead: PyDead-BIB: 8 generaciones de compiladores")
    elif rid == 10:
        print("Metal-Dead: SIMD AVX2 vectorizacion nativa")
    else:
        print("Metal-Dead: Estoy aqui para ayudarte!")

def chat_process(input_len):
    h = chat_hash(input_len)
    confidence = 70 + (h % 30)
    rid = h % 11
    chat_respond(rid)
    print(f"  [confianza: {confidence}%]")
    return confidence

def chat_help():
    print("Comandos:")
    print("  ayuda    — muestra este mensaje")
    print("  perfil   — muestra tu perfil")
    print("  memoria  — estadisticas de memoria")
    print("  salir    — terminar chat")

def chat_profile():
    print("Perfil de usuario:")
    print("  nombre: Andree")
    print("  idioma: es")
    print("  intereses: compiladores, IA, sistemas")

print("============================================================")
print("   Metal-Dead Chat — PyDead-BIB v3.0")
print("   Escribe 'salir' para terminar")
print("============================================================")
print("")
chat_process(5)
chat_process(12)
chat_process(20)
chat_process(35)
chat_process(42)
print("")
chat_help()
print("")
chat_profile()
print("")
print("chat ok")
