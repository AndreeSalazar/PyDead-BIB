"""
Python_base Test Runner 💀🦈
Ejecuta toda la suite de construcción paso a paso
"""

import sys
import os
import importlib
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# Definición de niveles y tests
LEVELS = [
    {
        "name": "01_hello",
        "title": "HELLO — Nivel 0: print() mínimo",
        "modules": [
            ("01_hello.test_00_print", [
                "test_print_string",
                "test_print_empty",
                "test_hello_compilable",
            ]),
        ],
    },
    {
        "name": "02_basic",
        "title": "BASIC — Nivel 1-3: Fundamentos",
        "modules": [
            ("02_basic.test_01_literals", [
                "test_literal_int_positive",
                "test_literal_float_simple",
                "test_literal_str_double",
                "test_literal_bool_true",
            ]),
            ("02_basic.test_02_operators", [
                "test_op_add",
                "test_op_mul",
                "test_cmp_eq",
                "test_logic_and",
            ]),
            ("02_basic.test_03_variables", [
                "test_var_assign",
                "test_aug_add",
                "test_reassign_same_type",
            ]),
        ],
    },
    {
        "name": "03_intermediate",
        "title": "INTERMEDIATE — Nivel 4-5: Control Flow & Functions",
        "modules": [
            ("03_intermediate.test_04_control_flow", [
                "test_if_simple",
                "test_for_range",
                "test_while_simple",
                "test_break",
            ]),
            ("03_intermediate.test_05_functions", [
                "test_def_simple",
                "test_def_params_default",
                "test_recursion_factorial",
                "test_closure_simple",
            ]),
        ],
    },
    {
        "name": "04_advanced",
        "title": "ADVANCED — Nivel 6-8: OOP, Exceptions, Modules",
        "modules": [
            ("04_advanced.test_06_classes", [
                "test_class_simple",
                "test_inheritance_simple",
                "test_staticmethod",
            ]),
            ("04_advanced.test_07_exceptions", [
                "test_try_except",
                "test_try_finally",
                "test_raise_simple",
            ]),
            ("04_advanced.test_08_modules", [
                "test_import_module",
                "test_main_guard",
            ]),
        ],
    },
    {
        "name": "05_complete",
        "title": "COMPLETE — Nivel 9-10: End-to-End & SIMD",
        "modules": [
            ("05_complete.test_09_end_to_end", [
                "test_e2e_hello_runs",
                "test_e2e_factorial",
                "test_e2e_classes",
            ]),
            ("05_complete.test_10_simd_native", [
                "test_simd_float_vector",
                "test_opt_const_folding",
            ]),
        ],
    },
]


def run_test(module_name, test_name):
    """Ejecuta un test individual"""
    try:
        module = importlib.import_module(module_name)
        test_func = getattr(module, test_name)
        result = test_func()
        return True, result, None
    except Exception as e:
        return False, None, str(e)


def run_level(level):
    """Ejecuta todos los tests de un nivel"""
    print(f"\n{'='*60}")
    print(f"{level['title']}")
    print(f"{'='*60}")
    
    passed = 0
    failed = 0
    
    for module_name, test_names in level["modules"]:
        for test_name in test_names:
            success, result, error = run_test(module_name, test_name)
            
            if success:
                passed += 1
                status = "✅"
            else:
                failed += 1
                status = "❌"
            
            full_name = f"{module_name}.{test_name}"
            print(f"  {status} {full_name:50}", end="")
            
            if success:
                # Truncate result if too long
                result_str = str(result)
                if len(result_str) > 20:
                    result_str = result_str[:17] + "..."
                print(f" → {result_str}")
            else:
                print(f"\n     ERROR: {error[:60]}")
    
    return passed, failed


def main():
    """Ejecuta toda la suite"""
    print("="*60)
    print("Python_base Test Suite 💀🦈")
    print("Construyendo PyDead-BIB: Python → Machine Code")
    print("="*60)
    
    total_passed = 0
    total_failed = 0
    
    for level in LEVELS:
        p, f = run_level(level)
        total_passed += p
        total_failed += f
    
    # Resumen final
    print("\n" + "="*60)
    print("RESUMEN FINAL")
    print("="*60)
    
    for level in LEVELS:
        # Re-run to count (inefficient but simple)
        p, f = run_level(level)
        print(f"  {level['name']:15} {p:3} passed, {f:3} failed")
    
    print("-"*60)
    total = total_passed + total_failed
    print(f"  TOTAL: {total_passed}/{total} tests")
    
    if total_failed == 0:
        print("\n  🦈 TODOS LOS NIVELES PASARON")
        print("  PyDead-BIB está listo para producción")
    else:
        print(f"\n  💀 {total_failed} tests fallaron")
        print("  Revisar implementación en src/rust/")
    
    print("="*60)
    
    return total_failed == 0


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
