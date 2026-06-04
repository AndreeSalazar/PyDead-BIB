# hello.py - GPU Matrix Multiplication Test
# This file is parsed by rustpython-parser, lowered to IR, and executed on the GPU via Vulkan.

a = Tensor([2, 2])
b = Tensor([2, 2])
c = a @ b
