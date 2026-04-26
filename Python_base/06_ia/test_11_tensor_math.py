# ============================================================
# PyDead-BIB Python_base — Nivel 11: Tensor Math (IA Basics)
# Tests que PyDead-BIB debe compilar → Runtime 2.0 tensor ops
# Usa solo escalares (sin listas) para compatibilidad con JIT actual
# ============================================================

def test_dot_product():
    """Producto punto — equivale a tensor_sum(tensor_mul(a, b))"""
    # dot([1,2,3,4], [5,6,7,8]) = 1*5 + 2*6 + 3*7 + 4*8 = 70
    result = 1.0 * 5.0 + 2.0 * 6.0 + 3.0 * 7.0 + 4.0 * 8.0
    return result == 70.0


def test_vector_add():
    """Suma de vectores escalares — equivale a tensor_add"""
    a0 = 1.0
    a1 = 2.0
    a2 = 3.0
    a3 = 4.0
    b0 = 8.0
    b1 = 7.0
    b2 = 6.0
    b3 = 5.0
    c0 = a0 + b0
    c1 = a1 + b1
    c2 = a2 + b2
    c3 = a3 + b3
    return c0 == 9.0 and c1 == 9.0 and c2 == 9.0 and c3 == 9.0


def test_vector_scale():
    """Escalar vector — equivale a tensor_scale(a, 3.0)"""
    a0 = 1.0
    a1 = 2.0
    a2 = 3.0
    a3 = 4.0
    a0 = a0 * 3.0
    a1 = a1 * 3.0
    a2 = a2 * 3.0
    a3 = a3 * 3.0
    return a0 == 3.0 and a3 == 12.0


def test_matmul_2x2():
    """Matmul manual 2x2 — equivale a tensor_matmul"""
    # A = [[1,2],[3,4]], B = [[5,6],[7,8]]
    a00 = 1.0
    a01 = 2.0
    a10 = 3.0
    a11 = 4.0
    b00 = 5.0
    b01 = 6.0
    b10 = 7.0
    b11 = 8.0
    c00 = a00 * b00 + a01 * b10
    c01 = a00 * b01 + a01 * b11
    c10 = a10 * b00 + a11 * b10
    c11 = a10 * b01 + a11 * b11
    return c00 == 19.0 and c01 == 22.0 and c10 == 43.0 and c11 == 50.0


def test_relu_manual():
    """ReLU manual — equivale a tensor_relu"""
    v0 = -2.0
    v1 = -1.0
    v2 = 0.0
    v3 = 1.0
    v4 = 2.0
    v5 = -0.5
    if v0 < 0.0:
        v0 = 0.0
    if v1 < 0.0:
        v1 = 0.0
    if v5 < 0.0:
        v5 = 0.0
    return v0 == 0.0 and v1 == 0.0 and v3 == 1.0 and v4 == 2.0 and v5 == 0.0


def test_mean():
    """Media aritmética — equivale a tensor_mean"""
    v0 = 2.0
    v1 = 4.0
    v2 = 6.0
    v3 = 8.0
    total = v0 + v1 + v2 + v3
    mean = total / 4.0
    return mean == 5.0


if __name__ == "__main__":
    print(test_dot_product())
    print(test_vector_add())
    print(test_vector_scale())
    print(test_matmul_2x2())
    print(test_relu_manual())
    print(test_mean())
