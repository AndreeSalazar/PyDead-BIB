class Animal:
    def __init__(self, edad):
        self.edad = edad

class Perro(Animal):
    def __init__(self, edad, nombre):
        self.edad = edad
        self.nombre = nombre

p = Perro(3, 42)
print(p.edad)
print(p.nombre)
