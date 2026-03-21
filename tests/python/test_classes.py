# PyDead-BIB Python Class Tests
# Tests de clases y OOP

class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    
    def distance(self):
        return (self.x ** 2 + self.y ** 2) ** 0.5

class Rectangle:
    def __init__(self, width, height):
        self.width = width
        self.height = height
    
    def area(self):
        return self.width * self.height
    
    def perimeter(self):
        return 2 * (self.width + self.height)

class Animal:
    def __init__(self, name):
        self.name = name
    
    def speak(self):
        return "..."

class Dog(Animal):
    def speak(self):
        return "Woof!"

class Cat(Animal):
    def speak(self):
        return "Meow!"

def test_point():
    p = Point(3, 4)
    return p.distance()  # Expected: 5.0

def test_rectangle():
    r = Rectangle(5, 3)
    return r.area()  # Expected: 15

def test_inheritance():
    dog = Dog("Rex")
    cat = Cat("Whiskers")
    return dog.speak() + " " + cat.speak()  # Expected: "Woof! Meow!"

if __name__ == "__main__":
    print("test_point:", test_point())
    print("test_rectangle:", test_rectangle())
    print("test_inheritance:", test_inheritance())
