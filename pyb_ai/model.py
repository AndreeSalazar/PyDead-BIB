mdl_vocab = 100
mdl_embed = 64
mdl_layers = 2

def mdl_calc_ram(vocab, embed, layers):
    eb = vocab * embed * 2
    lb = layers * 4 * embed * embed * 2
    ob = embed * vocab * 2
    return (eb + lb + ob) // 1024

def mdl_forward(token_id, vocab, layers):
    h = token_id * 31 + 7
    i = 0
    while i < layers:
        h = h * 17 + 13
        h = h % 65536
        i = i + 1
    return h % vocab

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

ram = mdl_calc_ram(mdl_vocab, mdl_embed, mdl_layers)
print(f"modelo: {ram} KB RAM")
out = mdl_forward(42, mdl_vocab, mdl_layers)
print(f"forward(42) = {out}")
out2 = mdl_forward(100, mdl_vocab, mdl_layers)
print(f"forward(100) = {out2}")
out3 = mdl_forward(7, mdl_vocab, mdl_layers)
print(f"forward(7) = {out3}")
s1 = mdl_score(10, 12)
print(f"score(10,12) = {s1}")
s2 = mdl_score(50, 50)
print(f"score(50,50) = {s2}")
s3 = mdl_score(1, 100)
print(f"score(1,100) = {s3}")
print("model ok")
