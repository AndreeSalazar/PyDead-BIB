def test_e2e_hello_runs():
    print("Welcome to PyDead-BIB E2E Tests")
    return True

def test_e2e_factorial():
    def fact(n):
        return 1 if n <= 1 else n * fact(n - 1)
    return fact(6) == 720

def test_e2e_classes():
    class Engine:
        def start(self):
            return 1

    class Car:
        def __init__(self):
            self.engine = Engine()

        def go(self):
            return self.engine.start()

    c = Car()
    return c.go() == 1

if __name__ == "__main__":
    print(test_e2e_hello_runs())
    print(test_e2e_factorial())
    print(test_e2e_classes())
