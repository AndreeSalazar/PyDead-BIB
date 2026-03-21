# PyDead-BIB JIT 2.0 Test — Classes
# Ejecutar: pyb run tests/jit_runner/test_jit_classes.py

print("=== PyDead-BIB JIT 2.0: Classes ===")

class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    
    def get_x(self):
        return self.x

# Test Point creation
p = Point(10, 20)
print("Point created")

# Test method call - store result first
result = p.get_x()
print("get_x() =")
print(result)

print("=== PASS ===")
