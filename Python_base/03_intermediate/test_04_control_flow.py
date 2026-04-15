def test_if_simple():
    if 10 > 5:
        return True
    return False

def test_for_range():
    total = 0
    for i in range(5):
        total += i
    return total == 10

def test_while_simple():
    count = 0
    while count < 3:
        count += 1
    return count == 3

def test_break():
    val = 0
    for i in range(10):
        if i == 5:
            break
        val = i
    return val == 4

if __name__ == "__main__":
    print(test_if_simple())
    print(test_for_range())
    print(test_while_simple())
    print(test_break())
