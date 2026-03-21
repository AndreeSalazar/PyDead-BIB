// ============================================================
// ADead-BIB — C Compiler Extensions (GCC + MSVC)
// ============================================================
// Handles compiler-specific extensions that appear in real-world
// C99/C11 code targeting GCC or MSVC:
//
//   GCC:  __attribute__((X)), __asm__, typeof, __alignof__,
//         __builtin_*, GNU statement expressions ({ ... })
//
//   MSVC: __declspec(X), __cdecl, __stdcall, __fastcall,
//         __int8/16/32/64, __assume, __debugbreak,
//         __volatile, __restrict, __unaligned
//
//   Both: alternate keywords __inline__, __volatile__, __const__
//
// The preprocessor / lexer calls into this module when it sees
// double-underscore-prefixed identifiers.
//
// Windows-compatible C headers are also declared here so that
// c_stdlib.rs can include them via `get_header()`.
// ============================================================

// ── Compiler extension keywords ─────────────────────────────────────────────

/// Test whether an identifier is a GCC keyword extension.
pub fn is_gcc_keyword(kw: &str) -> bool {
    matches!(
        kw,
        "__asm__"
            | "__asm"
            | "asm"
            | "__volatile__"
            | "__volatile"
            | "__inline__"
            | "__inline"
            | "__restrict"
            | "__restrict__"
            | "__const__"
            | "__signed__"
            | "__signed"
            | "__typeof__"
            | "__typeof"
            | "typeof"
            | "__alignof__"
            | "__alignof"
            | "__extension__"
            | "__attribute__"
            | "__attribute"
            | "__builtin_va_list"
            | "__gnuc_va_list"
    )
}

/// Test whether an identifier is an MSVC keyword extension.
pub fn is_msvc_keyword(kw: &str) -> bool {
    matches!(
        kw,
        "__cdecl"
            | "__stdcall"
            | "__fastcall"
            | "__thiscall"
            | "__vectorcall"
            | "__clrcall"
            | "__forceinline"
            | "__declspec"
            | "__int8"
            | "__int16"
            | "__int32"
            | "__int64"
            | "__int128"
            | "__w64"
            | "__ptr32"
            | "__ptr64"
            | "__unaligned"
            | "__assume"
            | "__debugbreak"
            | "__noop"
            | "__cpuid"
            | "__cpuidex"
            | "__rdtsc"
            | "__rdtscp"
            | "__pragma"
            | "_Pragma"
    )
}

/// Normalise alternate keywords to their canonical C equivalent.
///
/// Returns `Some(canonical)` if the keyword should be replaced, else `None`.
pub fn normalize_keyword(kw: &str) -> Option<&'static str> {
    match kw {
        "__inline__" | "__inline" | "__forceinline" => Some("inline"),
        "__volatile__" | "__volatile" => Some("volatile"),
        "__const__" | "__const" => Some("const"),
        "__signed__" | "__signed" => Some("signed"),
        "__restrict__" | "__restrict" => Some("restrict"),
        "__typeof__" | "__typeof" | "typeof" => Some("__typeof__"),
        "__alignof__" | "__alignof" | "_Alignof" => Some("_Alignof"),
        "__extension__" => Some(""), // skip
        "__int8" => Some("signed char"),
        "__int16" => Some("short"),
        "__int32" => Some("int"),
        "__int64" => Some("long long"),
        "wchar_t" => Some("unsigned short"),
        _ => None,
    }
}

// ── Windows-compatible C headers ─────────────────────────────────────────────

/// Minimal `windows.h` stub — declares the most commonly used Win32 types
/// and functions so that C code targeting Windows can compile.
pub const HEADER_WINDOWS: &str = r#"
/* windows.h — ADead-BIB built-in stub */
typedef void*          HANDLE;
typedef void*          HMODULE;
typedef void*          HINSTANCE;
typedef void*          HWND;
typedef void*          HDC;
typedef void*          HGDIOBJ;
typedef void*          LPVOID;
typedef const void*    LPCVOID;
typedef char*          LPSTR;
typedef const char*    LPCSTR;
typedef unsigned short LPWSTR;
typedef const unsigned short* LPCWSTR;
typedef unsigned long  DWORD;
typedef unsigned short WORD;
typedef unsigned char  BYTE;
typedef int            BOOL;
typedef long           LONG;
typedef long long      LONGLONG;
typedef unsigned long  ULONG;
typedef unsigned long long ULONGLONG;
typedef unsigned long  UINT;
typedef unsigned long long UINT64;
typedef long           HRESULT;
typedef long long      INT_PTR;
typedef unsigned long long UINT_PTR;
typedef long long      LONG_PTR;
typedef unsigned long long ULONG_PTR;
typedef unsigned long long SIZE_T;
typedef long long      SSIZE_T;

typedef struct _SECURITY_ATTRIBUTES {
    DWORD nLength;
    LPVOID lpSecurityDescriptor;
    BOOL bInheritHandle;
} SECURITY_ATTRIBUTES, *LPSECURITY_ATTRIBUTES;

typedef struct _OVERLAPPED {
    ULONG_PTR Internal;
    ULONG_PTR InternalHigh;
    DWORD Offset;
    DWORD OffsetHigh;
    HANDLE hEvent;
} OVERLAPPED, *LPOVERLAPPED;

typedef struct _FILETIME {
    DWORD dwLowDateTime;
    DWORD dwHighDateTime;
} FILETIME, *LPFILETIME;

typedef struct _SYSTEMTIME {
    WORD wYear, wMonth, wDayOfWeek, wDay;
    WORD wHour, wMinute, wSecond, wMilliseconds;
} SYSTEMTIME, *LPSYSTEMTIME;

/* Kernel32 */
HANDLE CreateFileA(LPCSTR, DWORD, DWORD, LPSECURITY_ATTRIBUTES, DWORD, DWORD, HANDLE);
BOOL   CloseHandle(HANDLE);
BOOL   ReadFile(HANDLE, LPVOID, DWORD, DWORD*, LPOVERLAPPED);
BOOL   WriteFile(HANDLE, LPCVOID, DWORD, DWORD*, LPOVERLAPPED);
DWORD  GetLastError(void);
void   SetLastError(DWORD);
LPVOID VirtualAlloc(LPVOID, SIZE_T, DWORD, DWORD);
BOOL   VirtualFree(LPVOID, SIZE_T, DWORD);
BOOL   VirtualProtect(LPVOID, SIZE_T, DWORD, DWORD*);
HANDLE GetCurrentProcess(void);
HANDLE GetCurrentThread(void);
DWORD  GetCurrentProcessId(void);
DWORD  GetCurrentThreadId(void);
void   ExitProcess(UINT);
void   ExitThread(DWORD);
HMODULE LoadLibraryA(LPCSTR);
BOOL    FreeLibrary(HMODULE);
void*   GetProcAddress(HMODULE, LPCSTR);
void    Sleep(DWORD);
DWORD   WaitForSingleObject(HANDLE, DWORD);
HANDLE  CreateThread(LPSECURITY_ATTRIBUTES, SIZE_T, void*, LPVOID, DWORD, DWORD*);
HANDLE  CreateMutexA(LPSECURITY_ATTRIBUTES, BOOL, LPCSTR);
BOOL    ReleaseMutex(HANDLE);
void    GetSystemTimeAsFileTime(LPFILETIME);
DWORD   GetTickCount(void);
BOOL    QueryPerformanceCounter(long long*);
BOOL    QueryPerformanceFrequency(long long*);
void*   HeapAlloc(HANDLE, DWORD, SIZE_T);
HANDLE  GetProcessHeap(void);
BOOL    HeapFree(HANDLE, DWORD, void*);
void    OutputDebugStringA(LPCSTR);
int     MultiByteToWideChar(UINT, DWORD, LPCSTR, int, unsigned short*, int);
int     WideCharToMultiByte(UINT, DWORD, const unsigned short*, int, LPSTR, int, LPCSTR, BOOL*);

/* Constants */
#define INVALID_HANDLE_VALUE ((HANDLE)(long long)(-1))
#define TRUE  1
#define FALSE 0
#define NULL  ((void*)0)
/* File access */
#define GENERIC_READ    0x80000000UL
#define GENERIC_WRITE   0x40000000UL
#define FILE_SHARE_READ 0x00000001UL
#define CREATE_ALWAYS   2
#define OPEN_EXISTING   3
#define FILE_ATTRIBUTE_NORMAL 0x00000080UL
/* Virtual memory */
#define MEM_COMMIT      0x1000
#define MEM_RESERVE     0x2000
#define MEM_RELEASE     0x8000
#define PAGE_READWRITE  0x04
#define PAGE_EXECUTE_READWRITE 0x40
/* Wait */
#define INFINITE        0xFFFFFFFFUL
#define WAIT_OBJECT_0   0x00000000UL
#define WAIT_TIMEOUT    0x00000102UL
"#;

/// `winnt.h` — core NT types and constants.
pub const HEADER_WINNT: &str = r#"
/* winnt.h — ADead-BIB built-in stub */
typedef unsigned char  UCHAR;
typedef unsigned short USHORT;
typedef unsigned long  ULONG;
typedef long           NTSTATUS;
typedef void*          PVOID;
typedef char           CHAR;
typedef short          SHORT;

typedef struct _UNICODE_STRING {
    unsigned short Length;
    unsigned short MaximumLength;
    unsigned short* Buffer;
} UNICODE_STRING, *PUNICODE_STRING;

typedef struct _LIST_ENTRY {
    struct _LIST_ENTRY *Flink;
    struct _LIST_ENTRY *Blink;
} LIST_ENTRY, *PLIST_ENTRY;

#define STATUS_SUCCESS          ((NTSTATUS)0x00000000L)
#define STATUS_UNSUCCESSFUL     ((NTSTATUS)0xC0000001L)
#define STATUS_NOT_IMPLEMENTED  ((NTSTATUS)0xC0000002L)
#define STATUS_ACCESS_DENIED    ((NTSTATUS)0xC0000022L)
"#;

/// `windef.h` — fundamental Windows type definitions.
pub const HEADER_WINDEF: &str = r#"
/* windef.h — ADead-BIB built-in stub */
typedef unsigned char  BYTE;
typedef unsigned short WORD;
typedef unsigned long  DWORD;
typedef int            BOOL;
typedef unsigned int   UINT;
typedef void*          HANDLE;
typedef void*          LPVOID;
typedef const void*    LPCVOID;
typedef char*          LPSTR;
typedef const char*    LPCSTR;
typedef long           LONG;

#define WINAPI   __stdcall
#define CALLBACK __stdcall
#define APIENTRY __stdcall
"#;

/// `intrin.h` — MSVC intrinsic function declarations.
pub const HEADER_INTRIN: &str = r#"
/* intrin.h — ADead-BIB built-in stub */
void   __debugbreak(void);
void   __noop(void);
long long __rdtsc(void);
void   __cpuid(int cpuInfo[4], int function_id);
void   __cpuidex(int cpuInfo[4], int function_id, int subfunction_id);
void  *_AddressOfReturnAddress(void);
void  *_ReturnAddress(void);
unsigned char  _BitScanForward(unsigned long *index, unsigned long mask);
unsigned char  _BitScanReverse(unsigned long *index, unsigned long mask);
unsigned char  _BitScanForward64(unsigned long *index, unsigned long long mask);
unsigned char  _BitScanReverse64(unsigned long *index, unsigned long long mask);
unsigned short __bswap_16(unsigned short x);
unsigned int   _byteswap_ulong(unsigned long x);
unsigned long long _byteswap_uint64(unsigned long long x);
unsigned char  _rotl8(unsigned char value, unsigned char shift);
unsigned short _rotl16(unsigned short value, unsigned char shift);
unsigned int   _rotl(unsigned int value, int shift);
unsigned long long _rotl64(unsigned long long value, int shift);
long _InterlockedIncrement(volatile long *p);
long _InterlockedDecrement(volatile long *p);
long _InterlockedExchange(volatile long *p, long val);
long _InterlockedCompareExchange(volatile long *p, long exchange, long comparand);
"#;

/// `immintrin.h` / AVX/SSE — SIMD intrinsics stub.
pub const HEADER_SIMD_INTRIN: &str = r#"
/* immintrin.h — ADead-BIB built-in SIMD stub */
typedef float  __m128  __attribute__((vector_size(16)));
typedef double __m128d __attribute__((vector_size(16)));
typedef long long __m128i __attribute__((vector_size(16)));
typedef float  __m256  __attribute__((vector_size(32)));
typedef double __m256d __attribute__((vector_size(32)));
typedef long long __m256i __attribute__((vector_size(32)));
typedef float  __m512  __attribute__((vector_size(64)));
typedef double __m512d __attribute__((vector_size(64)));
typedef long long __m512i __attribute__((vector_size(64)));

__m128  _mm_add_ps(__m128 a, __m128 b);
__m128  _mm_sub_ps(__m128 a, __m128 b);
__m128  _mm_mul_ps(__m128 a, __m128 b);
__m128  _mm_div_ps(__m128 a, __m128 b);
__m128d _mm_add_pd(__m128d a, __m128d b);
__m128d _mm_mul_pd(__m128d a, __m128d b);
__m128  _mm_load_ps(const float *p);
void    _mm_store_ps(float *p, __m128 a);
__m128d _mm_load_pd(const double *p);
void    _mm_store_pd(double *p, __m128d a);
__m256  _mm256_add_ps(__m256 a, __m256 b);
__m256  _mm256_mul_ps(__m256 a, __m256 b);
__m256d _mm256_add_pd(__m256d a, __m256d b);
__m256  _mm256_load_ps(const float *p);
void    _mm256_store_ps(float *p, __m256 a);
void    _mm_sfence(void);
void    _mm_lfence(void);
void    _mm_mfence(void);
"#;

/// `complex.h` — C99 complex numbers.
pub const HEADER_COMPLEX: &str = r#"
/* complex.h — C99 */
typedef double _Complex double_complex;
typedef float  _Complex float_complex;

double _Complex cadd(double _Complex a, double _Complex b);
double _Complex csub(double _Complex a, double _Complex b);
double _Complex cmul(double _Complex a, double _Complex b);
double _Complex cdiv(double _Complex a, double _Complex b);
double creal(double _Complex z);
double cimag(double _Complex z);
double cabs(double _Complex z);
double carg(double _Complex z);
double _Complex conj(double _Complex z);
double _Complex csqrt(double _Complex z);
double _Complex cexp(double _Complex z);
double _Complex clog(double _Complex z);
double _Complex cpow(double _Complex x, double _Complex y);
double _Complex csin(double _Complex z);
double _Complex ccos(double _Complex z);
"#;

/// `wchar.h` — wide character I/O and string functions.
pub const HEADER_WCHAR: &str = r#"
/* wchar.h */
typedef unsigned int wchar_t;
typedef unsigned long wint_t;
typedef void* mbstate_t;

int wprintf(const wchar_t *format, ...);
int wscanf(const wchar_t *format, ...);
wchar_t *wcscpy(wchar_t *dest, const wchar_t *src);
wchar_t *wcsncpy(wchar_t *dest, const wchar_t *src, size_t n);
wchar_t *wcscat(wchar_t *dest, const wchar_t *src);
size_t  wcslen(const wchar_t *s);
int     wcscmp(const wchar_t *s1, const wchar_t *s2);
int     wcsncmp(const wchar_t *s1, const wchar_t *s2, size_t n);
wchar_t *wcschr(const wchar_t *s, wchar_t c);
wchar_t *wcsstr(const wchar_t *haystack, const wchar_t *needle);
long    wcstol(const wchar_t *nptr, wchar_t **endptr, int base);
double  wcstod(const wchar_t *nptr, wchar_t **endptr);
size_t  mbstowcs(wchar_t *dest, const char *src, size_t n);
size_t  wcstombs(char *dest, const wchar_t *src, size_t n);
int     mbtowc(wchar_t *pwc, const char *s, size_t n);
int     wctomb(char *s, wchar_t wchar);
wint_t  btowc(int c);
int     wctob(wint_t c);
"#;

/// `uchar.h` — C11 Unicode character types.
pub const HEADER_UCHAR: &str = r#"
/* uchar.h — C11 */
typedef unsigned short char16_t;
typedef unsigned int   char32_t;

size_t mbrtoc16(char16_t *pc16, const char *s, size_t n, mbstate_t *ps);
size_t c16rtomb(char *s, char16_t c16, mbstate_t *ps);
size_t mbrtoc32(char32_t *pc32, const char *s, size_t n, mbstate_t *ps);
size_t c32rtomb(char *s, char32_t c32, mbstate_t *ps);
"#;

/// `wctype.h` — wide character classification.
pub const HEADER_WCTYPE: &str = r#"
/* wctype.h */
int iswalpha(int c);
int iswdigit(int c);
int iswalnum(int c);
int iswspace(int c);
int iswupper(int c);
int iswlower(int c);
int iswprint(int c);
int iswpunct(int c);
int iswxdigit(int c);
int towupper(int c);
int towlower(int c);
"#;

/// `tgmath.h` — type-generic math (C99).
pub const HEADER_TGMATH: &str = r#"
/* tgmath.h — type-generic, maps to math.h for doubles */
#define sin(x)  sin(x)
#define cos(x)  cos(x)
#define tan(x)  tan(x)
#define sqrt(x) sqrt(x)
#define pow(x,y) pow(x,y)
#define fabs(x) fabs(x)
#define exp(x)  exp(x)
#define log(x)  log(x)
#define ceil(x) ceil(x)
#define floor(x) floor(x)
#define round(x) round(x)
"#;
