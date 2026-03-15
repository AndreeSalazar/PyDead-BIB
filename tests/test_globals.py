counter = 0

def increment():
    global counter
    counter = counter + 1

def add_amount(n):
    global counter
    counter = counter + n

def get_counter():
    global counter
    return counter

increment()
increment()
increment()
add_amount(10)
result = get_counter()
print(f"counter = {result}")
print("globals ok")
