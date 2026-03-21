// ============================================================
// fastos_map.rs — <map> implementation
// ============================================================
// std::map<K,V> — Red-Black Tree ordered map
// ============================================================

pub const MAP_METHODS: &[&str] = &[
    "operator[]", "at",
    "insert", "emplace", "erase",
    "find", "count", "contains",
    "lower_bound", "upper_bound", "equal_range",
    "size", "empty", "clear",
    "begin", "end", "rbegin", "rend",
    "swap",
];

pub fn is_map_symbol(name: &str) -> bool {
    name == "map" || name == "multimap"
        || name == "unordered_map" || name == "unordered_multimap"
        || MAP_METHODS.contains(&name)
}

/// C inline implementation of std::map (sorted array) and std::unordered_map (hash table)
/// Keys are C strings, values are ints for simplicity
pub const MAP_IMPL: &str = r#"
typedef struct {
    char key[64];
    int value;
} __adb_map_entry;

typedef struct {
    __adb_map_entry* _data;
    size_t _size;
    size_t _cap;
} __adb_map;

static void __map_init(__adb_map* m) {
    m->_data = 0;
    m->_size = 0;
    m->_cap = 0;
}

static int __map_find_idx(__adb_map* m, const char* key) {
    for (size_t i = 0; i < m->_size; i++) {
        if (strcmp(m->_data[i].key, key) == 0) return (int)i;
    }
    return -1;
}

static void __map_insert_sorted(__adb_map* m, const char* key, int val) {
    int idx = __map_find_idx(m, key);
    if (idx >= 0) { m->_data[idx].value = val; return; }
    if (m->_size >= m->_cap) {
        size_t nc = m->_cap == 0 ? 8 : m->_cap * 2;
        __adb_map_entry* np = (__adb_map_entry*)malloc(nc * sizeof(__adb_map_entry));
        if (m->_data) {
            memcpy(np, m->_data, m->_size * sizeof(__adb_map_entry));
            free(m->_data);
        }
        m->_data = np;
        m->_cap = nc;
    }
    size_t pos = m->_size;
    for (size_t i = 0; i < m->_size; i++) {
        if (strcmp(key, m->_data[i].key) < 0) { pos = i; break; }
    }
    for (size_t i = m->_size; i > pos; i--) {
        m->_data[i] = m->_data[i - 1];
    }
    strcpy(m->_data[pos].key, key);
    m->_data[pos].value = val;
    m->_size++;
}

static int* __map_get(__adb_map* m, const char* key) {
    int idx = __map_find_idx(m, key);
    if (idx >= 0) return &m->_data[idx].value;
    __map_insert_sorted(m, key, 0);
    idx = __map_find_idx(m, key);
    return &m->_data[idx].value;
}

static int __map_count(__adb_map* m, const char* key) {
    return __map_find_idx(m, key) >= 0 ? 1 : 0;
}

static void __map_erase(__adb_map* m, const char* key) {
    int idx = __map_find_idx(m, key);
    if (idx < 0) return;
    for (size_t i = (size_t)idx; i < m->_size - 1; i++) {
        m->_data[i] = m->_data[i + 1];
    }
    m->_size--;
}

static size_t __map_size(__adb_map* m) { return m->_size; }
static int __map_empty(__adb_map* m) { return m->_size == 0; }

static void __map_free(__adb_map* m) {
    if (m->_data) free(m->_data);
    m->_data = 0;
    m->_size = 0;
    m->_cap = 0;
}

static unsigned int __hash_str(const char* s) {
    unsigned int h = 5381;
    while (*s) { h = h * 33 + (unsigned char)*s; s++; }
    return h;
}

typedef struct {
    __adb_map_entry* _buckets;
    char* _used;
    size_t _cap;
    size_t _size;
} __adb_umap;

static void __umap_init(__adb_umap* m) {
    m->_cap = 64;
    m->_buckets = (__adb_map_entry*)malloc(64 * sizeof(__adb_map_entry));
    m->_used = (char*)malloc(64);
    memset(m->_used, 0, 64);
    m->_size = 0;
}

static int* __umap_get(__adb_umap* m, const char* key) {
    unsigned int h = __hash_str(key) % m->_cap;
    for (size_t i = 0; i < m->_cap; i++) {
        size_t idx = (h + i) % m->_cap;
        if (!m->_used[idx]) {
            strcpy(m->_buckets[idx].key, key);
            m->_buckets[idx].value = 0;
            m->_used[idx] = 1;
            m->_size++;
            return &m->_buckets[idx].value;
        }
        if (strcmp(m->_buckets[idx].key, key) == 0) {
            return &m->_buckets[idx].value;
        }
    }
    return 0;
}

static size_t __umap_size(__adb_umap* m) { return m->_size; }

static void __umap_free(__adb_umap* m) {
    if (m->_buckets) free(m->_buckets);
    if (m->_used) free(m->_used);
    m->_buckets = 0;
    m->_used = 0;
    m->_size = 0;
}
"#;
