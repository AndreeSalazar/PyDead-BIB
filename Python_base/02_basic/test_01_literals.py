def test_literal_int_positive():
    x = 42
    return x == 42

def test_literal_float_simple():
    y = 3.1415
    return y > 3.0

def test_literal_str_double():
    s = "PyDead"
    return len(s) == 6

def test_literal_bool_true():
    b = True
    return b

if __name__ == "__main__":
    print(test_literal_int_positive())
    print(test_literal_float_simple())
    print(test_literal_str_double())
    print(test_literal_bool_true())
