# PyDead-BIB JIT 2.0 Test — Loops
# Ejecutar: pyb run tests/jit_runner/test_jit_loops.py

print("=== PyDead-BIB JIT 2.0: Loops ===")

# While loop
print("While loop 1-5:")
i = 1
while i <= 5:
    print(i)
    i = i + 1

# For loop with range
print("For loop 0-4:")
for j in range(5):
    print(j)

# Nested calculation
print("Sum 1-10:")
total = 0
k = 1
while k <= 10:
    total = total + k
    k = k + 1
print(total)

print("=== PASS ===")
