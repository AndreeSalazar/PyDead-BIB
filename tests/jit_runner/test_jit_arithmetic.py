# PyDead-BIB JIT 2.0 Test — Arithmetic
# Ejecutar: pyb run tests/jit_runner/test_jit_arithmetic.py

print("=== PyDead-BIB JIT 2.0: Arithmetic ===")

# INT + INT (Tipos Estrictos)
a = 10
b = 20
c = a + b
print("10 + 20 =")
print(c)

# INT * INT
d = 5 * 7
print("5 * 7 =")
print(d)

# INT - INT
e = 100 - 42
print("100 - 42 =")
print(e)

# INT // INT (floor division)
f = 17 // 3
print("17 // 3 =")
print(f)

# INT % INT (modulo)
g = 17 % 3
print("17 % 3 =")
print(g)

# Negative
h = -50
print("-50 =")
print(h)

print("=== PASS ===")
