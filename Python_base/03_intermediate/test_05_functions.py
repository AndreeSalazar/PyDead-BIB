def test_def_simple():
    def add(a, b):
        return a + b
    return add(2, 3) == 5

def test_def_params_default(a=10):
    return a == 10

def test_recursion_factorial():
    def fact(n):
        if n <= 1:
            return 1
        return n * fact(n - 1)
    return fact(5) == 120

def test_closure_simple():
    def outer(x):
        def inner(y):
            return x + y
        return inner
    add5 = outer(5)
    return add5(5) == 10

if __name__ == "__main__":
    print(test_def_simple())
    print(test_def_params_default())
    print(test_recursion_factorial())
    print(test_closure_simple())
