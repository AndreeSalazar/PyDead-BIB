try:
    x = 10
    print(x)
except:
    print(0)

try:
    raise ValueError("test error")
except ValueError:
    print("caught")

try:
    print(42)
finally:
    print(99)

print(1)
    