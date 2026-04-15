def test_var_assign():
    x = 10
    y = 20
    return x + y == 30

def test_aug_add():
    x = 5
    x += 5
    return x == 10

def test_reassign_same_type():
    s = "Hello"
    s = "World"
    return s == "World"

if __name__ == "__main__":
    print(test_var_assign())
    print(test_aug_add())
    print(test_reassign_same_type())
