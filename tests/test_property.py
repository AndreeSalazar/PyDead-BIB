class Rect:
    def __init__(self, w, h):
        self.w = w
        self.h = h

    def area(self):
        return self.w * self.h

r = Rect(4, 5)
print(r.area())
print(r.w)
print(r.h)
