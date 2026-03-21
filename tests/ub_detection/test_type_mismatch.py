# PyDead-BIB UB Detection Tests — Type Mismatch
# PyDead-BIB debe detectar esto en COMPILE TIME

def test_str_plus_int():
    """String + int — should be caught at compile time"""
    s = "Hello"
    n = 42
    result = s + n  # UB: Cannot add str and int
    return result

def test_int_plus_str():
    """Int + string — should be caught at compile time"""
    n = 42
    s = "World"
    result = n + s  # UB: Cannot add int and str
    return result

def test_list_plus_int():
    """List + int — should be caught"""
    arr = [1, 2, 3]
    n = 4
    result = arr + n  # UB: Cannot add list and int
    return result

# These should NOT trigger UB detection (valid code)
def test_valid_str_concat():
    """Valid string concatenation"""
    s1 = "Hello"
    s2 = " World"
    return s1 + s2  # Expected: "Hello World"

def test_valid_int_add():
    """Valid integer addition"""
    a = 10
    b = 20
    return a + b  # Expected: 30

def test_valid_str_multiply():
    """Valid string multiplication"""
    s = "ab"
    n = 3
    return s * n  # Expected: "ababab"
