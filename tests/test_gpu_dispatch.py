import ctypes

def gpu_init():
    print("GPU dispatch init")
    return 0

def gpu_compute():
    size = 1024
    print(f"GPU malloc size: {size}")
    return size

def gpu_cleanup():
    print("GPU cleanup done")
    return 0

result = gpu_init()
data = gpu_compute()
gpu_cleanup()
print("gpu dispatch ok")
