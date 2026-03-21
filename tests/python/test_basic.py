# PyDead-BIB Python Basic Tests
# Tests básicos de sintaxis Python

def test_arithmetic():
    x = 10 + 5
    y = x * 2
    z = y - 3
    return z  # Expected: 27

def test_strings():
    s = "Hello"
    t = "World"
    result = s + " " + t
    return result  # Expected: "Hello World"

def test_lists():
    arr = [1, 2, 3, 4, 5]
    total = 0
    for i in arr:
        total = total + i
    return total  # Expected: 15

def test_dict():
    d = {"a": 1, "b": 2, "c": 3}
    return d["b"]  # Expected: 2

def test_conditionals():
    x = 10
    if x > 5:
        return "greater"
    else:
        return "less"

def test_while_loop():
    count = 0
    i = 0
    while i < 10:
        count = count + 1
        i = i + 1
    return count  # Expected: 10

def test_functions():
    def add(a, b):
        return a + b
    return add(3, 4)  # Expected: 7

def test_recursion():
    def factorial(n):
        if n <= 1:
            return 1
        return n * factorial(n - 1)
    return factorial(5)  # Expected: 120

# Run all tests
if __name__ == "__main__":
    print("test_arithmetic:", test_arithmetic())
    print("test_strings:", test_strings())
    print("test_lists:", test_lists())
    print("test_dict:", test_dict())
    print("test_conditionals:", test_conditionals())
    print("test_while_loop:", test_while_loop())
    print("test_functions:", test_functions())
    print("test_recursion:", test_recursion())
