class SimpleClass:
    def __init__(self, x):
        self.x = x

    def get_x(self):
        return self.x

def test_class_simple():
    obj = SimpleClass(42)
    return obj.get_x() == 42


class Base:
    def identify(self):
        return "Base"

class Derived(Base):
    def identify(self):
        return "Derived"

def test_inheritance_simple():
    obj = Derived()
    return obj.identify() == "Derived"

class MathBox:
    @staticmethod
    def add(a, b):
        return a + b

def test_staticmethod():
    return MathBox.add(10, 10) == 20

if __name__ == "__main__":
    print(test_class_simple())
    print(test_inheritance_simple())
    print(test_staticmethod())
