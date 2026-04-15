def test_print_string():
    print("Hello, PyDead-BIB JIT 2.0!")
    return True

def test_print_empty():
    print()
    return True

def test_hello_compilable():
    x = "Compilable"
    print(x)
    return True

if __name__ == "__main__":
    test_print_string()
    test_print_empty()
    test_hello_compilable()
