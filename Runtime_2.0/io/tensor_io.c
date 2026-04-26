// ============================================================
// PyDead-BIB Runtime 2.0 — Tensor I/O 💀🦈
// Save/Load tensores — formato binario determinista
// ============================================================

#include "tensor_io.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Magic bytes: "PDB\0" = PyDead-BIB
#define PDB_MAGIC 0x00424450

// ── Single tensor save/load ──────────────────────────────

int tensor_save(const Tensor* t, const char* path) {
    FILE* f = fopen(path, "wb");
    if (!f) return -1;

    uint32_t magic = PDB_MAGIC;
    fwrite(&magic, sizeof(uint32_t), 1, f);
    fwrite(&t->ndim, sizeof(int32_t), 1, f);
    fwrite(t->shape, sizeof(int64_t), t->ndim, f);
    fwrite(t->data, sizeof(float), t->size, f);

    fclose(f);
    return 0;
}

Tensor* tensor_load(const char* path) {
    FILE* f = fopen(path, "rb");
    if (!f) return NULL;

    uint32_t magic;
    fread(&magic, sizeof(uint32_t), 1, f);
    if (magic != PDB_MAGIC) { fclose(f); return NULL; }

    int32_t ndim;
    fread(&ndim, sizeof(int32_t), 1, f);

    int64_t shape[TENSOR_MAX_DIMS];
    fread(shape, sizeof(int64_t), ndim, f);

    Tensor* t = tensor_create(shape, ndim);
    if (!t) { fclose(f); return NULL; }

    fread(t->data, sizeof(float), t->size, f);
    fclose(f);
    return t;
}

// ── Multi-tensor save/load (model weights) ───────────────
// Format:
//   [magic 4B][count 4B]
//   For each tensor:
//     [name_len 4B][name name_len B][ndim 4B][shape ndim*8B][data size*4B]

int tensor_save_all(Tensor** tensors, const char** names,
                     int32_t count, const char* path) {
    FILE* f = fopen(path, "wb");
    if (!f) return -1;

    uint32_t magic = PDB_MAGIC;
    fwrite(&magic, sizeof(uint32_t), 1, f);
    fwrite(&count, sizeof(int32_t), 1, f);

    for (int32_t i = 0; i < count; i++) {
        int32_t name_len = (int32_t)strlen(names[i]);
        fwrite(&name_len, sizeof(int32_t), 1, f);
        fwrite(names[i], 1, name_len, f);
        fwrite(&tensors[i]->ndim, sizeof(int32_t), 1, f);
        fwrite(tensors[i]->shape, sizeof(int64_t), tensors[i]->ndim, f);
        fwrite(tensors[i]->data, sizeof(float), tensors[i]->size, f);
    }

    fclose(f);
    return 0;
}

int tensor_load_all(const char* path, Tensor*** tensors,
                     char*** names, int32_t* count) {
    FILE* f = fopen(path, "rb");
    if (!f) return -1;

    uint32_t magic;
    fread(&magic, sizeof(uint32_t), 1, f);
    if (magic != PDB_MAGIC) { fclose(f); return -1; }

    fread(count, sizeof(int32_t), 1, f);

    *tensors = (Tensor**)malloc(*count * sizeof(Tensor*));
    *names = (char**)malloc(*count * sizeof(char*));

    for (int32_t i = 0; i < *count; i++) {
        int32_t name_len;
        fread(&name_len, sizeof(int32_t), 1, f);
        (*names)[i] = (char*)malloc(name_len + 1);
        fread((*names)[i], 1, name_len, f);
        (*names)[i][name_len] = '\0';

        int32_t ndim;
        fread(&ndim, sizeof(int32_t), 1, f);

        int64_t shape[TENSOR_MAX_DIMS];
        fread(shape, sizeof(int64_t), ndim, f);

        (*tensors)[i] = tensor_create(shape, ndim);
        fread((*tensors)[i]->data, sizeof(float), (*tensors)[i]->size, f);
    }

    fclose(f);
    return 0;
}

// ── Raw binary ──────────────────────────────────────────

int tensor_save_raw(const Tensor* t, const char* path) {
    FILE* f = fopen(path, "wb");
    if (!f) return -1;
    fwrite(t->data, sizeof(float), t->size, f);
    fclose(f);
    return 0;
}

Tensor* tensor_load_raw(const char* path, const int64_t* shape, int32_t ndim) {
    Tensor* t = tensor_create(shape, ndim);
    if (!t) return NULL;

    FILE* f = fopen(path, "rb");
    if (!f) { tensor_free(t); return NULL; }
    fread(t->data, sizeof(float), t->size, f);
    fclose(f);
    return t;
}
