class Animal:
    def __init__(self, name, age):
        self.name = name
        self.age = age

    def speak(self):
        return self.age

class Dog(Animal):
    def __init__(self, name, breed):
        self.name = name
        self.breed = breed

    def bark(self):
        return self.breed

a = Animal(10, 5)
d = Dog(20, 99)

print(a.speak())
print(d.bark())
print("inheritance ok")
