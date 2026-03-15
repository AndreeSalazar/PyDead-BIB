def cpu_hash(length):
    h = length * 31 + 7
    h = h * 17 + 13
    h = h % 65536
    return h

def cpu_forward(token_id, vocab, layers):
    h = token_id * 31 + 7
    i = 0
    while i < layers:
        h = h * 17 + 13
        h = h % 65536
        i = i + 1
    return h % vocab

def cpu_matmul_sim(rows, cols, depth):
    ops = rows * cols * depth * 2
    return ops

def cpu_simd_avx2(elements):
    lanes = 8
    iterations = elements // lanes
    remaining = elements % lanes
    total_ops = iterations * lanes + remaining
    return total_ops

def cpu_benchmark(iterations, vocab, layers):
    i = 0
    while i < iterations:
        cpu_forward(i, vocab, layers)
        i = i + 1
    return iterations

print("============================================================")
print("   Metal-Dead CPU — SIMD AVX2 Optimizado")
print("   PyDead-BIB v3.0 — Sin Runtime")
print("============================================================")
print("")
ops1 = cpu_matmul_sim(64, 64, 64)
print(f"matmul 64x64x64: {ops1} ops")
simd1 = cpu_simd_avx2(256)
print(f"SIMD AVX2 256 elem: {simd1} ops")
out1 = cpu_forward(42, 200, 2)
print(f"forward(42) = {out1}")
out2 = cpu_forward(100, 200, 4)
print(f"forward(100) = {out2}")
cpu_benchmark(50, 200, 2)
print("  50 iteraciones benchmark completadas")
print("")
print("metal_dead_cpu ok")
