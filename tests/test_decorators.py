class Contador:
    def __init__(self):
        self.total = 0

    def incrementar(self):
        self.total = self.total + 1
        return self.total

c = Contador()
print(c.incrementar())
print(c.incrementar())
print(c.total)
