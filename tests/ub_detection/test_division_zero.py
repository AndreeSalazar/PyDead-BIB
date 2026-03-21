# PyDead-BIB UB Detection Tests — Division by Zero
# PyDead-BIB debe detectar esto en COMPILE TIME

def test_obvious_division_zero():
    """Obvious division by zero — should be caught at compile time"""
    x = 10
    y = 0
    result = x / y  # UB: Division by zero
    return result

def test_floor_division_zero():
    """Floor division by zero"""
    x = 10
    y = 0
    result = x // y  # UB: Floor division by zero
    return result

def test_modulo_zero():
    """Modulo by zero"""
    x = 10
    y = 0
    result = x % y  # UB: Modulo by zero
    return result

def test_variable_division_zero():
    """Division by variable that is zero"""
    divisor = 0
    result = 100 / divisor  # UB: Should detect divisor is 0
    return result

# These should NOT trigger UB detection (valid code)
def test_valid_division():
    """Valid division — should compile fine"""
    x = 10
    y = 2
    return x / y  # Expected: 5.0

def test_valid_modulo():
    """Valid modulo — should compile fine"""
    x = 10
    y = 3
    return x % y  # Expected: 1
