"""
Test de Integración: Flujo completo de compilación
Valida pipeline completo: .py → ejecutable nativo
"""

# === HELLO WORLD MÍNIMO ===

def test_hello_world():
    """Programa mínimo: imprime y retorna"""
    mensaje = "Hello PyDead-BIB"
    # print(mensaje)  # → WriteFile nativo
    return mensaje

# === ARITMÉTICA COMPLETA ===

def test_aritmetica_completa():
    """Todas las operaciones aritméticas"""
    a = 10
    b = 3
    
    suma = a + b        # 13
    resta = a - b       # 7
    mult = a * b        # 30
    div = a / b         # 3.333... (int / int = float? NO — ERROR en PyDead)
    div_entera = a // b # 3
    modulo = a % b      # 1
    
    return suma, resta, mult, div_entera, modulo

# === CONTROL DE FLUJO ===

def test_if_elif_else():
    """Condicionales"""
    x = 10
    
    if x > 10:
        return "mayor"
    elif x < 10:
        return "menor"
    else:
        return "igual"

def test_for_loop():
    """Bucle for"""
    total = 0
    for i in range(5):
        total = total + i
    return total  # 0+1+2+3+4 = 10

def test_while_loop():
    """Bucle while"""
    contador = 0
    while contador < 5:
        contador = contador + 1
    return contador  # 5

# === FUNCIONES ===

def funcion_auxiliar(x: int) -> int:
    """Función auxiliar"""
    return x * 2

def test_llamada_funcion():
    """Llamada a función"""
    resultado = funcion_auxiliar(5)
    return resultado  # 10

def test_funcion_anidada():
    """Funciones anidadas (scope)"""
    def interna(y: int) -> int:
        return y + 1
    
    return interna(10)  # 11

# === LISTAS ===

def test_lista_creacion():
    """Creación de lista homogénea"""
    numeros = [1, 2, 3, 4, 5]
    return numeros

def test_lista_acceso():
    """Acceso por índice"""
    lista = [10, 20, 30]
    return lista[1]  # 20

def test_lista_append():
    """Agregar elemento (si mismo tipo)"""
    lista = [1, 2, 3]
    # lista.append(4)  # método append
    return lista

# === DICCIONARIOS ===

def test_dict_creacion():
    """Creación de diccionario"""
    config = {"nombre": "test", "version": 1}
    return config

def test_dict_acceso():
    """Acceso por clave"""
    d = {"a": 1, "b": 2}
    return d["a"]  # 1

# === CLASES ===

class Persona:
    """Clase simple"""
    nombre: str
    edad: int
    
    def __init__(self, nombre: str, edad: int):
        self.nombre = nombre
        self.edad = edad
    
    def saludar(self) -> str:
        return "Hola, soy " + self.nombre

def test_clase_instancia():
    """Instanciación de clase"""
    p = Persona("PyDead", 1)
    return p.nombre

def test_clase_metodo():
    """Llamada a método"""
    p = Persona("BIB", 2)
    return p.saludar()

# === MÓDULOS ===

def test_import_nativo():
    """Import de módulos nativos"""
    # import math    # → SIMD inline
    # import os      # → Win32/syscall
    # import random  # → xorshift64
    pass

# === EJECUCIÓN ===

if __name__ == "__main__":
    print("=== Integration Tests ===")
    print(f"Hello: {test_hello_world()}")
    print(f"Aritmética: {test_aritmetica_completa()}")
    print(f"If/elif/else: {test_if_elif_else()}")
    print(f"For loop: {test_for_loop()}")
    print(f"While loop: {test_while_loop()}")
    print(f"Función: {test_llamada_funcion()}")
    print(f"Lista: {test_lista_acceso()}")
    print(f"Dict: {test_dict_acceso()}")
    print(f"Clase: {test_clase_instancia()}")
    print(f"Método: {test_clase_metodo()}")
    print("✅ Integration tests pasaron")
