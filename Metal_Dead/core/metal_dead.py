def md_hash(length):
    h = length * 31 + 7
    h = h * 17 + 13
    h = h % 65536
    return h

def md_forward(token_id, vocab, layers):
    h = token_id * 31 + 7
    i = 0
    while i < layers:
        h = h * 17 + 13
        h = h % 65536
        i = i + 1
    return h % vocab

def md_respond(rid):
    if rid == 0:
        print("Metal-Dead: Hola! Soy tu IA compilada con PyDead-BIB")
    elif rid == 1:
        print("Metal-Dead: Interesante pregunta. Dejame pensar...")
    elif rid == 2:
        print("Metal-Dead: PyDead-BIB compila Python a x86-64 nativo")
    elif rid == 3:
        print("Metal-Dead: Puedo ayudarte con programacion e IA")
    elif rid == 4:
        print("Metal-Dead: Binario puro sin CPython ni runtime")
    elif rid == 5:
        print("Metal-Dead: Creado por Eddi Andree Salazar Matos")
    elif rid == 6:
        print("Metal-Dead: Cero dependencias. Solo x86-64 nativo")
    elif rid == 7:
        print("Metal-Dead: Mi cerebro es un transformer compilado")
    elif rid == 8:
        print("Metal-Dead: Puedo recordar y aprender sobre ti")
    elif rid == 9:
        print("Metal-Dead: PyDead-BIB hereda 8 generaciones de compiladores")
    elif rid == 10:
        print("Metal-Dead: SIMD AVX2 para vectorizacion nativa")
    elif rid == 11:
        print("Metal-Dead: GPU CUDA + CPU SIMD hibrido")
    elif rid == 12:
        print("Metal-Dead: async/await + generators nativos")
    elif rid == 13:
        print("Metal-Dead: Optimizer: constant folding + dead code elim")
    else:
        print("Metal-Dead: Estoy aqui para ayudarte!")

def md_calc_ram(vocab, embed, layers, hidden):
    ep = vocab * embed
    lp = layers * (4 * embed * embed + 2 * embed * hidden)
    op = embed * vocab
    total = ep + lp + op
    return (total * 4) // 1024

def md_think(input_len):
    h = md_hash(input_len)
    confidence = 70 + (h % 30)
    return confidence

def md_chat(input_len):
    h = md_hash(input_len)
    confidence = md_think(input_len)
    rid = h % 15
    md_respond(rid)
    print(f"  [confianza: {confidence}%]")
    return confidence

ram = md_calc_ram(200, 64, 2, 128)
print("============================================================")
print("   Metal-Dead para PyDead-BIB v3.0")
print("   IA Personal Ultra-Eficiente — Compilado NATIVO")
print("   GPU CUDA + CPU SIMD — Sin CPython — Sin Runtime")
print("   Eddi Andree Salazar Matos — Lima, Peru")
print("============================================================")
print(f"   modelo: {ram} KB RAM | vocab: 200 | embed: 64")
print("")
md_chat(5)
md_chat(10)
md_chat(15)
md_chat(20)
md_chat(25)
md_chat(30)
md_chat(42)
print("")
print(f"RAM total: {ram} KB")
print("metal_dead ok")
