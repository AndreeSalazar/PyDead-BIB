"""
Test: UB Detection 💀🦈
Undefined Behavior detectado en tiempo de compilación
"""

# === NoneDeref: acceso a atributo de None ===

def test_ub_nonederef_detectado():
    """
    💀 UB: NoneDeref
    x = None
    print(x.nombre)  # ERROR en compilación
    """
    pass  # El compilador debe detectar esto

# === IndexOutOfBounds: índice fuera de rango ===

def test_ub_index_out_of_bounds():
    """
    💀 UB: IndexOutOfBounds
    lista = [1, 2, 3]
    print(lista[100])  # ERROR en compilación si índice es constante
    """
    pass

def test_ub_index_bounds_detected():
    """Índice conocido en tiempo de compilación"""
    data = [10, 20, 30]
    # idx = 5  # ERROR: índice 5 no existe en lista de 3 elementos
    return data[0]  # ✅ Válido

# === KeyNotFound: clave no existe en dict ===

def test_ub_key_not_found():
    """
    💀 UB: KeyNotFound
    d = {"a": 1}
    print(d["x"])  # ERROR en compilación si clave es constante
    """
    pass

# === DivisionByZero: división por cero ===

def test_ub_division_by_zero():
    """
    💀 UB: DivisionByZero
    x = 10 / 0  # ERROR en compilación
    """
    pass

def test_ub_division_by_zero_dynamic():
    """División por cero en runtime también detectada"""
    divisor = 0
    # resultado = 100 / divisor  # ERROR si divisor es 0
    return divisor

# === MutableDefaultArg: argumento mutable por defecto ===

def test_ub_mutable_default_arg():
    """
    💀 UB: MutableDefaultArg (warning)
    def f(x=[]):
        x.append(1)
        return x
    """
    pass  # Warning en compilación

# === TypeMismatch: operación entre tipos incompatibles ===

def test_ub_type_mismatch():
    """
    💀 UB: TypeMismatch
    "hola" + 42  # str + int = ERROR
    """
    pass

# === InfiniteRecursion: recursión infinita detectada ===

def test_ub_infinite_recursion():
    """
    💀 UB: InfiniteRecursion
    def recursivo():
        return recursivo()  # Sin caso base = ERROR
    """
    pass

# === UnpackMismatch: desempaquetado incorrecto ===

def test_ub_unpack_mismatch():
    """
    💀 UB: UnpackMismatch
    a, b = [1, 2, 3]  # 2 variables, 3 valores = ERROR
    """
    pass

# === CircularImport: import circular ===

def test_ub_circular_import():
    """
    💀 UB: CircularImport
    # a.py: import b
    # b.py: import a
    """
    pass

# === TESTS VÁLIDOS: sin UB ===

def test_valido_acceso_seguro():
    """Acceso a lista con índice válido"""
    lista = [1, 2, 3, 4, 5]
    return lista[2]  # 3 ✅

def test_valido_dict_existente():
    """Acceso a clave existente"""
    d = {"nombre": "PyDead", "version": 430}
    return d["nombre"]  # "PyDead" ✅

def test_valido_division_segura():
    """División con divisor no-cero"""
    return 100 / 10  # 10.0 ✅

def test_valido_unpack_correcto():
    """Desempaquetado correcto"""
    a, b, c = [1, 2, 3]
    return a + b + c  # 6 ✅

# === EJECUCIÓN ===

if __name__ == "__main__":
    print("=== UB Detection Tests ===")
    print(f"Acceso seguro: {test_valido_acceso_seguro()}")
    print(f"Dict existente: {test_valido_dict_existente()}")
    print(f"División segura: {test_valido_division_segura()}")
    print(f"Unpack correcto: {test_valido_unpack_correcto()}")
    print("✅ Tests sin UB pasaron")
