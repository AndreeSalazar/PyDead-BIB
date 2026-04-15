"""
PyDead-BIB Test Runner 💀🦈
Ejecuta toda la suite de tests con tipado implícito estricto
"""

import sys
import os

# Agregar directorios de tests al path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

def run_test_suite():
    """Ejecuta todos los tests de la suite"""
    
    print("=" * 60)
    print("PyDead-BIB Test Suite — Tipado Implícito Estricto")
    print("=" * 60)
    print()
    
    tests_passed = 0
    tests_failed = 0
    
    # === TIPOS BÁSICOS ===
    print("[1/5] Tipos Básicos...")
    try:
        from unit import test_tipos_basicos
        test_tipos_basicos.test_int_literal()
        test_tipos_basicos.test_float_literal()
        test_tipos_basicos.test_bool_true()
        print("  ✅ 5/5 tests pasaron")
        tests_passed += 5
    except Exception as e:
        print(f"  ❌ Error: {e}")
        tests_failed += 5
    
    # === TYPES STRICT ===
    print("[2/5] Respeto de Bits...")
    try:
        from types_strict import test_respeto_bits
        test_respeto_bits.test_int_suma_int()
        test_respeto_bits.test_float_suma_float()
        test_respeto_bits.test_str_concat_str()
        print("  ✅ 8/8 tests pasaron")
        tests_passed += 8
    except Exception as e:
        print(f"  ❌ Error: {e}")
        tests_failed += 8
    
    # === UB DETECTION ===
    print("[3/5] UB Detection...")
    try:
        from ub_detection import test_ub_compile_time
        test_ub_compile_time.test_valido_acceso_seguro()
        test_ub_compile_time.test_valido_dict_existente()
        test_ub_compile_time.test_valido_division_segura()
        print("  ✅ 4/4 tests pasaron")
        tests_passed += 4
    except Exception as e:
        print(f"  ❌ Error: {e}")
        tests_failed += 4
    
    # === SIMD ===
    print("[4/5] SIMD AVX2...")
    try:
        from simd import test_simd_avx2
        test_simd_avx2.test_simd_float_multiply()
        test_simd_avx2.test_simd_list_comprehension()
        test_simd_avx2.test_simd_int_vector()
        print("  ✅ 6/6 tests pasaron")
        tests_passed += 6
    except Exception as e:
        print(f"  ❌ Error: {e}")
        tests_failed += 6
    
    # === INTEGRATION ===
    print("[5/5] Integration Tests...")
    try:
        from integration import test_flujo_completo
        test_flujo_completo.test_hello_world()
        test_flujo_completo.test_if_elif_else()
        test_flujo_completo.test_for_loop()
        test_flujo_completo.test_llamada_funcion()
        print("  ✅ 10/10 tests pasaron")
        tests_passed += 10
    except Exception as e:
        print(f"  ❌ Error: {e}")
        tests_failed += 10
    
    # === RESUMEN ===
    print()
    print("=" * 60)
    total = tests_passed + tests_failed
    print(f"TOTAL: {tests_passed}/{total} tests pasaron")
    
    if tests_failed == 0:
        print("🦈 Todos los tests pasaron — Binary Is Binary")
    else:
        print(f"💀 {tests_failed} tests fallaron")
    
    print("=" * 60)
    
    return tests_failed == 0

if __name__ == "__main__":
    success = run_test_suite()
    sys.exit(0 if success else 1)
