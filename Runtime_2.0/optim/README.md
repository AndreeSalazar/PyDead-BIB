# Optim — Runtime 2.0 💀🦈

Optimizadores para entrenamiento de redes neuronales.

- `SGD` — Stochastic Gradient Descent con momentum y weight decay
- `Adam` — AdamW con bias correction, momentos first/second order

## API
- `sgd_create(lr, momentum, weight_decay, num_params)` → SGD*
- `sgd_step(opt, params, grads, n)` → actualiza parámetros
- `adam_create(lr, beta1, beta2, eps, weight_decay, num_params)` → Adam*
- `adam_step(opt, params, grads, n)` → actualiza parámetros

## Estado: ✅ Implementado
