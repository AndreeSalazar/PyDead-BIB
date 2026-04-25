# Autograd — Runtime 2.0 💀🦈

Diferenciación automática tape-based para entrenamiento.

**Estado: 📐 Diseño** — header con estructuras definidas, implementación pendiente.

## Concepto
- `Tape` graba cada operación forward
- `tape_backward()` recorre al revés calculando gradientes
- Solo necesita 4 backward ops para MLPs: matmul, add, relu, softmax+CE
