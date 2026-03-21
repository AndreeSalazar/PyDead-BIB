# PyDead-BIB JIT 2.0 Test — Conditionals
# Ejecutar: pyb run tests/jit_runner/test_jit_conditionals.py

print("=== PyDead-BIB JIT 2.0: Conditionals ===")

# Simple if
x = 10
if x > 5:
    print("x > 5: True")

# If-else
y = 3
if y > 5:
    print("y > 5")
else:
    print("y <= 5")

# If-elif-else
z = 50
if z < 25:
    print("small")
elif z < 75:
    print("medium")
else:
    print("large")

# Comparison operators
a = 10
b = 20
if a < b:
    print("a < b: True")
if a != b:
    print("a != b: True")
if a <= b:
    print("a <= b: True")

print("=== PASS ===")
