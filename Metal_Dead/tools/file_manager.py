def fm_hash(length):
    h = length * 31 + 7
    h = h * 17 + 13
    h = h % 65536
    return h

def fm_list_files(dir_hash):
    count = dir_hash % 20 + 1
    print(f"archivos listados: {count}")
    return count

def fm_create_dir(name_len):
    h = fm_hash(name_len)
    print(f"directorio creado (hash={h})")
    return 1

def fm_file_size(path_len):
    h = fm_hash(path_len)
    size_kb = h % 1000 + 1
    print(f"tamano: {size_kb} KB")
    return size_kb

def fm_copy_file(src_len, dst_len):
    h1 = fm_hash(src_len)
    h2 = fm_hash(dst_len)
    print(f"archivo copiado (src={h1} dst={h2})")
    return 1

def fm_analyze_dir(dir_len):
    h = fm_hash(dir_len)
    files = h % 50 + 5
    dirs = h % 10 + 1
    total_kb = files * 25
    print(f"analisis: {files} archivos, {dirs} dirs, {total_kb} KB")
    return files

def fm_search_files(pattern_len, dir_len):
    h1 = fm_hash(pattern_len)
    h2 = fm_hash(dir_len)
    matches = (h1 + h2) % 15 + 1
    print(f"busqueda: {matches} coincidencias")
    return matches

def fm_create_project(name_len):
    h = fm_hash(name_len)
    files_created = h % 8 + 3
    print(f"proyecto creado: {files_created} archivos")
    return files_created

print("============================================================")
print("   File Manager para PyDead-BIB v3.0")
print("   Gestion de archivos — Compilado NATIVO")
print("============================================================")
fm_list_files(42)
fm_create_dir(10)
fm_file_size(25)
fm_copy_file(15, 20)
fm_analyze_dir(30)
fm_search_files(5, 30)
fm_create_project(12)
print("")
print("file_manager ok")
