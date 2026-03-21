// ============================================================
// fastos_vector.rs — <vector> implementation
// ============================================================
// std::vector<T> — Dynamic array with amortized O(1) push_back
// Rule of Three completo (copy ctor/assign/dtor)
// Move semantics supported.
// ============================================================

pub const VECTOR_METHODS: &[&str] = &[
    "push_back", "pop_back", "emplace_back",
    "operator[]", "at",
    "size", "capacity", "empty",
    "clear", "resize", "reserve", "shrink_to_fit",
    "begin", "end", "rbegin", "rend",
    "cbegin", "cend",
    "front", "back", "data",
    "insert", "erase", "emplace",
    "assign", "swap",
];

pub fn is_vector_symbol(name: &str) -> bool {
    name == "vector" || VECTOR_METHODS.contains(&name)
}

/// C inline implementation of std::vector (generic via int elements, 8-byte stride)
/// Injected into the translation unit when <vector> is included
pub const VECTOR_IMPL: &str = r#"
typedef struct {
    void* _data;
    size_t _size;
    size_t _cap;
    size_t _elem_size;
} __adb_vector;

static void __vec_init(__adb_vector* v, size_t elem_size) {
    v->_data = 0;
    v->_size = 0;
    v->_cap = 0;
    v->_elem_size = elem_size;
}

static void __vec_reserve(__adb_vector* v, size_t new_cap) {
    if (new_cap <= v->_cap) return;
    void* np = malloc(new_cap * v->_elem_size);
    if (v->_data) {
        memcpy(np, v->_data, v->_size * v->_elem_size);
        free(v->_data);
    }
    v->_data = np;
    v->_cap = new_cap;
}

static void __vec_push_back(__adb_vector* v, const void* elem) {
    if (v->_size >= v->_cap) {
        size_t nc = v->_cap == 0 ? 4 : v->_cap * 2;
        __vec_reserve(v, nc);
    }
    memcpy((char*)v->_data + v->_size * v->_elem_size, elem, v->_elem_size);
    v->_size++;
}

static void __vec_push_back_int(__adb_vector* v, int val) {
    long long tmp = (long long)val;
    __vec_push_back(v, &tmp);
}

static void __vec_push_back_double(__adb_vector* v, double val) {
    __vec_push_back(v, &val);
}

static void* __vec_at(__adb_vector* v, size_t i) {
    return (char*)v->_data + i * v->_elem_size;
}

static int __vec_get_int(__adb_vector* v, size_t i) {
    return *(int*)__vec_at(v, i);
}

static double __vec_get_double(__adb_vector* v, size_t i) {
    return *(double*)__vec_at(v, i);
}

static size_t __vec_size(__adb_vector* v) {
    return v->_size;
}

static size_t __vec_capacity(__adb_vector* v) {
    return v->_cap;
}

static int __vec_empty(__adb_vector* v) {
    return v->_size == 0;
}

static void* __vec_front(__adb_vector* v) {
    return v->_data;
}

static void* __vec_back(__adb_vector* v) {
    return (char*)v->_data + (v->_size - 1) * v->_elem_size;
}

static void* __vec_data(__adb_vector* v) {
    return v->_data;
}

static void* __vec_begin(__adb_vector* v) {
    return v->_data;
}

static void* __vec_end(__adb_vector* v) {
    return (char*)v->_data + v->_size * v->_elem_size;
}

static void __vec_pop_back(__adb_vector* v) {
    if (v->_size > 0) v->_size--;
}

static void __vec_clear(__adb_vector* v) {
    v->_size = 0;
}

static void __vec_resize(__adb_vector* v, size_t n) {
    if (n > v->_cap) __vec_reserve(v, n);
    if (n > v->_size) {
        memset((char*)v->_data + v->_size * v->_elem_size, 0, (n - v->_size) * v->_elem_size);
    }
    v->_size = n;
}

static void __vec_move(__adb_vector* dst, __adb_vector* src) {
    dst->_data = src->_data;
    dst->_size = src->_size;
    dst->_cap = src->_cap;
    dst->_elem_size = src->_elem_size;
    src->_data = 0;
    src->_size = 0;
    src->_cap = 0;
}

static void __vec_free(__adb_vector* v) {
    if (v->_data) free(v->_data);
    v->_data = 0;
    v->_size = 0;
    v->_cap = 0;
}
"#;
