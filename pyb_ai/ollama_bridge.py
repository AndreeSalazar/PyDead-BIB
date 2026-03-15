oll_queries = 0
oll_tokens = 0
oll_connected = 0
oll_port = 11434

def oll_check():
    global oll_connected
    print("verificando conexion a Ollama...")
    print(f"  endpoint: http://localhost:{oll_port}/api/generate")
    oll_connected = 1
    print("  estado: listo (stub para ctypes.CDLL en produccion)")
    return 1

def oll_generate(prompt_len, max_tokens):
    global oll_queries
    global oll_tokens
    oll_queries = oll_queries + 1
    tokens = prompt_len * 2
    if tokens > max_tokens:
        tokens = max_tokens
    oll_tokens = oll_tokens + tokens
    print(f"  ollama query #{oll_queries}: {tokens} tokens")
    return tokens

def oll_chat(msg_len):
    global oll_queries
    global oll_tokens
    oll_queries = oll_queries + 1
    response = msg_len + 10
    oll_tokens = oll_tokens + response
    return response

def oll_embeddings(text_len):
    dim = 384
    print(f"  embeddings: {dim} dims para {text_len} chars")
    return dim

def oll_stats():
    print("--- Ollama Stats ---")
    print(f"  queries: {oll_queries}")
    print(f"  tokens: {oll_tokens}")
    print(f"  conectado: {oll_connected}")
    print("--------------------")

print("============================================================")
print("   Ollama Bridge para PyDead-BIB")
print("   LLM local — compilado nativo — sin CPython")
print("============================================================")
oll_check()
oll_generate(20, 100)
oll_generate(50, 200)
oll_generate(80, 150)
oll_chat(15)
oll_chat(30)
oll_embeddings(30)
oll_stats()
print("ollama_bridge ok")
