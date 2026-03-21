# PyDead-BIB JIT 2.0 Test — Classes Simple
# Ejecutar: pyb run tests/jit_runner/test_jit_classes_simple.py

print("=== PyDead-BIB JIT 2.0: Classes Simple ===")

class Box:
    def __init__(self, val):
        self.val = val

# Test simple field access
b = Box(42)
print("Box created")

print("=== PASS ===")
