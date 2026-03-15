try:
    x = 10
    print(x)
except:
    print(0)

try:
    raise ValueError("bad")
except ValueError:
    print("caught")

try:
    y = 5
    print(y)
finally:
    print(99)

print(1)
