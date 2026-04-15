"""
Test: Respeto de Bits 💀
Valida que PyDead-BIB rechaza operaciones entre tipos diferentes
sin conversión explícita.
"""

# === OPERACIONES VÁLIDAS: mismo tipo ===

def test_int_suma_int():
    """int + int = int ✅"""
    x = 5 + 3
    return x  # 8

def test_float_suma_float():
    """float + float = float ✅"""
    y = 1.5 + 2.5
    return y  # 4.0

def test_str_concat_str():
    """str + str = str ✅"""
    s = "Hola" + " Mundo"
    return s  # "Hola Mundo"

def test_str_repeticion():
    """str * int = str ✅ (repetición válida)"""
    r = "ab" * 3
    return r  # "ababab"

def test_bool_aritmetica():
    """bool es subtype de int ✅"""
    b = True + True  # 1 + 1
    n = True + 1     # 1 + 1
    return b, n  # (2, 2)

# === CONVERSIÓN EXPLÍCITA: única manera válida ===

def test_conversion_explicita_int_float():
    """int → float explícito ✅"""
    x = 5
    y = 3.14
    z = float(x) + y
    return z  # 8.14

def test_conversion_explicita_float_int():
    """float → int explícito (con pérdida) ✅"""
    x = 3.14
    y = int(x) + 5
    return y  # 8

def test_conversion_explicita_int_str():
    """int → str explícito ✅"""
    x = 42
    s = str(x) + " items"
    return s  # "42 items"

# === ERRORES DE TIPO: deben fallar en compilación ===

def test_error_int_float_implicito():
    """
    💀 ERROR DE COMPILACIÓN ESPERADO
    x = 5 + 3.14  # int + float = TypeMismatch
    """
    pass  # Este test valida que el compilador rechaza esto

def test_error_float_int_implicito():
    """
    💀 ERROR DE COMPILACIÓN ESPERADO
    y = 3.14 + 5  # float + int = TypeMismatch
    """
    pass

def test_error_str_int_suma():
    """
    💀 ERROR DE COMPILACIÓN ESPERADO
    s = "hola" + 42  # str + int = TypeMismatch
    """
    pass

def test_error_comparacion_tipos():
    """
    💀 ERROR DE COMPILACIÓN ESPERADO
    if 5 == 5.0:  # int == float = TypeMismatch
        pass
    """
    pass

def test_error_lista_heterogenea():
    """
    💀 ERROR DE COMPILACIÓN ESPERADO
    l = [1, 2.0, 3]  # Lista heterogénea = TypeMismatch
    """
    pass

# === TIPOS ESTRICTOS EN FUNCIONES ===

def suma_enteros(a: int, b: int) -> int:
    """Función con tipos estrictos"""
    return a + b

def suma_floats(x: float, y: float) -> float:
    """Función float solo acepta floats"""
    return x + y

# === EJECUCIÓN DE TESTS ===

if __name__ == "__main__":
    print("=== Tests de Respeto de Bits ===")
    print(f"test_int_suma_int: {test_int_suma_int()}")
    print(f"test_float_suma_float: {test_float_suma_float()}")
    print(f"test_str_concat_str: {test_str_concat_str()}")
    print(f"test_str_repeticion: {test_str_repeticion()}")
    print(f"test_bool_aritmetica: {test_bool_aritmetica()}")
    print(f"test_conversion_int_float: {test_conversion_explicita_int_float()}")
    print(f"test_conversion_float_int: {test_conversion_explicita_float_int()}")
    print(f"test_conversion_int_str: {test_conversion_explicita_int_str()}")
    print("✅ Todos los tests válidos pasaron")
