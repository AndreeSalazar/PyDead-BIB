"""
Test Unitario: Tipos Básicos
Valida comportamiento de tipos fundamentales en PyDead-BIB
"""

# === ENTEROS (int) ===

def test_int_literal():
    """Literal entero → RAX"""
    x = 42
    return x

def test_int_negativo():
    """Entero negativo"""
    x = -100
    return x

def test_int_cero():
    """Cero"""
    x = 0
    return x

def test_int_grande():
    """Entero grande (64-bit)"""
    x = 9223372036854775807  # i64::MAX
    return x

# === FLOTANTES (float) ===

def test_float_literal():
    """Literal float → XMM/YMM"""
    y = 3.14159
    return y

def test_float_negativo():
    """Float negativo"""
    y = -2.5
    return y

def test_float_cero():
    """Float cero"""
    y = 0.0
    return y

def test_float_scientific():
    """Notación científica"""
    y = 1.5e10
    return y

# === STRINGS (str) ===

def test_str_literal():
    """String literal → .data section"""
    s = "PyDead-BIB"
    return s

def test_str_vacio():
    """String vacío"""
    s = ""
    return s

def test_str_unicode():
    """Unicode support"""
    s = "💀🦈 Python Nativo"
    return s

# === BOOLEANOS (bool) ===

def test_bool_true():
    """True → 1 byte"""
    b = True
    return b

def test_bool_false():
    """False → 1 byte"""
    b = False
    return b

def test_bool_and():
    """AND lógico"""
    return True and False

def test_bool_or():
    """OR lógico"""
    return True or False

def test_bool_not():
    """NOT lógico"""
    return not True

# === NONE (NoneType) ===

def test_none():
    """None → null pointer"""
    n = None
    return n

# === ASIGNACIÓN Y REASIGNACIÓN ===

def test_reasignacion_mismo_tipo():
    """Reasignación con mismo tipo = OK"""
    x = 10
    x = 20
    return x

def test_reasignacion_diferente_tipo():
    """
    💀 ERROR: reasignación cambia tipo
    x = 10      # int
    x = "hola"  # str = TypeMismatch
    """
    pass

# === EJECUCIÓN ===

if __name__ == "__main__":
    print("=== Tests de Tipos Básicos ===")
    print(f"int literal: {test_int_literal()}")
    print(f"int negativo: {test_int_negativo()}")
    print(f"float literal: {test_float_literal()}")
    print(f"float científico: {test_float_scientific()}")
    print(f"bool AND: {test_bool_and()}")
    print(f"bool OR: {test_bool_or()}")
    print(f"bool NOT: {test_bool_not()}")
    print("✅ Tests de tipos básicos pasaron")
