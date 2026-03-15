mem_count = 0
mem_max = 500

def mem_add(importance):
    global mem_count
    if mem_count < mem_max:
        mem_count = mem_count + 1
        return 1
    return 0

def mem_search(query_hash):
    found = query_hash % 5
    return found

def mem_stats():
    print(f"memorias: {mem_count}/{mem_max}")

mem_add(3)
mem_add(5)
mem_add(2)
mem_add(8)
mem_add(1)
mem_stats()
r = mem_search(42)
print(f"busqueda: {r} resultados")
print("memory ok")
