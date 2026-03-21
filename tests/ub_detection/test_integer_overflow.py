# PyDead-BIB UB Detection Tests — Integer Overflow
# PyDead-BIB debe detectar esto en COMPILE TIME

def test_large_power():
    """Large exponentiation — should warn about overflow"""
    base = 2
    exp = 100
    result = base ** exp  # UB: 2^100 overflows i64
    return result

def test_large_multiplication():
    """Large multiplication — should warn about overflow"""
    a = 9223372036854775807  # i64 max
    b = 2
    result = a * b  # UB: Overflow
    return result

def test_large_addition():
    """Large addition — should warn about overflow"""
    a = 9223372036854775807  # i64 max
    b = 1
    result = a + b  # UB: Overflow
    return result

def test_large_shift():
    """Large shift — should warn"""
    x = 1
    shift = 100
    result = x << shift  # UB: Shift by more than 63 bits
    return result

# These should NOT trigger UB detection (valid code)
def test_valid_power():
    """Valid exponentiation"""
    return 2 ** 10  # Expected: 1024

def test_valid_multiplication():
    """Valid multiplication"""
    return 1000 * 1000  # Expected: 1000000

def test_valid_shift():
    """Valid shift"""
    return 1 << 10  # Expected: 1024
