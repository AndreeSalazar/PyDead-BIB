# PyDead-BIB UB Detection Tests — None Dereference
# PyDead-BIB debe detectar esto en COMPILE TIME

def test_none_method_call():
    """Calling method on None — should be caught at compile time"""
    x = None
    result = x.upper()  # UB: None has no method 'upper'
    return result

def test_none_attribute():
    """Accessing attribute on None — should be caught"""
    x = None
    result = x.value  # UB: None has no attribute 'value'
    return result

def test_none_in_function():
    """Passing None to function expecting value"""
    def process(obj):
        return obj.data  # UB if obj is None
    
    return process(None)

def test_none_subscript():
    """Subscripting None — should be caught"""
    x = None
    result = x[0]  # UB: None is not subscriptable
    return result

# These should NOT trigger UB detection (valid code)
def test_valid_none_check():
    """Valid None check"""
    x = None
    if x is not None:
        return x.upper()
    return "default"

def test_valid_optional():
    """Valid optional handling"""
    def get_value(x):
        if x is None:
            return 0
        return x
    return get_value(None)  # Expected: 0
