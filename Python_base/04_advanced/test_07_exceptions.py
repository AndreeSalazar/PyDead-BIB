def test_try_except():
    try:
        x = 1 / 0
    except ZeroDivisionError:
        return True
    return False

def test_try_finally():
    val = 0
    try:
        val += 1
    finally:
        val += 2
    return val == 3

def test_raise_simple():
    try:
        raise ValueError("Error")
    except ValueError:
        return True
    return False

if __name__ == "__main__":
    print(test_try_except())
    print(test_try_finally())
    print(test_raise_simple())
