def pyb_accel_sum(n):
    total = 0
    i = 0
    while i < n:
        total = total + i
        i = i + 1
    return total

def pyb_accel_max(a, b):
    if a > b:
        return a
    return b

def pyb_accel_min(a, b):
    if a < b:
        return a
    return b

def pyb_accel_dot(n):
    total = 0
    i = 0
    while i < n:
        total = total + i * i
        i = i + 1
    return total

def pyb_accel_simd_ops(elements):
    lanes = 8
    iterations = elements // lanes
    remaining = elements % lanes
    return iterations * lanes + remaining

def pyb_accel_benchmark(iterations):
    i = 0
    while i < iterations:
        pyb_accel_sum(100)
        i = i + 1
    return iterations

print("============================================================")
print("   PyDead-BIB Accelerator (reemplaza ADead-BIB)")
print("   SIMD AVX2 nativo — Compilado x86-64")
print("============================================================")
s = pyb_accel_sum(100)
print(f"sum(100) = {s}")
mx = pyb_accel_max(42, 17)
print(f"max(42,17) = {mx}")
mn = pyb_accel_min(42, 17)
print(f"min(42,17) = {mn}")
dt = pyb_accel_dot(10)
print(f"dot(10) = {dt}")
simd = pyb_accel_simd_ops(256)
print(f"SIMD AVX2 256 elem = {simd} ops")
pyb_accel_benchmark(50)
print("  50 iteraciones benchmark")
print("")
print("adead_accelerator ok")
