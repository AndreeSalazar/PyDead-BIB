class Point:
    x: float
    y: float

    def __init__(self, x: float, y: float):
        self.x = x
        self.y = y

    def dist(self) -> float:
        return self.x * self.x + self.y * self.y

class Vector:
    dx: float
    dy: float

    def __init__(self, dx: float, dy: float):
        self.dx = dx
        self.dy = dy

    def magnitude(self) -> float:
        return self.dx * self.dx + self.dy * self.dy
