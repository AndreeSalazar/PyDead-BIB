def ai_hash(length):
    h = length * 31 + 7
    h = h * 17 + 13
    h = h % 65536
    return h

def ai_calc_ram(vocab, embed, layers):
    eb = vocab * embed * 2
    lb = layers * 4 * embed * embed * 2
    ob = embed * vocab * 2
    return (eb + lb + ob) // 1024

def ai_respond(input_len):
    h = ai_hash(input_len)
    confidence = 70 + (h % 30)
    rid = h % 15
    if rid == 0:
        print("AI: Hola! Soy Metal-Dead compilado con PyDead-BIB")
    elif rid == 1:
        print("AI: Interesante. Dejame analizar...")
    elif rid == 2:
        print("AI: PyDead-BIB: Python a x86-64 nativo sin runtime")
    elif rid == 3:
        print("AI: Puedo ayudarte con programacion e IA")
    elif rid == 4:
        print("AI: Binario puro sin CPython")
    elif rid == 5:
        print("AI: Creado por Eddi Andree Salazar Matos")
    elif rid == 6:
        print("AI: Cero dependencias. Solo x86-64")
    elif rid == 7:
        print("AI: Transformer ligero compilado nativo")
    elif rid == 8:
        print("AI: Puedo recordar y aprender sobre ti")
    elif rid == 9:
        print("AI: ADead-BIB: 8 generaciones de compiladores")
    elif rid == 10:
        print("AI: SIMD AVX2 vectorizacion 256 bits")
    elif rid == 11:
        print("AI: Techne License v1.0")
    elif rid == 12:
        print("AI: async/await + generators nativos")
    elif rid == 13:
        print("AI: Optimizer: constant folding + dead code elim")
    else:
        print("AI: Estoy aqui para ayudarte!")
    print(f"  [confianza: {confidence}%]")
    return confidence

def ai_ollama(prompt_len):
    tokens = prompt_len * 2
    if tokens > 200:
        tokens = 200
    print(f"AI (Ollama llama3): {tokens} tokens generados")
    return tokens

ram = ai_calc_ram(100, 64, 2)

print("============================================================")
print("   PyDead-BIB AI Suite v3.0")
print("   Metal-Dead + IA-Personal + Ollama Bridge")
print("   Compilado NATIVO — Sin CPython — Sin Runtime")
print("   Eddi Andree Salazar Matos — Lima, Peru")
print("============================================================")
print(f"   modelo: {ram} KB RAM | vocab: 100 | embed: 64")
print(f"   capas: 2 | ollama: localhost:11434")
print("")

print("--- Aprendizaje ---")
print("AI: interes #1 registrado — compiladores")
print("AI: interes #2 registrado — inteligencia artificial")
print("AI: interes #3 registrado — sistemas operativos")
print("AI: hecho #1 guardado — creador de PyDead-BIB")
print("AI: hecho #2 guardado — Lima Peru")
print("")

print("--- Conversacion (7 turnos) ---")
c1 = ai_respond(5)
c2 = ai_respond(10)
c3 = ai_respond(15)
c4 = ai_respond(20)
c5 = ai_respond(25)
c6 = ai_respond(30)
c7 = ai_respond(35)
print("")

print("--- Ollama LLM (3 queries) ---")
t1 = ai_ollama(20)
t2 = ai_ollama(40)
t3 = ai_ollama(80)
t12 = t1 + t2
total_tokens = t12 + t3
print(f"  total tokens: {total_tokens}")
print("")

print("========== PERFIL ==========")
print("  interacciones: 10")
print("  memorias: 10")
print("  intereses: 3")
print("  hechos: 2")
print("============================")
print("")

print("========== STATS ===========")
print("  razonamientos: 7")
print("  respuestas: 7")
print(f"  modelo: {ram} KB RAM")
print("  ollama queries: 3")
print(f"  ollama tokens: {total_tokens}")
print("============================")
print("")

print("--- Benchmark: 100 razonamientos ---")
bench_step(1)
bench_step(2)
bench_step(3)
bench_step(10)
bench_step(25)
bench_step(50)
bench_step(75)
bench_step(100)
print("  100 razonamientos completados")
print("--- Fin Benchmark ---")
print("")
print("========== FIN DEMO ==========")
print("main ok")
