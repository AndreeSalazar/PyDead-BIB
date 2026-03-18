def main() -> int:
    d = {"nombre": "Andre", "ciudad": "Lima"}
    print(d["nombre"])
    print(d["ciudad"])
    
    # Test setting a new key
    d["lenguaje"] = "PyDead-BIB"
    print(d["lenguaje"])
    
    # Test overwriting existing key
    d["ciudad"] = "Arequipa"
    print(d["ciudad"])
    
    # Test get method
    print(d.get("nombre"))
    
    return 0

main()
