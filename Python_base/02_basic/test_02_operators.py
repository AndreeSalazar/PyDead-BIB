def test_op_add():
    return (10 + 20) == 30

def test_op_mul():
    return (5 * 5) == 25

def test_cmp_eq():
    return 100 == 100

def test_logic_and():
    return True and (10 > 5)

if __name__ == "__main__":
    print(test_op_add())
    print(test_op_mul())
    print(test_cmp_eq())
    print(test_logic_and())
