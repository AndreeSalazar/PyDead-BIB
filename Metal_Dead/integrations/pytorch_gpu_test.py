"""
PyTorch GPU Test — runs with CPython to verify CUDA
Then Metal_Dead uses PyDead-BIB compiled code for inference
"""
import torch
import time

print("=" * 60)
print("   PyTorch GPU Test para Metal-Dead")
print("   PyDead-BIB v3.0 + PyTorch CUDA")
print("=" * 60)

# GPU detection
if torch.cuda.is_available():
    gpu_name = torch.cuda.get_device_name(0)
    vram = torch.cuda.get_device_properties(0).total_memory / (1024**3)
    print(f"\n   GPU: {gpu_name}")
    print(f"   VRAM: {vram:.1f} GB")
    print(f"   CUDA: {torch.version.cuda}")
    device = torch.device("cuda")
else:
    print("\n   GPU: No disponible, usando CPU")
    device = torch.device("cpu")

print(f"   PyTorch: {torch.__version__}")
print(f"   Device: {device}")

# Quick benchmark
print("\n--- Benchmark GPU ---")

# Matrix multiplication
sizes = [256, 512, 1024, 2048]
for size in sizes:
    a = torch.randn(size, size, device=device)
    b = torch.randn(size, size, device=device)
    
    if device.type == "cuda":
        torch.cuda.synchronize()
    
    start = time.perf_counter()
    for _ in range(10):
        c = torch.matmul(a, b)
    
    if device.type == "cuda":
        torch.cuda.synchronize()
    
    elapsed = (time.perf_counter() - start) * 1000 / 10
    gflops = (2 * size**3) / (elapsed / 1000) / 1e9
    print(f"  matmul {size}x{size}: {elapsed:.2f}ms ({gflops:.1f} GFLOPS)")

# Softmax
print("\n--- Softmax ---")
x = torch.randn(1024, 1024, device=device)
start = time.perf_counter()
for _ in range(100):
    y = torch.softmax(x, dim=-1)
if device.type == "cuda":
    torch.cuda.synchronize()
elapsed = (time.perf_counter() - start) * 1000 / 100
print(f"  softmax 1024x1024: {elapsed:.2f}ms")

# Simple transformer layer simulation
print("\n--- Transformer Layer Sim ---")
batch, seq, embed, heads = 1, 128, 256, 8
head_dim = embed // heads

q = torch.randn(batch, heads, seq, head_dim, device=device)
k = torch.randn(batch, heads, seq, head_dim, device=device)
v = torch.randn(batch, heads, seq, head_dim, device=device)

start = time.perf_counter()
for _ in range(10):
    scores = torch.matmul(q, k.transpose(-2, -1)) / (head_dim ** 0.5)
    weights = torch.softmax(scores, dim=-1)
    attn = torch.matmul(weights, v)
if device.type == "cuda":
    torch.cuda.synchronize()
elapsed = (time.perf_counter() - start) * 1000 / 10
print(f"  attention seq={seq} embed={embed} heads={heads}: {elapsed:.2f}ms")

# VRAM usage
if device.type == "cuda":
    allocated = torch.cuda.memory_allocated() / (1024**2)
    reserved = torch.cuda.memory_reserved() / (1024**2)
    print(f"\n--- VRAM ---")
    print(f"  allocated: {allocated:.1f} MB")
    print(f"  reserved: {reserved:.1f} MB")

print("\n" + "=" * 60)
print("   PyTorch GPU Test COMPLETADO")
print("=" * 60)
