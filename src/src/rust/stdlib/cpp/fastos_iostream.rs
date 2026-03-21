// ============================================================
// fastos_iostream.rs — <iostream> implementation
// ============================================================
// std::cout, std::cin, std::cerr, std::endl
// Implementado sobre fastos_stdio internamente
// operator<< chaining via printf wrappers
// ============================================================

pub const IOSTREAM_OBJECTS: &[&str] = &[
    "cout", "cin", "cerr", "clog",
];

pub const IOSTREAM_MANIPULATORS: &[&str] = &[
    "endl", "flush", "ends",
    "dec", "hex", "oct",
    "fixed", "scientific",
    "left", "right", "internal",
    "boolalpha", "noboolalpha",
    "showbase", "noshowbase",
    "showpoint", "noshowpoint",
    "showpos", "noshowpos",
    "uppercase", "nouppercase",
    "setw", "setprecision", "setfill",
];

pub const IOSTREAM_CLASSES: &[&str] = &[
    "ostream", "istream", "iostream",
    "ofstream", "ifstream", "fstream",
    "ostringstream", "istringstream", "stringstream",
];

pub fn is_iostream_symbol(name: &str) -> bool {
    IOSTREAM_OBJECTS.contains(&name)
        || IOSTREAM_MANIPULATORS.contains(&name)
        || IOSTREAM_CLASSES.contains(&name)
}

/// C inline implementation of iostream (cout/cin/cerr via printf/scanf)
/// operator<< returns ostream* for chaining
pub const IOSTREAM_IMPL: &str = r#"
typedef struct {
    int _fd;
    int _base;
} __adb_ostream;

typedef struct {
    int _fd;
} __adb_istream;

static __adb_ostream __adb_cout_obj = {1, 10};
static __adb_ostream __adb_cerr_obj = {2, 10};
static __adb_istream __adb_cin_obj = {0};

static __adb_ostream* __cout_str(__adb_ostream* os, const char* s) {
    printf("%s", s);
    return os;
}

static __adb_ostream* __cout_int(__adb_ostream* os, int v) {
    if (os->_base == 16) printf("%x", v);
    else printf("%d", v);
    return os;
}

static __adb_ostream* __cout_long(__adb_ostream* os, long v) {
    if (os->_base == 16) printf("%lx", v);
    else printf("%ld", v);
    return os;
}

static __adb_ostream* __cout_double(__adb_ostream* os, double v) {
    printf("%f", v);
    return os;
}

static __adb_ostream* __cout_char(__adb_ostream* os, char c) {
    printf("%c", c);
    return os;
}

static __adb_ostream* __cout_bool(__adb_ostream* os, int b) {
    printf("%s", b ? "true" : "false");
    return os;
}

static __adb_ostream* __cout_endl(__adb_ostream* os) {
    printf("\n");
    return os;
}

static __adb_ostream* __cout_hex(__adb_ostream* os) {
    os->_base = 16;
    return os;
}

static __adb_ostream* __cout_dec(__adb_ostream* os) {
    os->_base = 10;
    return os;
}

static __adb_istream* __cin_int(__adb_istream* is, int* v) {
    scanf("%d", v);
    return is;
}

static __adb_istream* __cin_str(__adb_istream* is, char* buf) {
    scanf("%s", buf);
    return is;
}
"#;
