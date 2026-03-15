tok_vocab_size = 200

def tok_encode_char(code):
    if code >= 97:
        if code <= 122:
            return code - 97 + 10
    if code >= 65:
        if code <= 90:
            return code - 65 + 10
    if code >= 48:
        if code <= 57:
            return code - 48 + 40
    if code == 32:
        return 50
    if code == 46:
        return 51
    if code == 44:
        return 52
    if code == 33:
        return 53
    if code == 63:
        return 54
    return 2

def tok_decode_id(t):
    if t >= 10:
        if t < 36:
            return t - 10 + 97
    if t >= 40:
        if t < 50:
            return t - 40 + 48
    if t == 50:
        return 32
    if t == 51:
        return 46
    if t == 52:
        return 44
    if t == 53:
        return 33
    if t == 54:
        return 63
    return 63

def tok_hash_word(length, first_char):
    h = length * 31 + first_char
    h = h * 17 + 13
    return h % tok_vocab_size

c1 = tok_encode_char(104)
c2 = tok_encode_char(111)
c3 = tok_encode_char(108)
c4 = tok_encode_char(97)
print(f"vocab: {tok_vocab_size} tokens")
print(f"encode h={c1} o={c2} l={c3} a={c4}")
d1 = tok_decode_id(c1)
d2 = tok_decode_id(c2)
print(chr(d1))
print(chr(d2))
wh = tok_hash_word(4, 104)
print(f"hash hola = {wh}")
print("tokenizer ok")
