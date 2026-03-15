mem_count = 0
mem_max = 1000
mem_accesses = 0

def mem_add(importance):
    global mem_count
    global mem_accesses
    if mem_count < mem_max:
        mem_count = mem_count + 1
        mem_accesses = mem_accesses + 1
        return 1
    return 0

def mem_search(query_hash, count):
    found = query_hash % 5
    if found > count:
        found = count
    return found

def mem_hash(length, first):
    h = length * 31 + first
    h = h * 17 + 13
    return h % 65536

def mem_stats():
    print(f"memorias: {mem_count}/{mem_max}")
    print(f"accesos: {mem_accesses}")

mem_add(1)
mem_add(2)
mem_add(3)
mem_add(5)
mem_add(8)
h = mem_hash(5, 104)
r = mem_search(h, mem_count)
print(f"busqueda: {r} resultados")
mem_stats()
print("memory ok")
