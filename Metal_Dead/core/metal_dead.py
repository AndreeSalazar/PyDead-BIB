VOCAB = 200
EMBED = 64
LAYERS = 4
HIDDEN = 128
NUM_CATEGORIES = 6
SEQ_LEN = 8
BENCH_ITERS = 50

total_inferences = 0
total_tokens_gen = 0
total_score_sum = 0
total_score_count = 0


def md_hash(length):
    h = length * 31 + 7
    h = h * 17 + 13
    h = h ^ (h * 5 + 3)
    h = h % 65536
    return h


def md_forward(token_id, vocab, layers):
    h = token_id * 31 + 7
    i = 0
    while i < layers:
        h = h * 17 + 13
        h = h ^ (h // 256)
        h = h % 65536
        i = i + 1
    return h % vocab


def md_respond(rid):
    if rid == 0:
        msg = "Hola! Soy tu IA compilada con PyDead-BIB"
    elif rid == 1:
        msg = "Interesante pregunta. Dejame pensar..."
    elif rid == 2:
        msg = "PyDead-BIB compila Python a x86-64 nativo"
    elif rid == 3:
        msg = "Puedo ayudarte con programacion e IA"
    elif rid == 4:
        msg = "Binario puro sin CPython ni runtime"
    elif rid == 5:
        msg = "Creado por Eddi Andree Salazar Matos"
    elif rid == 6:
        msg = "Cero dependencias. Solo x86-64 nativo"
    elif rid == 7:
        msg = "Mi cerebro es un transformer compilado"
    elif rid == 8:
        msg = "Puedo recordar y aprender sobre ti"
    elif rid == 9:
        msg = "PyDead-BIB hereda 8 generaciones de compiladores"
    elif rid == 10:
        msg = "SIMD AVX2 para vectorizacion nativa"
    elif rid == 11:
        msg = "GPU CUDA + CPU SIMD hibrido"
    elif rid == 12:
        msg = "async/await + generators nativos"
    elif rid == 13:
        msg = "Optimizer: constant folding + dead code elim"
    elif rid == 14:
        msg = "Register allocator con graph-coloring"
    elif rid == 15:
        msg = "Loop unrolling + strength reduction activos"
    elif rid == 16:
        msg = "SSA form + phi-node insertion nativa"
    elif rid == 17:
        msg = "Peephole optimizer en backend x86-64"
    elif rid == 18:
        msg = "JIT tiering: interprete -> baseline -> opt"
    elif rid == 19:
        msg = "GC generacional con write barriers"
    else:
        msg = "Estoy aqui para ayudarte!"
    print(f"  Metal-Dead: {msg}")
    return msg


def md_calc_ram(vocab, embed, layers, hidden):
    ep = vocab * embed
    lp = layers * (4 * embed * embed + 2 * embed * hidden)
    op = embed * vocab
    total = ep + lp + op
    return (total * 4) // 1024


def md_think(input_len):
    h = md_hash(input_len)
    steps = 1 + (h % 4)
    chain = h
    i = 0
    while i < steps:
        chain = chain * 13 + 7
        chain = chain ^ (chain // 128)
        chain = chain % 65536
        i = i + 1
    confidence = 60 + (chain % 40)
    depth = steps
    return confidence, depth


def md_classify(input_len):
    h = md_hash(input_len)
    cat_id = h % NUM_CATEGORIES
    if cat_id == 0:
        label = "pregunta"
    elif cat_id == 1:
        label = "comando"
    elif cat_id == 2:
        label = "saludo"
    elif cat_id == 3:
        label = "codigo"
    elif cat_id == 4:
        label = "opinion"
    else:
        label = "otro"
    strength = 50 + (h % 50)
    return label, cat_id, strength


def md_generate(seed, length, vocab, layers):
    global total_tokens_gen
    tokens = []
    h = seed
    i = 0
    while i < length:
        h = md_forward(h, vocab, layers)
        tokens.append(h)
        h = h * 7 + 3
        i = i + 1
    total_tokens_gen = total_tokens_gen + length
    return tokens


def md_score(tokens, vocab):
    global total_score_sum
    global total_score_count
    total = 0
    uniq = []
    i = 0
    while i < len(tokens):
        t = tokens[i]
        total = total + t
        found = 0
        j = 0
        while j < len(uniq):
            if uniq[j] == t:
                found = 1
            j = j + 1
        if found == 0:
            uniq.append(t)
        i = i + 1
    count = len(tokens)
    if count == 0:
        return 0, 0, 0
    avg = total // count
    diversity = (len(uniq) * 100) // count
    range_score = 0
    if count > 1:
        lo = tokens[0]
        hi = tokens[0]
        k = 1
        while k < count:
            if tokens[k] < lo:
                lo = tokens[k]
            if tokens[k] > hi:
                hi = tokens[k]
            k = k + 1
        range_score = ((hi - lo) * 100) // vocab
    quality = (diversity + range_score) // 2
    total_score_sum = total_score_sum + quality
    total_score_count = total_score_count + 1
    return avg, diversity, quality


def md_benchmark(iters, vocab, embed, layers, hidden):
    global total_inferences
    best_q = 0
    worst_q = 100
    sum_q = 0
    sum_conf = 0
    i = 0
    while i < iters:
        seed = (i * 37 + 11) % 1000
        tokens = md_generate(seed, SEQ_LEN, vocab, layers)
        avg, div, quality = md_score(tokens, vocab)
        conf, depth = md_think(seed)
        total_inferences = total_inferences + 1
        sum_q = sum_q + quality
        sum_conf = sum_conf + conf
        if quality > best_q:
            best_q = quality
        if quality < worst_q:
            worst_q = quality
        i = i + 1
    avg_q = sum_q // iters
    avg_conf = sum_conf // iters
    return avg_q, best_q, worst_q, avg_conf


def md_chat(input_len):
    global total_inferences
    total_inferences = total_inferences + 1
    h = md_hash(input_len)
    confidence, depth = md_think(input_len)
    label, cat_id, strength = md_classify(input_len)
    rid = h % 20
    md_respond(rid)
    tokens = md_generate(h, SEQ_LEN, VOCAB, LAYERS)
    avg, diversity, quality = md_score(tokens, VOCAB)
    print(f"    [clase: {label}({strength}%) | confianza: {confidence}% | profundidad: {depth} | calidad: {quality}%]")
    return confidence


# ── main ──
ram = md_calc_ram(VOCAB, EMBED, LAYERS, HIDDEN)

print("============================================================")
print("   Metal-Dead para PyDead-BIB v4.0")
print("   IA Personal Ultra-Eficiente - Compilado NATIVO")
print("   GPU CUDA + CPU SIMD - Sin CPython - Sin Runtime")
print("   Eddi Andree Salazar Matos - Lima, Peru")
print("============================================================")
print(f"   modelo: {ram} KB RAM | vocab: {VOCAB} | embed: {EMBED} | capas: {LAYERS}")
print("")

print("-- Test: hash --")
h0 = md_hash(0)
h1 = md_hash(1)
h2 = md_hash(100)
print(f"  hash(0)={h0}  hash(1)={h1}  hash(100)={h2}")

print("")
print("-- Test: forward pass --")
f0 = md_forward(0, VOCAB, LAYERS)
f1 = md_forward(42, VOCAB, LAYERS)
f2 = md_forward(99, VOCAB, LAYERS)
print(f"  forward(0)={f0}  forward(42)={f1}  forward(99)={f2}")

print("")
print("-- Test: classify --")
inputs = [3, 7, 12, 18, 25, 40]
ci = 0
while ci < len(inputs):
    label, cat_id, strength = md_classify(inputs[ci])
    print(f"  input_len={inputs[ci]} -> {label} (cat={cat_id}, strength={strength}%)")
    ci = ci + 1

print("")
print("-- Test: generate --")
g1 = md_generate(42, SEQ_LEN, VOCAB, LAYERS)
g2 = md_generate(99, SEQ_LEN, VOCAB, LAYERS)
print(f"  seed=42  tokens={g1}")
print(f"  seed=99  tokens={g2}")

print("")
print("-- Test: score --")
avg1, div1, q1 = md_score(g1, VOCAB)
avg2, div2, q2 = md_score(g2, VOCAB)
print(f"  seq1: avg={avg1} diversity={div1}% quality={q1}%")
print(f"  seq2: avg={avg2} diversity={div2}% quality={q2}%")

print("")
print("-- Test: think --")
t1_conf, t1_dep = md_think(5)
t2_conf, t2_dep = md_think(50)
t3_conf, t3_dep = md_think(100)
print(f"  think(5):   confianza={t1_conf}% profundidad={t1_dep}")
print(f"  think(50):  confianza={t2_conf}% profundidad={t2_dep}")
print(f"  think(100): confianza={t3_conf}% profundidad={t3_dep}")

print("")
print("-- Test: respond (20 branches) --")
ri = 0
while ri < 21:
    md_respond(ri)
    ri = ri + 1

print("")
print("-- Test: chat (full pipeline) --")
md_chat(5)
md_chat(10)
md_chat(15)
md_chat(20)
md_chat(25)
md_chat(30)
md_chat(42)

print("")
print("-- Test: benchmark --")
avg_q, best_q, worst_q, avg_conf = md_benchmark(BENCH_ITERS, VOCAB, EMBED, LAYERS, HIDDEN)
print(f"  {BENCH_ITERS} iteraciones completadas")
print(f"  calidad: avg={avg_q}% best={best_q}% worst={worst_q}%")
print(f"  confianza promedio: {avg_conf}%")

print("")
print("-- Resumen --")
print(f"  RAM total: {ram} KB")
print(f"  inferencias: {total_inferences}")
print(f"  tokens generados: {total_tokens_gen}")
if total_score_count > 0:
    global_avg = total_score_sum // total_score_count
    print(f"  calidad global promedio: {global_avg}%")
print("============================================================")
print("metal_dead ok")
