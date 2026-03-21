# PyDead-BIB JIT 2.0 Test — Classes
# Ejecutar: pyb run tests/jit_runner/test_jit_classes.py

print("=== PyDead-BIB JIT 2.0: Classes ===")

class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    
    def sum(self):
        return self.x + self.y

class Counter:
    def __init__(self):
        self.count = 0
    
    def increment(self):
        self.count = self.count + 1
        return self.count

# Test Point
p = Point(10, 20)
print("Point(10, 20).sum() =")
print(p.sum())

# Test Counter
c = Counter()
print("Counter increments:")
print(c.increment())
print(c.increment())
print(c.increment())

print("=== PASS ===")
