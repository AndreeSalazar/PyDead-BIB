# Autograd — Runtime 2.0 💀🦈

Diferenciación automática tape-based para entrenamiento.

**Estado: ✅ Implementado** — tape_create, tape_record, tape_backward, ag_* ops.

## API

- `tape_create(capacity)` → crea tape con capacidad para N nodos
- `ag_add/mul/matmul/relu/softmax(tape, ...)` → forward + registro en tape
- `ag_cross_entropy(tape, logits, labels, batch)` → loss + registro
- `tape_backward(tape)` → reverse-mode AD, calcula gradientes
- `tape_zero_grads(tape)` → resetea gradientes para siguiente iteración

## Backward ops implementados

- `OP_ADD` → dA = grad, dB = grad
- `OP_SUB` → dA = grad, dB = -grad
- `OP_MUL` → dA = grad * B, dB = grad * A
- `OP_MATMUL` → dA = grad @ B^T, dB = A^T @ grad
- `OP_RELU` → dX = grad * (X > 0)
- `OP_SOFTMAX` → dX = S * (dY - sum(dY * S))
- `OP_LOSS_CE` → dLogits = (softmax - one_hot) / batch
