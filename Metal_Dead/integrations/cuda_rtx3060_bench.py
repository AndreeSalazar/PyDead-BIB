"""
Real CUDA RTX 3060 12GB Benchmark — runs with CPython + PyTorch
Tests actual GPU performance on your hardware
Then Metal_Dead uses PyDead-BIB compiled code for native inference
"""
import torch
import time

print("=" * 64)
print("   Metal-Dead CUDA RTX 3060 12GB — Real GPU Benchmark")
print("   PyDead-BIB v4.0 + PyTorch CUDA + JIT KILLER v2.0")
print("=" * 64)

# GPU detection
if torch.cuda.is_available():
    gpu_name = torch.cuda.get_device_name(0)
    props = torch.cuda.get_device_properties(0)
    vram_gb = props.total_memory / (1024**3)
    sm_count = props.multi_processor_count
    print(f"\n   GPU:  {gpu_name}")
    print(f"   VRAM: {vram_gb:.1f} GB")
    print(f"   SMs:  {sm_count}")
    print(f"   CUDA: {torch.version.cuda}")
    device = torch.device("cuda")
else:
    print("\n   GPU: No CUDA disponible, usando CPU")
    device = torch.device("cpu")

print(f"   PyTorch: {torch.__version__}")
print(f"   Device:  {device}")

# 1. Matrix Multiplication Benchmark
print("\n--- 1. MATMUL Benchmark ---")
sizes = [256, 512, 1024, 2048, 4096]
for size in sizes:
    a = torch.randn(size, size, device=device, dtype=torch.float32)
    b = torch.randn(size, size, device=device, dtype=torch.float32)
    
    # warmup
    for _ in range(3):
        c = torch.matmul(a, b)
    if device.type == "cuda":
        torch.cuda.synchronize()
    
    start = time.perf_counter()
    iters = 20 if size <= 2048 else 5
    for _ in range(iters):
        c = torch.matmul(a, b)
    if device.type == "cuda":
        torch.cuda.synchronize()
    
    elapsed = (time.perf_counter() - start) * 1000 / iters
    gflops = (2 * size**3) / (elapsed / 1000) / 1e9
    print(f"  matmul {size}x{size}: {elapsed:.3f}ms  ({gflops:.0f} GFLOPS)")

# 2. Attention Benchmark
print("\n--- 2. ATTENTION Benchmark ---")
configs = [
    (1, 128, 256, 8, "small"),
    (1, 256, 512, 8, "medium"),
    (1, 512, 768, 12, "large"),
    (4, 256, 512, 8, "batch4"),
]
for batch, seq, embed, heads, label in configs:
    head_dim = embed // heads
    q = torch.randn(batch, heads, seq, head_dim, device=device)
    k = torch.randn(batch, heads, seq, head_dim, device=device)
    v = torch.randn(batch, heads, seq, head_dim, device=device)
    
    # warmup
    for _ in range(3):
        scores = torch.matmul(q, k.transpose(-2, -1)) / (head_dim ** 0.5)
        weights = torch.softmax(scores, dim=-1)
        attn = torch.matmul(weights, v)
    if device.type == "cuda":
        torch.cuda.synchronize()
    
    start = time.perf_counter()
    for _ in range(20):
        scores = torch.matmul(q, k.transpose(-2, -1)) / (head_dim ** 0.5)
        weights = torch.softmax(scores, dim=-1)
        attn = torch.matmul(weights, v)
    if device.type == "cuda":
        torch.cuda.synchronize()
    
    elapsed = (time.perf_counter() - start) * 1000 / 20
    print(f"  attention [{label}] b={batch} seq={seq} e={embed} h={heads}: {elapsed:.3f}ms")

# 3. Softmax + GELU
print("\n--- 3. ACTIVATIONS Benchmark ---")
x = torch.randn(4096, 4096, device=device)
for _ in range(5):
    _ = torch.softmax(x, dim=-1)
if device.type == "cuda":
    torch.cuda.synchronize()

start = time.perf_counter()
for _ in range(50):
    y = torch.softmax(x, dim=-1)
if device.type == "cuda":
    torch.cuda.synchronize()
elapsed = (time.perf_counter() - start) * 1000 / 50
print(f"  softmax 4096x4096: {elapsed:.3f}ms")

start = time.perf_counter()
for _ in range(50):
    y = torch.nn.functional.gelu(x)
if device.type == "cuda":
    torch.cuda.synchronize()
elapsed = (time.perf_counter() - start) * 1000 / 50
print(f"  GELU 4096x4096: {elapsed:.3f}ms")

# 4. FP16 / BF16 Performance
print("\n--- 4. MIXED PRECISION ---")
if device.type == "cuda":
    size = 2048
    a16 = torch.randn(size, size, device=device, dtype=torch.float16)
    b16 = torch.randn(size, size, device=device, dtype=torch.float16)
    for _ in range(5):
        c16 = torch.matmul(a16, b16)
    torch.cuda.synchronize()
    
    start = time.perf_counter()
    for _ in range(20):
        c16 = torch.matmul(a16, b16)
    torch.cuda.synchronize()
    elapsed16 = (time.perf_counter() - start) * 1000 / 20
    gflops16 = (2 * size**3) / (elapsed16 / 1000) / 1e9
    print(f"  FP16 matmul {size}x{size}: {elapsed16:.3f}ms  ({gflops16:.0f} GFLOPS)")
    
    # BF16 if supported
    try:
        abf = torch.randn(size, size, device=device, dtype=torch.bfloat16)
        bbf = torch.randn(size, size, device=device, dtype=torch.bfloat16)
        for _ in range(5):
            cbf = torch.matmul(abf, bbf)
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        for _ in range(20):
            cbf = torch.matmul(abf, bbf)
        torch.cuda.synchronize()
        elapsed_bf = (time.perf_counter() - start) * 1000 / 20
        gflops_bf = (2 * size**3) / (elapsed_bf / 1000) / 1e9
        print(f"  BF16 matmul {size}x{size}: {elapsed_bf:.3f}ms  ({gflops_bf:.0f} GFLOPS)")
    except Exception as e:
        print(f"  BF16: not supported ({e})")
else:
    print("  (skip — no CUDA)")

# 5. Memory Bandwidth
print("\n--- 5. MEMORY BANDWIDTH ---")
if device.type == "cuda":
    sizes_mb = [1, 4, 16, 64, 256]
    for mb in sizes_mb:
        elements = mb * 1024 * 1024 // 4
        src = torch.randn(elements, device=device)
        dst = torch.empty(elements, device=device)
        for _ in range(5):
            dst.copy_(src)
        torch.cuda.synchronize()
        
        start = time.perf_counter()
        for _ in range(50):
            dst.copy_(src)
        torch.cuda.synchronize()
        elapsed = (time.perf_counter() - start) / 50
        bw = (mb * 2) / elapsed / 1024  # GB/s (read + write)
        print(f"  copy {mb}MB: {elapsed*1000:.3f}ms  ({bw:.0f} GB/s)")
else:
    print("  (skip — no CUDA)")

# 6. VRAM Summary
print("\n--- 6. VRAM Usage ---")
if device.type == "cuda":
    allocated = torch.cuda.memory_allocated() / (1024**2)
    reserved = torch.cuda.memory_reserved() / (1024**2)
    max_alloc = torch.cuda.max_memory_allocated() / (1024**2)
    print(f"  allocated:  {allocated:.1f} MB")
    print(f"  reserved:   {reserved:.1f} MB")
    print(f"  peak alloc: {max_alloc:.1f} MB")
else:
    print("  (no CUDA)")

print("\n" + "=" * 64)
print("   Metal-Dead CUDA RTX 3060 Benchmark COMPLETADO")
print("   Binary Is Binary")
print("=" * 64)
