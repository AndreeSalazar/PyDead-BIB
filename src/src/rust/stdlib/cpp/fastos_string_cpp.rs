// ============================================================
// fastos_string_cpp.rs — <string> implementation
// ============================================================
// std::string — Dynamic string with SSO (Small String Optimization)
// Strings <= 15 chars stored inline, longer strings on heap.
// ============================================================

pub const STRING_METHODS: &[&str] = &[
    "length", "size", "empty", "clear",
    "c_str", "data",
    "operator[]", "at",
    "operator+", "operator+=", "operator==", "operator!=",
    "operator<", "operator>", "operator<=", "operator>=",
    "find", "rfind", "find_first_of", "find_last_of",
    "find_first_not_of", "find_last_not_of",
    "substr", "append", "insert", "erase", "replace",
    "compare", "copy",
    "begin", "end", "rbegin", "rend",
    "front", "back",
    "push_back", "pop_back",
    "resize", "reserve", "capacity",
    "swap",
];

pub const STRING_CONSTANTS: &[(&str, &str)] = &[
    ("npos", "SIZE_MAX"),
];

pub fn is_string_cpp_symbol(name: &str) -> bool {
    name == "string" || name == "basic_string" || name == "wstring"
        || STRING_METHODS.contains(&name)
        || STRING_CONSTANTS.iter().any(|(n, _)| *n == name)
}

/// C inline implementation of std::string with SSO
/// Injected into the translation unit when <string> is included
pub const STRING_IMPL: &str = r#"
typedef struct {
    char* _ptr;
    size_t _size;
    size_t _cap;
    char _buf[24];
} __adb_string;

static __adb_string __str_new(const char* src) {
    __adb_string s;
    size_t len = strlen(src);
    if (len <= 22) {
        memcpy(s._buf, src, len + 1);
        s._ptr = s._buf;
        s._size = len;
        s._cap = 23;
    } else {
        s._ptr = (char*)malloc(len + 1);
        memcpy(s._ptr, src, len + 1);
        s._size = len;
        s._cap = len + 1;
    }
    return s;
}

static __adb_string __str_new_empty() {
    __adb_string s;
    s._buf[0] = 0;
    s._ptr = s._buf;
    s._size = 0;
    s._cap = 23;
    return s;
}

static const char* __str_cstr(const __adb_string* s) {
    return s->_ptr;
}

static size_t __str_size(const __adb_string* s) {
    return s->_size;
}

static size_t __str_length(const __adb_string* s) {
    return s->_size;
}

static int __str_empty(const __adb_string* s) {
    return s->_size == 0;
}

static size_t __str_capacity(const __adb_string* s) {
    return s->_cap;
}

static char __str_at(const __adb_string* s, size_t i) {
    return s->_ptr[i];
}

static char __str_front(const __adb_string* s) {
    return s->_ptr[0];
}

static char __str_back(const __adb_string* s) {
    return s->_ptr[s->_size - 1];
}

static void __str_reserve(__adb_string* s, size_t new_cap) {
    if (new_cap <= s->_cap) return;
    char* np = (char*)malloc(new_cap);
    memcpy(np, s->_ptr, s->_size + 1);
    if (s->_ptr != s->_buf) free(s->_ptr);
    s->_ptr = np;
    s->_cap = new_cap;
}

static void __str_append(__adb_string* s, const char* src) {
    size_t slen = strlen(src);
    size_t need = s->_size + slen + 1;
    if (need > s->_cap) {
        size_t nc = s->_cap * 2;
        if (nc < need) nc = need;
        __str_reserve(s, nc);
    }
    memcpy(s->_ptr + s->_size, src, slen + 1);
    s->_size += slen;
}

static void __str_push_back(__adb_string* s, char c) {
    char tmp[2];
    tmp[0] = c;
    tmp[1] = 0;
    __str_append(s, tmp);
}

static __adb_string __str_concat(const __adb_string* a, const __adb_string* b) {
    __adb_string r = __str_new(a->_ptr);
    __str_append(&r, b->_ptr);
    return r;
}

static __adb_string __str_concat_cstr(const __adb_string* a, const char* b) {
    __adb_string r = __str_new(a->_ptr);
    __str_append(&r, b);
    return r;
}

static int __str_eq(const __adb_string* a, const __adb_string* b) {
    if (a->_size != b->_size) return 0;
    return strcmp(a->_ptr, b->_ptr) == 0;
}

static int __str_ne(const __adb_string* a, const __adb_string* b) {
    return !__str_eq(a, b);
}

static int __str_lt(const __adb_string* a, const __adb_string* b) {
    return strcmp(a->_ptr, b->_ptr) < 0;
}

static int __str_compare(const __adb_string* a, const __adb_string* b) {
    return strcmp(a->_ptr, b->_ptr);
}

static __adb_string __str_substr(const __adb_string* s, size_t pos, size_t len) {
    __adb_string r = __str_new_empty();
    if (pos >= s->_size) return r;
    size_t avail = s->_size - pos;
    if (len > avail) len = avail;
    __str_reserve(&r, len + 1);
    memcpy(r._ptr, s->_ptr + pos, len);
    r._ptr[len] = 0;
    r._size = len;
    return r;
}

static size_t __str_find(const __adb_string* s, const char* needle) {
    const char* p = strstr(s->_ptr, needle);
    if (p) return (size_t)(p - s->_ptr);
    return (size_t)-1;
}

static void __str_clear(__adb_string* s) {
    s->_size = 0;
    s->_ptr[0] = 0;
}

static void __str_free(__adb_string* s) {
    if (s->_ptr != s->_buf) free(s->_ptr);
}
"#;
