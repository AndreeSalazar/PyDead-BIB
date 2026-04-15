import sys

def test_import_module():
    return sys is not None

def test_main_guard():
    # Solo ver que compila
    return __name__ is not None

if __name__ == "__main__":
    print(test_import_module())
    print(test_main_guard())
