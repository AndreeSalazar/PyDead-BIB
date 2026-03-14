def test_builtins() -> int:
    print("testing builtins")
    x: int = abs(-5)
    y: int = min(3, 7)
    z: int = max(3, 7)
    total: int = x + y + z
    return total

result: int = test_builtins()
