// ============================================================
// ADead-BIB Built-in C++ Standard Library Headers
// ============================================================
// Provides C++ standard library declarations as built-in strings.
// When #include <iostream> is found, we inject these declarations
// directly — no filesystem, no libstdc++, no libc++ needed.
//
// Strategy: inject only flat C-style declarations that the parser
// already handles. STL types (vector, string, cout) are recognized
// by the parser's type_names set and handled specially during IR
// conversion. The headers only need to declare functions.
//
// ADead-BIB owns the headers. 💀🦈
// ============================================================

/// Common C++ prologue — fundamental types and C-compatible declarations
pub const CPP_COMMON_PROLOGUE: &str = r#"
typedef unsigned long size_t;
typedef long ptrdiff_t;
typedef long intptr_t;
typedef unsigned long uintptr_t;

int printf(const char *format, ...);
int scanf(const char *format, ...);
int sprintf(char *str, const char *format, ...);
int puts(const char *s);
int putchar(int c);
void *malloc(size_t size);
void *calloc(size_t num, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);
void *memcpy(void *dest, const void *src, size_t n);
void *memset(void *s, int c, size_t n);
size_t strlen(const char *s);
int strcmp(const char *s1, const char *s2);
char *strcpy(char *dest, const char *src);
int atoi(const char *s);
double atof(const char *s);
void exit(int status);
void abort();
"#;

/// Look up a C++ header by name and return its declarations.
/// All headers inject flat C-compatible declarations only.
/// STL types are recognized by the parser's type_names prescan.
pub fn get_cpp_header(name: &str) -> Option<&'static str> {
    match name {
        // C++ Standard Library — real implementations injected as C inline code
        "iostream" | "iomanip" | "sstream" | "fstream" => Some(HEADER_IO),
        "string" | "string_view" => Some(HEADER_EMPTY),
        "vector" | "array" | "list" | "deque" | "forward_list" => Some(HEADER_EMPTY),
        "map" | "unordered_map" | "set" | "unordered_set" => Some(HEADER_EMPTY),
        "stack" | "queue" | "span" => Some(HEADER_EMPTY),
        "algorithm" | "numeric" | "ranges" => Some(HEADER_EMPTY),
        "memory" | "functional" | "utility" | "tuple" => Some(HEADER_EMPTY),
        "optional" | "variant" | "any" => Some(HEADER_EMPTY),
        "type_traits" => Some(HEADER_EMPTY),
        "limits" | "concepts" => Some(HEADER_EMPTY),
        "chrono" | "thread" | "mutex" | "atomic" | "future" | "condition_variable" => {
            Some(HEADER_EMPTY)
        }
        "initializer_list" | "iterator" => Some(HEADER_EMPTY),
        "stdexcept" | "exception" => Some(HEADER_EMPTY),
        "regex" | "random" | "filesystem" | "format" | "coroutine" | "numbers" | "bit" => {
            Some(HEADER_EMPTY)
        }
        "cassert" => Some(HEADER_EMPTY),
        "cstdio" | "stdio.h" => Some(HEADER_IO),
        "cstdlib" | "stdlib.h" => Some(HEADER_CSTDLIB),
        "cstring" | "string.h" => Some(HEADER_CSTRING),
        "cmath" | "math.h" => Some(HEADER_CMATH),
        "climits" | "cstdint" | "stdint.h" | "inttypes.h" => Some(HEADER_CLIMITS),
        "cstddef" | "stddef.h" => Some(HEADER_EMPTY),

        // ==========================================
        // ADead-BIB v7.0 — header_main.h (HEREDA TODO)
        // ==========================================
        // Un solo include. Todo C + C++ disponible. Sin linker.
        "header_main.h" => Some(HEADER_MAIN_CPP_COMPLETE),

        // ==========================================
        // ADead-BIB v7.0 — fastos C++ headers (aliases)
        // ==========================================
        "fastos_iostream" => Some(HEADER_IO),
        "fastos_vector" => Some(HEADER_EMPTY),
        "fastos_string_cpp" => Some(HEADER_EMPTY),
        "fastos_map" => Some(HEADER_EMPTY),
        "fastos_memory" => Some(HEADER_EMPTY),
        "fastos_algorithm" => Some(HEADER_EMPTY),
        "fastos_functional" => Some(HEADER_EMPTY),
        "fastos_utility" => Some(HEADER_EMPTY),
        "fastos_exception" => Some(HEADER_EMPTY),

        // fastos C headers (C-compatible in C++ mode)
        "fastos_stdio.h" => Some(HEADER_IO),
        "fastos_stdlib.h" => Some(HEADER_CSTDLIB),
        "fastos_string.h" => Some(HEADER_CSTRING),
        "fastos_math.h" => Some(HEADER_CMATH),
        "fastos_types.h" => Some(HEADER_CLIMITS),

        // ==========================================
        // DirectX 12 fastos headers
        // ==========================================
        "fastos_windows.h" => Some(HEADER_FASTOS_WINDOWS),
        "fastos_wrl.h" => Some(HEADER_FASTOS_WRL),
        "fastos_d3d12.h" => Some(HEADER_FASTOS_D3D12),
        "fastos_dxgi.h" => Some(HEADER_FASTOS_DXGI),

        _ => None,
    }
}

/// Check if a symbol name is a known C++ stdlib function/type/class.
/// Uses the stdlib/cpp/ registries for authoritative lookup.
pub fn is_known_cpp_symbol(name: &str) -> bool {
    use crate::stdlib::cpp::fastos_iostream;
    use crate::stdlib::cpp::fastos_vector;
    use crate::stdlib::cpp::fastos_string_cpp;
    use crate::stdlib::cpp::fastos_map;
    use crate::stdlib::cpp::fastos_memory;
    use crate::stdlib::cpp::fastos_algorithm;
    use crate::stdlib::cpp::fastos_functional;
    use crate::stdlib::cpp::fastos_utility;
    use crate::stdlib::cpp::fastos_exceptions;
    use crate::stdlib::cpp::fastos_set;
    use crate::stdlib::cpp::fastos_list;
    use crate::stdlib::cpp::fastos_deque;
    use crate::stdlib::cpp::fastos_stack_queue;
    use crate::stdlib::cpp::fastos_array;
    use crate::stdlib::cpp::fastos_tuple;
    use crate::stdlib::cpp::fastos_optional;
    use crate::stdlib::cpp::fastos_variant;
    use crate::stdlib::cpp::fastos_any;
    use crate::stdlib::cpp::fastos_chrono;
    use crate::stdlib::cpp::fastos_thread;
    use crate::stdlib::cpp::fastos_future;
    use crate::stdlib::cpp::fastos_mutex;
    use crate::stdlib::cpp::fastos_atomic;
    use crate::stdlib::cpp::fastos_condition_variable;
    use crate::stdlib::cpp::fastos_regex;
    use crate::stdlib::cpp::fastos_random;
    use crate::stdlib::cpp::fastos_filesystem;
    use crate::stdlib::cpp::fastos_numeric;
    use crate::stdlib::cpp::fastos_string_view;
    use crate::stdlib::cpp::fastos_span;
    use crate::stdlib::cpp::fastos_initializer_list;
    use crate::stdlib::cpp::fastos_iterator;

    fastos_iostream::is_iostream_symbol(name)
        || fastos_vector::is_vector_symbol(name)
        || fastos_string_cpp::is_string_cpp_symbol(name)
        || fastos_map::is_map_symbol(name)
        || fastos_memory::is_memory_symbol(name)
        || fastos_algorithm::is_algorithm_symbol(name)
        || fastos_functional::is_functional_symbol(name)
        || fastos_utility::is_utility_symbol(name)
        || fastos_exceptions::is_exception_symbol(name)
        || fastos_set::is_set_symbol(name)
        || fastos_list::is_list_symbol(name)
        || fastos_deque::is_deque_symbol(name)
        || fastos_stack_queue::is_stack_queue_symbol(name)
        || fastos_array::is_array_symbol(name)
        || fastos_tuple::is_tuple_symbol(name)
        || fastos_optional::is_optional_symbol(name)
        || fastos_variant::is_variant_symbol(name)
        || fastos_any::is_any_symbol(name)
        || fastos_chrono::is_chrono_symbol(name)
        || fastos_thread::is_thread_symbol(name)
        || fastos_future::is_future_symbol(name)
        || fastos_mutex::is_mutex_symbol(name)
        || fastos_atomic::is_atomic_symbol(name)
        || fastos_condition_variable::is_condition_variable_symbol(name)
        || fastos_regex::is_regex_symbol(name)
        || fastos_random::is_random_symbol(name)
        || fastos_filesystem::is_filesystem_symbol(name)
        || fastos_numeric::is_numeric_symbol(name)
        || fastos_string_view::is_string_view_symbol(name)
        || fastos_span::is_span_symbol(name)
        || fastos_initializer_list::is_initializer_list_symbol(name)
        || fastos_iterator::is_iterator_symbol(name)
}

// ========================================
// Header constants — flat C-compatible declarations only
// STL types (vector, string, cout, etc.) are recognized by the
// parser's prescan and handled during IR lowering.
// ========================================

/// Empty header — no declarations needed, types recognized by parser
pub const HEADER_EMPTY: &str = "";

// NOTE: Real C implementations of std::string (SSO), std::vector (move semantics),
// std::iostream (operator<< chains), and std::function (type erasure) are defined in
// the fastos_*.rs modules under src/rust/stdlib/cpp/. They serve as the specification
// for how these types should behave. The parser recognizes std:: types via prescan,
// and IR lowering handles method dispatch. The C inline code is not injected via
// headers because the parser can't handle complex C struct/function definitions
// in the preprocessor output. Instead, the ISA compiler handles these types natively.

/// I/O header — injects printf/scanf/puts
pub const HEADER_IO: &str = r#"
int printf(const char *format, ...);
int scanf(const char *format, ...);
int sprintf(char *str, const char *format, ...);
int snprintf(char *str, size_t size, const char *format, ...);
int puts(const char *s);
int putchar(int c);
int getchar();
"#;

/// <cstdlib> / <stdlib.h>
pub const HEADER_CSTDLIB: &str = r#"
void *malloc(size_t size);
void *calloc(size_t num, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);
int atoi(const char *s);
long atol(const char *s);
double atof(const char *s);
void exit(int status);
void abort();
int abs(int x);
long labs(long x);
int rand();
void srand(unsigned int seed);
int system(const char *command);
char *getenv(const char *name);
"#;

/// <cstring> / <string.h>
pub const HEADER_CSTRING: &str = r#"
void *memcpy(void *dest, const void *src, size_t n);
void *memmove(void *dest, const void *src, size_t n);
void *memset(void *s, int c, size_t n);
int memcmp(const void *s1, const void *s2, size_t n);
size_t strlen(const char *s);
int strcmp(const char *s1, const char *s2);
int strncmp(const char *s1, const char *s2, size_t n);
char *strcpy(char *dest, const char *src);
char *strncpy(char *dest, const char *src, size_t n);
char *strcat(char *dest, const char *src);
char *strchr(const char *s, int c);
char *strrchr(const char *s, int c);
char *strstr(const char *haystack, const char *needle);
char *strdup(const char *s);
"#;

/// <cmath> / <math.h>
pub const HEADER_CMATH: &str = r#"
double sin(double x);
double cos(double x);
double tan(double x);
double asin(double x);
double acos(double x);
double atan(double x);
double atan2(double y, double x);
double exp(double x);
double log(double x);
double log2(double x);
double log10(double x);
double pow(double base, double exp);
double sqrt(double x);
double cbrt(double x);
double ceil(double x);
double floor(double x);
double round(double x);
double fabs(double x);
double fmod(double x, double y);
double hypot(double x, double y);
int abs(int x);
"#;

/// <climits> / <cstdint>
#[allow(dead_code)]
pub const HEADER_CLIMITS: &str = r#"
typedef signed char int8_t;
typedef short int16_t;
typedef int int32_t;
typedef long int64_t;
typedef unsigned char uint8_t;
typedef unsigned short uint16_t;
typedef unsigned int uint32_t;
typedef unsigned long uint64_t;
"#;

/// <type_traits> — C++11/14/17/20 type traits
/// ADead-BIB implements these as template structs with static constexpr value.
/// The parser recognizes these as known template types.
pub const HEADER_TYPE_TRAITS: &str = r#"
/* ADead-BIB <type_traits> — C++11/14/17/20 */

/* integral_constant */
template<typename T, T v>
struct integral_constant {
    static constexpr T value = v;
};

typedef integral_constant<bool, true> true_type;
typedef integral_constant<bool, false> false_type;

/* Primary type categories */
template<typename T> struct is_void : false_type {};
template<> struct is_void<void> : true_type {};

template<typename T> struct is_integral : false_type {};
template<> struct is_integral<bool> : true_type {};
template<> struct is_integral<char> : true_type {};
template<> struct is_integral<short> : true_type {};
template<> struct is_integral<int> : true_type {};
template<> struct is_integral<long> : true_type {};

template<typename T> struct is_floating_point : false_type {};
template<> struct is_floating_point<float> : true_type {};
template<> struct is_floating_point<double> : true_type {};

template<typename T> struct is_pointer : false_type {};
template<typename T> struct is_pointer<T*> : true_type {};

template<typename T> struct is_reference : false_type {};
template<typename T> struct is_reference<T&> : true_type {};
template<typename T> struct is_reference<T&&> : true_type {};

template<typename T> struct is_array : false_type {};

template<typename T> struct is_const : false_type {};
template<typename T> struct is_const<const T> : true_type {};

/* Type relationships */
template<typename T, typename U> struct is_same : false_type {};
template<typename T> struct is_same<T, T> : true_type {};

/* Type modifications */
template<typename T> struct remove_const { typedef T type; };
template<typename T> struct remove_const<const T> { typedef T type; };

template<typename T> struct remove_volatile { typedef T type; };
template<typename T> struct remove_volatile<volatile T> { typedef T type; };

template<typename T> struct remove_cv { typedef T type; };
template<typename T> struct remove_cv<const T> { typedef T type; };
template<typename T> struct remove_cv<volatile T> { typedef T type; };
template<typename T> struct remove_cv<const volatile T> { typedef T type; };

template<typename T> struct remove_reference { typedef T type; };
template<typename T> struct remove_reference<T&> { typedef T type; };
template<typename T> struct remove_reference<T&&> { typedef T type; };

template<typename T> struct remove_pointer { typedef T type; };
template<typename T> struct remove_pointer<T*> { typedef T type; };

template<typename T> struct add_pointer { typedef T* type; };
template<typename T> struct add_const { typedef const T type; };
template<typename T> struct add_lvalue_reference { typedef T& type; };
template<typename T> struct add_rvalue_reference { typedef T&& type; };

/* SFINAE helpers */
template<bool B, typename T = void> struct enable_if {};
template<typename T> struct enable_if<true, T> { typedef T type; };

template<bool B, typename T, typename F> struct conditional { typedef T type; };
template<typename T, typename F> struct conditional<false, T, F> { typedef F type; };

/* C++14 _t aliases (template type aliases) */
template<typename T> using remove_const_t = typename remove_const<T>::type;
template<typename T> using remove_volatile_t = typename remove_volatile<T>::type;
template<typename T> using remove_cv_t = typename remove_cv<T>::type;
template<typename T> using remove_reference_t = typename remove_reference<T>::type;
template<typename T> using remove_pointer_t = typename remove_pointer<T>::type;
template<typename T> using add_pointer_t = typename add_pointer<T>::type;
template<typename T> using add_const_t = typename add_const<T>::type;
template<bool B, typename T = void> using enable_if_t = typename enable_if<B, T>::type;
template<bool B, typename T, typename F> using conditional_t = typename conditional<B, T, F>::type;

/* C++17 _v aliases (variable templates) */
template<typename T, typename U> constexpr bool is_same_v = is_same<T, U>::value;
template<typename T> constexpr bool is_integral_v = is_integral<T>::value;
template<typename T> constexpr bool is_floating_point_v = is_floating_point<T>::value;
template<typename T> constexpr bool is_pointer_v = is_pointer<T>::value;
template<typename T> constexpr bool is_reference_v = is_reference<T>::value;
template<typename T> constexpr bool is_void_v = is_void<T>::value;
template<typename T> constexpr bool is_const_v = is_const<T>::value;
template<typename T> constexpr bool is_array_v = is_array<T>::value;

/* void_t (C++17 SFINAE helper) */
template<typename...> using void_t = void;

/* decay — strips references and cv-qualifiers */
template<typename T> struct decay { typedef T type; };
template<typename T> struct decay<T&> { typedef T type; };
template<typename T> struct decay<T&&> { typedef T type; };
template<typename T> struct decay<const T> { typedef T type; };
template<typename T> struct decay<volatile T> { typedef T type; };
template<typename T> using decay_t = typename decay<T>::type;
"#;

// ================================================================
// ADead-BIB v7.0 — header_main.h for C++ (COMPLETE)
// ================================================================
// Includes ALL C declarations + C++ stream/STL type recognition
// Sin linker externo — NUNCA
// ================================================================

const HEADER_MAIN_CPP_COMPLETE: &str = r#"
/* header_main.h — ADead-BIB Universal Header v7.0 (C++ mode) */
/* Un solo include. Todo C + C++ disponible. Sin linker. */

typedef unsigned long size_t;
typedef long ptrdiff_t;
typedef long intptr_t;
typedef unsigned long uintptr_t;

typedef signed char int8_t;
typedef short int16_t;
typedef int int32_t;
typedef long int64_t;
typedef unsigned char uint8_t;
typedef unsigned short uint16_t;
typedef unsigned int uint32_t;
typedef unsigned long uint64_t;

/* C Standard Library (available in C++ mode) */
int printf(const char *format, ...);
int scanf(const char *format, ...);
int sprintf(char *str, const char *format, ...);
int snprintf(char *str, size_t size, const char *format, ...);
int puts(const char *s);
int putchar(int c);
int getchar();

void *malloc(size_t size);
void *calloc(size_t num, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);
int atoi(const char *s);
long atol(const char *s);
double atof(const char *s);
void exit(int status);
void abort();
int abs(int x);
int rand();
void srand(unsigned int seed);

void *memcpy(void *dest, const void *src, size_t n);
void *memmove(void *dest, const void *src, size_t n);
void *memset(void *s, int c, size_t n);
int memcmp(const void *s1, const void *s2, size_t n);
size_t strlen(const char *s);
int strcmp(const char *s1, const char *s2);
char *strcpy(char *dest, const char *src);
char *strncpy(char *dest, const char *src, size_t n);
char *strcat(char *dest, const char *src);
char *strchr(const char *s, int c);
char *strstr(const char *haystack, const char *needle);
char *strdup(const char *s);

double sin(double x);
double cos(double x);
double tan(double x);
double sqrt(double x);
double pow(double base, double exp);
double log(double x);
double log2(double x);
double log10(double x);
double exp(double x);
double ceil(double x);
double floor(double x);
double round(double x);
double fabs(double x);
double fmod(double x, double y);
double atan2(double y, double x);

/* C++ STL types are recognized by parser prescan. */
/* std::cout, std::cin, std::string, std::vector<T>, etc. */
/* No declarations needed — handled during IR lowering. */

/* TREE SHAKING: ADead-BIB includes only what you use. */
/* std::cout << "Hello" → only cout implementation in binary. */

/* === DirectX 12 Headers (fastos) === */
#include <fastos_windows.h>
#include <fastos_wrl.h>
#include <fastos_d3d12.h>
#include <fastos_dxgi.h>
"#;

// ================================================================
// fastos_windows.h — Windows API types and macros
// ================================================================
const HEADER_FASTOS_WINDOWS: &str = r#"
// ================================================================
// fastos_windows.h — ADead-BIB Windows/GDI/DX12 Master Header
// ================================================================
// Este header le da a ADead-BIB TODO el conocimiento de Windows:
//   - Tipos primitivos (BYTE, WORD, DWORD, UINT, HANDLE, etc.)
//   - Constantes Win32 (WS_*, WM_*, SW_*, CS_*, IDC_*, COLOR_*, etc.)
//   - Constantes GDI (PS_*, RGB macro, COLORREF)
//   - Tipos DX12/DXGI (D3D_FEATURE_LEVEL, DXGI_FORMAT, etc.)
//   - Structs ABI (MSG, RECT, POINT, WNDCLASSEXA, GUID, etc.)
//   - Funciones: kernel32, user32, gdi32, d3d12, dxgi
//   - Helpers: CreateWindowSimple, MessageLoop, RGB, DrawLine, etc.
// Inspirado por Rust windows-rs: usar A (ANSI) para strings simples.
// ================================================================

// ===================== TIPOS PRIMITIVOS ==========================
typedef unsigned char BYTE;
typedef unsigned short WORD;
typedef unsigned int UINT;
typedef unsigned long DWORD;
typedef unsigned long long UINT64;
typedef unsigned long long ULONG_PTR;
typedef unsigned long long SIZE_T;
typedef unsigned long long ULONGLONG;
typedef unsigned char UINT8;
typedef unsigned short UINT16;
typedef unsigned int UINT32;
typedef int INT;
typedef long LONG;
typedef long long LONGLONG;
typedef long long INT64;
typedef long HRESULT;
typedef int BOOL;
typedef void* HANDLE;
typedef void* HWND;
typedef void* HINSTANCE;
typedef void* HMODULE;
typedef void* HDC;
typedef void* HICON;
typedef void* HCURSOR;
typedef void* HBRUSH;
typedef void* HMENU;
typedef void* HMONITOR;
typedef void* HPEN;
typedef void* HGDIOBJ;
typedef void* HFONT;
typedef void* HBITMAP;
typedef void* HRGN;
typedef void* LPVOID;
typedef const void* LPCVOID;
typedef char* LPSTR;
typedef const char* LPCSTR;
typedef wchar_t* LPWSTR;
typedef const wchar_t* LPCWSTR;
typedef wchar_t WCHAR;
typedef float FLOAT;
typedef const char* PCSTR;
typedef const wchar_t* PCWSTR;
typedef unsigned int COLORREF;
typedef ULONG_PTR WPARAM;
typedef LONG LPARAM;
typedef LONG LRESULT;
typedef void* WNDPROC;

// ===================== CONSTANTES WIN32 ==========================
// Todas como #define para evitar generar datos en la data section
// Window Styles (WS_*)
#define WS_OVERLAPPED       0x00000000
#define WS_POPUP            0x80000000
#define WS_CHILD            0x40000000
#define WS_VISIBLE          0x10000000
#define WS_CAPTION          0x00C00000
#define WS_SYSMENU          0x00080000
#define WS_THICKFRAME       0x00040000
#define WS_MINIMIZEBOX      0x00020000
#define WS_MAXIMIZEBOX      0x00010000
#define WS_OVERLAPPEDWINDOW 0x00CF0000

// ShowWindow Commands (SW_*)
#define SW_HIDE            0
#define SW_SHOWNORMAL      1
#define SW_SHOW            5
#define SW_SHOWDEFAULT     10

// Window Messages (WM_*)
#define WM_NULL           0x0000
#define WM_CREATE         0x0001
#define WM_DESTROY        0x0002
#define WM_MOVE           0x0003
#define WM_SIZE           0x0005
#define WM_CLOSE          0x0010
#define WM_QUIT           0x0012
#define WM_PAINT          0x000F
#define WM_KEYDOWN        0x0100
#define WM_KEYUP          0x0101
#define WM_CHAR           0x0102
#define WM_COMMAND        0x0111
#define WM_TIMER          0x0113
#define WM_MOUSEMOVE      0x0200
#define WM_LBUTTONDOWN    0x0201
#define WM_LBUTTONUP      0x0202
#define WM_RBUTTONDOWN    0x0204
#define WM_RBUTTONUP      0x0205

// PeekMessage flags
#define PM_NOREMOVE 0x0000
#define PM_REMOVE   0x0001

// Class Styles (CS_*)
#define CS_HREDRAW        0x0002
#define CS_VREDRAW        0x0001
#define CS_OWNDC          0x0020

// System Cursors (IDC_*)
#define IDC_ARROW          32512
#define IDC_IBEAM          32513
#define IDC_WAIT           32514
#define IDC_CROSS          32515
#define IDC_HAND           32649

// System Colors (COLOR_*)
#define COLOR_WINDOW       5
#define COLOR_BTNFACE      15
#define COLOR_BACKGROUND   1

// Virtual Key Codes (VK_*)
#define VK_ESCAPE          0x1B
#define VK_RETURN          0x0D
#define VK_SPACE           0x20
#define VK_LEFT            0x25
#define VK_UP              0x26
#define VK_RIGHT           0x27
#define VK_DOWN            0x28

// HRESULT values
#define S_OK           0
#define S_FALSE        1
#define E_FAIL         (-1)
#define E_NOINTERFACE  (-2147467262)
#define E_INVALIDARG   (-2147024809)

// ===================== CONSTANTES GDI ============================
// Pen Styles (PS_*)
#define PS_SOLID           0
#define PS_DASH            1
#define PS_DOT             2
#define PS_DASHDOT         3
#define PS_NULL            5

// Stock Objects
#define NULL_BRUSH         5
#define WHITE_BRUSH        0
#define BLACK_BRUSH        4
#define WHITE_PEN          6
#define BLACK_PEN          7

// ===================== CONSTANTES DX12/DXGI ======================
// D3D_FEATURE_LEVEL
#define D3D_FEATURE_LEVEL_11_0 0xB000
#define D3D_FEATURE_LEVEL_11_1 0xB100
#define D3D_FEATURE_LEVEL_12_0 0xC000
#define D3D_FEATURE_LEVEL_12_1 0xC100

// DXGI_FORMAT (most common)
#define DXGI_FORMAT_R8G8B8A8_UNORM 28
#define DXGI_FORMAT_B8G8R8A8_UNORM 87
#define DXGI_FORMAT_D32_FLOAT      40
#define DXGI_FORMAT_R32_FLOAT      41

// DXGI_USAGE
#define DXGI_USAGE_RENDER_TARGET_OUTPUT 32

// DXGI_SWAP_EFFECT
#define DXGI_SWAP_EFFECT_FLIP_DISCARD 4

// D3D12 command list types
#define D3D12_COMMAND_LIST_TYPE_DIRECT  0
#define D3D12_COMMAND_LIST_TYPE_BUNDLE  1
#define D3D12_COMMAND_LIST_TYPE_COMPUTE 2
#define D3D12_COMMAND_LIST_TYPE_COPY    3

// D3D12 descriptor heap types
#define D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV 0
#define D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER     1
#define D3D12_DESCRIPTOR_HEAP_TYPE_RTV         2
#define D3D12_DESCRIPTOR_HEAP_TYPE_DSV         3

// D3D12 resource states
#define D3D12_RESOURCE_STATE_PRESENT       0
#define D3D12_RESOURCE_STATE_RENDER_TARGET 4

// D3D12 fence flags
#define D3D12_FENCE_FLAG_NONE 0

// ===================== STRUCTS ===================================
struct GUID {
    DWORD Data1;
    DWORD Data2_3;
    DWORD Data4_lo;
    DWORD Data4_hi;
};
typedef GUID IID;
typedef const GUID* REFGUID;
typedef const IID* REFIID;

struct LARGE_INTEGER {
    LONGLONG QuadPart;
};

struct RECT {
    LONG left;
    LONG top;
    LONG right;
    LONG bottom;
};

struct POINT {
    LONG x;
    LONG y;
};

struct MSG {
    HWND hwnd;
    UINT message;
    ULONG_PTR wParam;
    LONG lParam;
    DWORD time;
    POINT pt;
};

struct SECURITY_ATTRIBUTES {
    DWORD nLength;
    LPVOID lpSecurityDescriptor;
    BOOL bInheritHandle;
};

struct IUnknown {
    virtual UINT AddRef() = 0;
    virtual UINT Release() = 0;
    virtual HRESULT QueryInterface(REFIID riid, void** ppvObject) = 0;
};

// ANSI window class (same layout as Rust windows-rs WNDCLASSEXA)
struct WNDCLASSEXA {
    UINT cbSize;
    UINT style;
    WNDPROC lpfnWndProc;
    int cbClsExtra;
    int cbWndExtra;
    HINSTANCE hInstance;
    HICON hIcon;
    HCURSOR hCursor;
    HBRUSH hbrBackground;
    LPCSTR lpszMenuName;
    LPCSTR lpszClassName;
    HICON hIconSm;
};

struct WNDCLASSEXW {
    UINT cbSize;
    UINT style;
    WNDPROC lpfnWndProc;
    int cbClsExtra;
    int cbWndExtra;
    HINSTANCE hInstance;
    HICON hIcon;
    HCURSOR hCursor;
    HBRUSH hbrBackground;
    LPCWSTR lpszMenuName;
    LPCWSTR lpszClassName;
    HICON hIconSm;
};

// ===================== HELPER MACROS =============================
inline COLORREF RGB(int r, int g, int b) { return r + g * 256 + b * 65536; }
inline LONG HIWORD(ULONG_PTR l) { return (LONG)((l >> 16) & 0xFFFF); }
inline LONG LOWORD(ULONG_PTR l) { return (LONG)(l & 0xFFFF); }
int SUCCEEDED(HRESULT hr) { return hr >= 0; }
int FAILED(HRESULT hr) { return hr < 0; }

// ===================== WIN32 API (kernel32.dll) ==================
extern "C" {
HMODULE GetModuleHandleA(LPCSTR lpModuleName);
HMODULE GetModuleHandleW(LPCWSTR lpModuleName);
void ExitProcess(UINT uExitCode);
HANDLE CreateEventA(void* lpSecurity, BOOL bManualReset, BOOL bInitialState, LPCSTR lpName);
DWORD WaitForSingleObject(HANDLE hHandle, DWORD dwMilliseconds);
BOOL CloseHandle(HANDLE hObject);
void Sleep(DWORD dwMilliseconds);

// ===================== MSVCRT ====================================
void* memset(void* dest, int c, int count);
void* memcpy(void* dest, const void* src, int count);

// ===================== INTRINSICS ================================
// __store32(ptr, byte_offset, value) — writes 4 bytes at ptr+offset
// Used for GUID construction and 4-byte struct field writes
void __store32(void* ptr, int offset, int value);

// ===================== USER32 (ANSI) =============================
UINT RegisterClassExA(const WNDCLASSEXA* lpwcx);
HWND CreateWindowExA(DWORD dwExStyle, LPCSTR lpClassName, LPCSTR lpWindowName, DWORD dwStyle, int X, int Y, int nWidth, int nHeight, HWND hWndParent, HMENU hMenu, HINSTANCE hInstance, LPVOID lpParam);
LRESULT DefWindowProcA(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam);
BOOL PeekMessageA(void* lpMsg, HWND hWnd, UINT wMsgFilterMin, UINT wMsgFilterMax, UINT wRemoveMsg);
LRESULT DispatchMessageA(const void* lpMsg);

// ===================== USER32 (Wide) =============================
UINT RegisterClassExW(const WNDCLASSEXW* lpwcx);
HWND CreateWindowExW(DWORD dwExStyle, LPCWSTR lpClassName, LPCWSTR lpWindowName, DWORD dwStyle, int X, int Y, int nWidth, int nHeight, HWND hWndParent, HMENU hMenu, HINSTANCE hInstance, LPVOID lpParam);
LRESULT DefWindowProcW(HWND hWnd, UINT Msg, WPARAM wParam, LPARAM lParam);
BOOL GetMessageW(void* lpMsg, HWND hWnd, UINT wMsgFilterMin, UINT wMsgFilterMax);
LRESULT DispatchMessageW(const void* lpMsg);

// ===================== USER32 (shared) ===========================
BOOL ShowWindow(HWND hWnd, int nCmdShow);
BOOL UpdateWindow(HWND hWnd);
BOOL TranslateMessage(const void* lpMsg);
void PostQuitMessage(int nExitCode);
HCURSOR LoadCursorW(HINSTANCE hInstance, int lpCursorName);
BOOL AdjustWindowRect(RECT* lpRect, DWORD dwStyle, BOOL bMenu);
HDC GetDC(HWND hWnd);
int ReleaseDC(HWND hWnd, HDC hDC);
BOOL InvalidateRect(HWND hWnd, const RECT* lpRect, BOOL bErase);
int FillRect(HDC hDC, const RECT* lprc, HBRUSH hbr);

// ===================== GDI32 =====================================
COLORREF SetPixel(HDC hdc, int x, int y, COLORREF color);
HBRUSH CreateSolidBrush(COLORREF color);
BOOL DeleteObject(HGDIOBJ ho);
HGDIOBJ SelectObject(HDC hdc, HGDIOBJ h);
BOOL Rectangle(HDC hdc, int left, int top, int right, int bottom);
HPEN CreatePen(int iStyle, int cWidth, COLORREF color);
BOOL MoveToEx(HDC hdc, int x, int y, void* lppt);
BOOL LineTo(HDC hdc, int x, int y);
BOOL Polygon(HDC hdc, const void* apt, int cpt);

// ===================== D3D12 =====================================
HRESULT D3D12CreateDevice(void* pAdapter, int MinimumFeatureLevel, void* riid, void** ppDevice);
HRESULT D3D12GetDebugInterface(void* riid, void** ppvDebug);
HRESULT D3D12SerializeRootSignature(const void* pRootSignature, UINT Version, void** ppBlob, void** ppErrorBlob);

// ===================== DXGI ======================================
HRESULT CreateDXGIFactory1(void* riid, void** ppFactory);
HRESULT CreateDXGIFactory2(UINT Flags, void* riid, void** ppFactory);
}

// ===================== HELPER FUNCTIONS ==========================
// ADead-BIB convenience wrappers — make Win32/GDI calls trivial
// NOTA: Funciones con >4 args (CreateWindowExA) deben llamarse directo
//       desde main() — no desde inline helpers (limitación del codegen).

// Allocate a MSG buffer on the heap (avoids stack struct ABI issues)
// Usage: void* msg = AllocMSG();
inline void* AllocMSG() {
    return malloc(64);
}

// Draw a line from (x1,y1) to (x2,y2) with given color and width
// Usage: DrawLine(hdc, 0, 0, 100, 100, RGB(255,0,0), 2);
inline void DrawLine(HDC hdc, int x1, int y1, int x2, int y2, COLORREF color, int width) {
    HPEN pen = CreatePen(PS_SOLID, width, color);
    HGDIOBJ old = SelectObject(hdc, pen);
    MoveToEx(hdc, x1, y1, 0);
    LineTo(hdc, x2, y2);
    SelectObject(hdc, old);
    DeleteObject(pen);
}

// Fill a gradient triangle (red top -> green/blue bottom)
// Scanline rasterizer using SetPixel. Step=2 for speed.
// Usage: FillGradientTriangle(hdc, 640, 100, 340, 550, 940, 550);
inline void FillGradientTriangle(HDC hdc, int tx, int ty, int lx, int ly, int rx, int ry) {
    int height = ly - ty;
    if (height <= 0) { return; }
    int y = ty;
    while (y <= ly) {
        int t = y - ty;
        int xl = tx + (lx - tx) * t / height;
        int xr = tx + (rx - tx) * t / height;
        int r = 255 - t * 200 / height;
        int g = t * 200 / height;
        int b = t * 100 / height;
        if (r < 0) { r = 0; }
        COLORREF c = RGB(r, g, b);
        int x = xl;
        while (x <= xr) {
            SetPixel(hdc, x, y, c);
            x = x + 2;
        }
        y = y + 2;
    }
}

// Draw triangle outline with given color and pen width
// Usage: DrawTriangleOutline(hdc, 640, 100, 340, 550, 940, 550, RGB(255,255,255), 3);
inline void DrawTriangleOutline(HDC hdc, int tx, int ty, int lx, int ly, int rx, int ry, COLORREF color, int width) {
    HPEN pen = CreatePen(PS_SOLID, width, color);
    HGDIOBJ old = SelectObject(hdc, pen);
    MoveToEx(hdc, tx, ty, 0);
    LineTo(hdc, lx, ly);
    LineTo(hdc, rx, ry);
    LineTo(hdc, tx, ty);
    SelectObject(hdc, old);
    DeleteObject(pen);
}

// Run the message loop (keeps window alive until user closes)
// Usage: MessageLoop();
inline void MessageLoop() {
    void* pmsg = malloc(64);
    int running = 1;
    while (running) {
        if (PeekMessageA(pmsg, 0, 0, 0, PM_REMOVE)) {
            TranslateMessage(pmsg);
            DispatchMessageA(pmsg);
        }
    }
}
"#;

// ================================================================
// fastos_wrl.h — ComPtr<T>
// ================================================================
const HEADER_FASTOS_WRL: &str = r#"
namespace Microsoft {
namespace WRL {
template<typename T>
class ComPtr {
public:
    T* ptr;
    ComPtr() : ptr(0) {}
    ~ComPtr() { if (ptr) { ptr->Release(); ptr = 0; } }
    T* Get() const { return ptr; }
    T** GetAddressOf() { return &ptr; }
    T* operator->() const { return ptr; }
    T** operator&() { return &ptr; }
    void Reset() { if (ptr) { ptr->Release(); ptr = 0; } }
    T* Detach() { T* tmp = ptr; ptr = 0; return tmp; }
    operator bool() const { return ptr != 0; }
};
}
}
using Microsoft::WRL::ComPtr;
"#;

// ================================================================
// fastos_d3d12.h — D3D12 interfaces (minimal for HelloTriangle)
// ================================================================
const HEADER_FASTOS_D3D12: &str = r#"
struct D3D12_COMMAND_QUEUE_DESC {
    UINT Type;
    INT Priority;
    UINT Flags;
    UINT NodeMask;
};

struct D3D12_DESCRIPTOR_HEAP_DESC {
    UINT Type;
    UINT NumDescriptors;
    UINT Flags;
    UINT NodeMask;
};

struct D3D12_CPU_DESCRIPTOR_HANDLE {
    UINT64 ptr;
};

struct D3D12_GPU_DESCRIPTOR_HANDLE {
    UINT64 ptr;
};

struct D3D12_VERTEX_BUFFER_VIEW {
    UINT64 BufferLocation;
    UINT SizeInBytes;
    UINT StrideInBytes;
};

struct D3D12_INPUT_ELEMENT_DESC {
    LPCSTR SemanticName;
    UINT SemanticIndex;
    UINT Format;
    UINT InputSlot;
    UINT AlignedByteOffset;
    UINT InputSlotClass;
    UINT InstanceDataStepRate;
};

struct D3D12_VIEWPORT {
    FLOAT TopLeftX;
    FLOAT TopLeftY;
    FLOAT Width;
    FLOAT Height;
    FLOAT MinDepth;
    FLOAT MaxDepth;
};

struct D3D12_RECT {
    LONG left;
    LONG top;
    LONG right;
    LONG bottom;
};

struct D3D12_RESOURCE_BARRIER {
    UINT Type;
    UINT Flags;
};

struct D3D12_HEAP_PROPERTIES {
    UINT Type;
    UINT CPUPageProperty;
    UINT MemoryPoolPreference;
    UINT CreationNodeMask;
    UINT VisibleNodeMask;
};

struct D3D12_RESOURCE_DESC {
    UINT Dimension;
    UINT64 Alignment;
    UINT64 Width;
    UINT Height;
    UINT16 DepthOrArraySize;
    UINT16 MipLevels;
    UINT Format;
    UINT SampleCount;
    UINT SampleQuality;
    UINT Layout;
    UINT Flags;
};

struct ID3D12Object : public IUnknown {
    virtual HRESULT SetName(LPCWSTR Name) = 0;
};
struct ID3D12DeviceChild : public ID3D12Object {};
struct ID3D12Pageable : public ID3D12DeviceChild {};

struct ID3D12Resource : public ID3D12Pageable {
    virtual HRESULT Map(UINT Subresource, const void* pReadRange, void** ppData) = 0;
    virtual void Unmap(UINT Subresource, const void* pWrittenRange) = 0;
    virtual UINT64 GetGPUVirtualAddress() = 0;
};

struct ID3D12CommandAllocator : public ID3D12Pageable {
    virtual HRESULT Reset() = 0;
};

struct ID3D12Fence : public ID3D12Pageable {
    virtual UINT64 GetCompletedValue() = 0;
    virtual HRESULT SetEventOnCompletion(UINT64 Value, HANDLE hEvent) = 0;
    virtual HRESULT Signal(UINT64 Value) = 0;
};

struct ID3D12DescriptorHeap : public ID3D12Pageable {
    virtual D3D12_CPU_DESCRIPTOR_HANDLE GetCPUDescriptorHandleForHeapStart() = 0;
};

struct ID3D12RootSignature : public ID3D12DeviceChild {};
struct ID3D12PipelineState : public ID3D12Pageable {};
struct ID3D12CommandList : public ID3D12DeviceChild {};

struct ID3D12GraphicsCommandList : public ID3D12CommandList {
    virtual HRESULT Close() = 0;
    virtual HRESULT Reset(ID3D12CommandAllocator* pAllocator, ID3D12PipelineState* pInitialState) = 0;
    virtual void RSSetViewports(UINT NumViewports, const D3D12_VIEWPORT* pViewports) = 0;
    virtual void RSSetScissorRects(UINT NumRects, const D3D12_RECT* pRects) = 0;
    virtual void DrawInstanced(UINT VertexCountPerInstance, UINT InstanceCount, UINT StartVertexLocation, UINT StartInstanceLocation) = 0;
};

struct ID3D12CommandQueue : public ID3D12Pageable {
    virtual void ExecuteCommandLists(UINT NumCommandLists, ID3D12CommandList* const* ppCommandLists) = 0;
    virtual HRESULT Signal(ID3D12Fence* pFence, UINT64 Value) = 0;
};

struct ID3D12Device : public ID3D12Object {
    virtual HRESULT CreateCommandQueue(const D3D12_COMMAND_QUEUE_DESC* pDesc, REFIID riid, void** ppCommandQueue) = 0;
    virtual HRESULT CreateCommandAllocator(UINT type, REFIID riid, void** ppCommandAllocator) = 0;
    virtual HRESULT CreateFence(UINT64 InitialValue, UINT Flags, REFIID riid, void** ppFence) = 0;
    virtual HRESULT CreateDescriptorHeap(const D3D12_DESCRIPTOR_HEAP_DESC* pDesc, REFIID riid, void** ppvHeap) = 0;
    virtual UINT GetDescriptorHandleIncrementSize(UINT DescriptorHeapType) = 0;
    virtual HRESULT CreateRenderTargetView(ID3D12Resource* pResource, const void* pDesc, D3D12_CPU_DESCRIPTOR_HANDLE DestDescriptor) = 0;
};

namespace DirectX {
    struct XMFLOAT2 {
        float x, y;
        XMFLOAT2() : x(0), y(0) {}
        XMFLOAT2(float _x, float _y) : x(_x), y(_y) {}
    };
    struct XMFLOAT3 {
        float x, y, z;
        XMFLOAT3() : x(0), y(0), z(0) {}
        XMFLOAT3(float _x, float _y, float _z) : x(_x), y(_y), z(_z) {}
    };
    struct XMFLOAT4 {
        float x, y, z, w;
        XMFLOAT4() : x(0), y(0), z(0), w(0) {}
        XMFLOAT4(float _x, float _y, float _z, float _w) : x(_x), y(_y), z(_z), w(_w) {}
    };
}
using namespace DirectX;
"#;

// ================================================================
// fastos_dxgi.h — DXGI interfaces (minimal)
// ================================================================
const HEADER_FASTOS_DXGI: &str = r#"
struct DXGI_SAMPLE_DESC {
    UINT Count;
    UINT Quality;
};

struct DXGI_SWAP_CHAIN_DESC1 {
    UINT Width;
    UINT Height;
    UINT Format;
    BOOL Stereo;
    DXGI_SAMPLE_DESC SampleDesc;
    UINT BufferUsage;
    UINT BufferCount;
    UINT Scaling;
    UINT SwapEffect;
    UINT AlphaMode;
    UINT Flags;
};

struct DXGI_ADAPTER_DESC1 {
    WCHAR Description[128];
    UINT VendorId;
    UINT DeviceId;
    UINT SubSysId;
    UINT Revision;
    UINT64 DedicatedVideoMemory;
    UINT64 DedicatedSystemMemory;
    UINT64 SharedSystemMemory;
    LARGE_INTEGER AdapterLuid;
    UINT Flags;
};

struct IDXGIObject : public IUnknown {};
struct IDXGIAdapter : public IDXGIObject {};
struct IDXGIAdapter1 : public IDXGIAdapter {
    virtual HRESULT GetDesc1(DXGI_ADAPTER_DESC1* pDesc) = 0;
};
struct IDXGIOutput : public IDXGIObject {};

struct IDXGISwapChain : public IDXGIObject {
    virtual HRESULT Present(UINT SyncInterval, UINT Flags) = 0;
    virtual HRESULT GetBuffer(UINT Buffer, REFIID riid, void** ppSurface) = 0;
};
struct IDXGISwapChain1 : public IDXGISwapChain {};
struct IDXGISwapChain3 : public IDXGISwapChain1 {
    virtual UINT GetCurrentBackBufferIndex() = 0;
};

struct IDXGIFactory : public IDXGIObject {};
struct IDXGIFactory1 : public IDXGIFactory {
    virtual HRESULT EnumAdapters1(UINT Adapter, IDXGIAdapter1** ppAdapter) = 0;
};
struct IDXGIFactory4 : public IDXGIFactory1 {};
"#;
