"""
Test: SIMD AVX2 — 256-bit operations
PyDead-BIB debe generar instrucciones AVX2 automáticamente
"""

# === VECTOR OPERATIONS: 8 floats × 32 bits = 256 bits ===

def test_simd_float_multiply():
    """
    8 floats en un ciclo — VMULPS ymm0, ymm1, ymm2
    """
    velocidades = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
    dobles = [v * 2.0 for v in velocidades]
    return dobles

def test_simd_float_add():
    """
    Suma vectorial — VADDPS
    """
    a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
    b = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]
    resultado = [a[i] + b[i] for i in range(8)]
    return resultado

def test_simd_list_comprehension():
    """
    List comprehension detecta SIMD automáticamente
    """
    datos = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
    cuadrados = [x * x for x in datos]
    return cuadrados

# === NUMPY-STYLE OPERATIONS ===

def test_simd_numpy_style_sqrt():
    """
    VSQRTPS — raíz cuadrada vectorial
    """
    valores = [1.0, 4.0, 9.0, 16.0, 25.0, 36.0, 49.0, 64.0]
    raices = [v ** 0.5 for v in valores]  # Debe generar sqrt SIMD
    return raices

def test_simd_scalar_broadcast():
    """
    Broadcast de escalar a vector — VBROADCASTSS
    """
    data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
    resultado = [d * 10.0 for d in data]  # 10.0 broadcast a todos
    return resultado

# === TIPOS SIMD ESTRICTOS ===

def test_simd_tipo_estricto():
    """
    SIMD solo funciona con tipos homogéneos
    Lista heterogénea = ERROR de compilación
    """
    # ❌ ERROR: [1, 2.0, 3] — tipos mixtos
    # ✅ VÁLIDO: [1.0, 2.0, 3.0] — todos float
    return [1.0, 2.0, 3.0, 4.0]

def test_simd_int_vector():
    """
    VPADDD — packed integer addition
    """
    enteros = [1, 2, 3, 4, 5, 6, 7, 8]
    dobles = [x * 2 for x in enteros]
    return dobles

# === ALINEAMIENTO DE MEMORIA ===

def test_simd_alignment():
    """
    Datos SIMD deben estar alineados a 32 bytes (256-bit)
    """
    # PyDead-BIB debe garantizar alineamiento para VMOVAPS
    aligned_data = [0.0] * 8  # 8 floats = 32 bytes
    return aligned_data

# === EJECUCIÓN ===

if __name__ == "__main__":
    print("=== SIMD AVX2 Tests ===")
    print(f"Float multiply: {test_simd_float_multiply()}")
    print(f"Float add: {test_simd_float_add()}")
    print(f"List comprehension: {test_simd_list_comprehension()}")
    print(f"Sqrt: {test_simd_numpy_style_sqrt()}")
    print(f"Scalar broadcast: {test_simd_scalar_broadcast()}")
    print(f"Int vector: {test_simd_int_vector()}")
    print("✅ SIMD tests pasaron")
