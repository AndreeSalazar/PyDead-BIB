# PyDead-BIB JIT 2.0 Test — Functions
# Ejecutar: pyb run tests/jit_runner/test_jit_functions.py

print("=== PyDead-BIB JIT 2.0: Functions ===")

def add(x, y):
    return x + y

def multiply(a, b):
    return a * b

def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

# Test add
result = add(5, 3)
print("add(5, 3) =")
print(result)

# Test multiply
result2 = multiply(4, 6)
print("multiply(4, 6) =")
print(result2)

# Test factorial
result3 = factorial(5)
print("factorial(5) =")
print(result3)

print("=== PASS ===")
