tok_vocab_size = 100

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
    return 63

print(f"vocab: {tok_vocab_size}")
c1 = tok_encode_char(104)
c2 = tok_encode_char(111)
c3 = tok_encode_char(108)
c4 = tok_encode_char(97)
print(f"h={c1} o={c2} l={c3} a={c4}")
d1 = tok_decode_id(c1)
d2 = tok_decode_id(c2)
d3 = tok_decode_id(c3)
d4 = tok_decode_id(c4)
print(chr(d1))
print(chr(d2))
print(chr(d3))
print(chr(d4))
print("tokenizer ok")
