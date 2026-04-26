# I/O — Runtime 2.0 💀🦈

Serialización de tensores — formato binario `.pdb` (PyDead-BIB).

- `tensor_save/load` — tensor individual
- `tensor_save_all/load_all` — múltiples tensores con nombres (model weights)
- `tensor_save_raw/load_raw` — raw float binary (interop)

## Formato .pdb
```
[magic "PDB\0" 4B][ndim 4B][shape ndim*8B][data size*4B]
```

## Estado: ✅ Implementado
