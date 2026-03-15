def vk_init():
    print("Vulkan init")
    return 0

def vk_compute(elements):
    ops = elements * 4
    print(f"Vulkan compute: {ops} ops")
    return ops

def vk_dispatch(x, y, z):
    threads = x * y * z
    print(f"Vulkan dispatch: {threads} threads")
    return threads

def vk_cleanup():
    print("Vulkan cleanup done")
    return 0

vk_init()
ops = vk_compute(1024)
threads = vk_dispatch(32, 32, 1)
vk_cleanup()
print("vulkan dispatch ok")
