mdl_vocab = 200
mdl_embed = 64
mdl_layers = 2
mdl_hidden = 128
mdl_heads = 4

def mdl_calc_ram(vocab, embed, layers, hidden):
    embed_p = vocab * embed
    layer_p = layers * (4 * embed * embed + 2 * embed * hidden)
    output_p = embed * vocab
    total = embed_p + layer_p + output_p
    return (total * 4) // 1024

def mdl_forward(token_id, vocab, layers):
    h = token_id * 31 + 7
    i = 0
    while i < layers:
        h = h * 17 + 13
        h = h % 65536
        i = i + 1
    return h % vocab

def mdl_attention(query, key, embed):
    score = query * key
    score = score % 1000
    return score

def mdl_generate(seed, length, vocab, layers):
    h = seed
    i = 0
    while i < length:
        h = mdl_forward(h, vocab, layers)
        i = i + 1
    return h

def mdl_score(a, b):
    diff = a - b
    if diff < 0:
        diff = 0 - diff
    if diff == 0:
        return 100
    if diff < 5:
        return 80
    if diff < 10:
        return 50
    return 10

ram = mdl_calc_ram(mdl_vocab, mdl_embed, mdl_layers, mdl_hidden)
print(f"modelo: {ram} KB RAM")
print(f"vocab={mdl_vocab} embed={mdl_embed} layers={mdl_layers}")
out = mdl_forward(42, mdl_vocab, mdl_layers)
print(f"forward(42) = {out}")
gen = mdl_generate(7, 5, mdl_vocab, mdl_layers)
print(f"generate(7,5) = {gen}")
s = mdl_score(10, 12)
print(f"score(10,12) = {s}")
print("model ok")
