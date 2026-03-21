# PyDead-BIB C ABI Tests — ctypes.Structure
# Tests de estructuras C

import ctypes

class Point(ctypes.Structure):
    _fields_ = [
        ("x", ctypes.c_int),
        ("y", ctypes.c_int)
    ]

class Rectangle(ctypes.Structure):
    _fields_ = [
        ("width", ctypes.c_int),
        ("height", ctypes.c_int),
        ("area", ctypes.c_int)
    ]

class Person(ctypes.Structure):
    _fields_ = [
        ("age", ctypes.c_int),
        ("height", ctypes.c_float),
        ("id", ctypes.c_longlong)
    ]

def test_point_structure():
    p = Point()
    p.x = 10
    p.y = 20
    return p.x + p.y  # Expected: 30

def test_rectangle_structure():
    r = Rectangle()
    r.width = 5
    r.height = 3
    r.area = r.width * r.height
    return r.area  # Expected: 15

def test_person_structure():
    person = Person()
    person.age = 25
    person.height = 1.75
    person.id = 123456789
    return person.age  # Expected: 25

def test_structure_pointer():
    p = Point()
    p.x = 100
    p.y = 200
    ptr = ctypes.pointer(p)
    return ptr.contents.x  # Expected: 100

def test_structure_array():
    PointArray = Point * 3
    arr = PointArray()
    arr[0].x = 1
    arr[0].y = 2
    arr[1].x = 3
    arr[1].y = 4
    arr[2].x = 5
    arr[2].y = 6
    total = arr[0].x + arr[1].x + arr[2].x
    return total  # Expected: 9

if __name__ == "__main__":
    print("test_point_structure:", test_point_structure())
    print("test_rectangle_structure:", test_rectangle_structure())
    print("test_person_structure:", test_person_structure())
    print("test_structure_pointer:", test_structure_pointer())
    print("test_structure_array:", test_structure_array())
