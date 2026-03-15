def ws_hash(length):
    h = length * 31 + 7
    h = h * 17 + 13
    h = h % 65536
    return h

def ws_search(query_len):
    h = ws_hash(query_len)
    results = h % 10 + 1
    print(f"busqueda: {results} resultados")
    return results

def ws_extract(url_len):
    h = ws_hash(url_len)
    words = h % 500 + 100
    print(f"extraccion: {words} palabras")
    return words

def ws_summarize(text_len):
    summary_len = text_len // 4
    if summary_len < 10:
        summary_len = 10
    print(f"resumen: {summary_len} palabras")
    return summary_len

def ws_rank_results(num_results, query_len):
    h = ws_hash(query_len)
    top_score = 70 + (h % 30)
    print(f"ranking: top score {top_score}% de {num_results} resultados")
    return top_score

print("============================================================")
print("   Web Search para PyDead-BIB v3.0")
print("   Busqueda inteligente — Compilado NATIVO")
print("============================================================")
r1 = ws_search(10)
r2 = ws_search(25)
r3 = ws_search(42)
ws_extract(50)
ws_extract(100)
ws_summarize(500)
ws_summarize(1000)
ws_rank_results(r1, 10)
ws_rank_results(r2, 25)
print("")
print("web_search ok")
