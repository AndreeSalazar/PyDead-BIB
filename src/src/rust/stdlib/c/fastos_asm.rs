// ============================================================
// fastos_asm.rs — Soporte de __builtin_* y asm volatile
// ============================================================
// ADead-BIB reconoce como válidos los __builtin_* de GCC/Clang
// y los atributos __attribute__((...)). Sin esto, el compilador
// no puede parsear: kprintf (__builtin_va_list), __packed,
// __noreturn, asm volatile("hlt"), etc.
//
// Cuando el compilador encuentra estos tokens, los mapea
// a equivalentes internos en lugar de fallar.
// ============================================================

// ─── __builtin_* de GCC/Clang reconocidos ──────────────────
pub const BUILTINS: &[(&str, &str, &str)] = &[
    // (builtin_name, tipo_retorno, descripcion)
    ("__builtin_va_list",       "type",   "va_list — argumento variable lista"),
    ("__builtin_va_start",      "macro",  "__builtin_va_start(ap, last) → inicia va_list"),
    ("__builtin_va_arg",        "macro",  "__builtin_va_arg(ap, type) → extrae argumento"),
    ("__builtin_va_end",        "macro",  "__builtin_va_end(ap) → termina va_list"),
    ("__builtin_va_copy",       "macro",  "__builtin_va_copy(dest, src) → copia va_list"),
    ("__builtin_offsetof",      "macro",  "offsetof(type, member) → offset de campo"),
    ("__builtin_expect",        "fn",     "__builtin_expect(expr, val) → branch hint"),
    ("__builtin_unreachable",   "fn",     "__builtin_unreachable() → unreachable hint"),
    ("__builtin_trap",          "fn",     "__builtin_trap() → genera trap/int3"),
    ("__builtin_prefetch",      "fn",     "__builtin_prefetch(addr, rw, locality)"),
    ("__builtin_clz",           "fn",     "count leading zeros (32-bit)"),
    ("__builtin_clzll",         "fn",     "count leading zeros (64-bit)"),
    ("__builtin_ctz",           "fn",     "count trailing zeros (32-bit)"),
    ("__builtin_ctzll",         "fn",     "count trailing zeros (64-bit)"),
    ("__builtin_popcount",      "fn",     "population count (32-bit)"),
    ("__builtin_popcountll",    "fn",     "population count (64-bit)"),
    ("__builtin_bswap16",       "fn",     "byte swap 16-bit"),
    ("__builtin_bswap32",       "fn",     "byte swap 32-bit"),
    ("__builtin_bswap64",       "fn",     "byte swap 64-bit"),
    ("__builtin_memcpy",        "fn",     "memcpy intrinsic"),
    ("__builtin_memset",        "fn",     "memset intrinsic"),
    ("__builtin_memcmp",        "fn",     "memcmp intrinsic"),
    ("__builtin_strlen",        "fn",     "strlen intrinsic"),
];

// ─── __attribute__((...)) reconocidos ────────────────────────
pub const GCC_ATTRIBUTES: &[(&str, &str)] = &[
    ("noreturn",        "Función que nunca retorna (panic, halt)"),
    ("packed",          "Estructura sin padding de alineamiento"),
    ("aligned",         "__attribute__((aligned(N))) — alineamiento forzado"),
    ("unused",          "Variable/parámetro no usado — suprime warning"),
    ("used",            "Símbolo siempre emitido aunque parezca no usarse"),
    ("noinline",        "Nunca inline esta función"),
    ("always_inline",   "Siempre inline esta función"),
    ("cold",            "Código poco frecuente (panic, error paths)"),
    ("hot",             "Código frecuente (scheduler_tick, irq handlers)"),
    ("section",         "__attribute__((section(\".text.init\"))) — sección ELF"),
    ("visibility",      "__attribute__((visibility(\"hidden\")))"),
    ("constructor",     "Ejecutado antes de main()"),
    ("destructor",      "Ejecutado después de main()"),
    ("interrupt",       "ISR — guarda/restaura contexto completo"),
    ("naked",           "Sin prólogo/epílogo (para context switch ASM)"),
    ("format",          "__attribute__((format(printf,N,M))) — format checking"),
    ("warn_unused_result","Warn si no se usa el valor de retorno"),
    ("nonnull",         "Parámetros que no pueden ser NULL"),
    ("malloc",          "El retorno apunta a memoria recién asignada"),
    ("pure",            "Sin side effects, resultado depends solo de args"),
    ("const",           "Como pure pero tampoco lee memoria global"),
    ("deprecated",      "Símbolo deprecated — warning al usarlo"),
    ("alias",           "Alias de otro símbolo"),
    ("weak",            "Símbolo débil — sobreescribible"),
    ("fastcall",        "ABI fastcall (args en regs)"),
    ("stdcall",         "ABI stdcall (Windows calling convention)"),
    ("cdecl",           "ABI cdecl (C default)"),
    ("regparm",         "__attribute__((regparm(N))) — args en registros"),
    ("vector_size",     "Tipo SIMD — __attribute__((vector_size(16)))"),
    ("transparent_union","Union transparente — cualquier miembro compatible"),
    ("cleanup",         "__attribute__((cleanup(fn))) — destructor en scope exit"),
    ("warn_if_not_aligned","Warning si no está alineado"),
    ("retain",          "Retener el símbolo incluso con --gc-sections"),
];

// ─── Alias de tipos especiales del compilador ────────────────
pub const COMPILER_TYPES: &[(&str, &str)] = &[
    ("__builtin_va_list", "struct __va_list_tag[1]"),
    ("va_list",           "__builtin_va_list"),
    ("__gnuc_va_list",    "__builtin_va_list"),
    ("__int128",          "128-bit integer (extensión GCC)"),
    ("__int128_t",        "signed __int128"),
    ("__uint128_t",       "unsigned __int128"),
    ("__m128",            "SSE 128-bit float vector"),
    ("__m256",            "AVX 256-bit float vector"),
    ("__m512",            "AVX-512 float vector"),
    ("__SIZE_TYPE__",     "unsigned long long"),
    ("__PTRDIFF_TYPE__",  "long long"),
    ("__UINTPTR_TYPE__",  "unsigned long long"),
    ("__INTPTR_TYPE__",   "long long"),
];

// ─── Macros de compatibilidad ────────────────────────────────
pub const COMPAT_MACROS: &[(&str, &str)] = &[
    ("__packed",       "__attribute__((packed))"),
    ("__noreturn",     "__attribute__((noreturn))"),
    ("__unused",       "__attribute__((unused))"),
    ("__always_inline","__attribute__((always_inline)) inline"),
    ("__noinline",     "__attribute__((noinline))"),
    ("__cold",         "__attribute__((cold))"),
    ("__hot",          "__attribute__((hot))"),
    ("__naked",        "__attribute__((naked))"),
    ("likely",         "__builtin_expect(!!(x), 1)"),
    ("unlikely",       "__builtin_expect(!!(x), 0)"),
    ("barrier",        "asm volatile(\"\" ::: \"memory\")"),
    ("mb",             "asm volatile(\"mfence\" ::: \"memory\")  /* memory barrier */"),
    ("rmb",            "asm volatile(\"lfence\" ::: \"memory\")  /* read barrier */"),
    ("wmb",            "asm volatile(\"sfence\" ::: \"memory\")  /* write barrier */"),
    ("ARRAY_SIZE",     "(sizeof(arr)/sizeof((arr)[0]))"),
    ("container_of",   "((type*)((char*)(ptr) - offsetof(type, member)))"),
    ("BIT",            "(1ULL << (n))"),
    ("GENMASK",        "(((1ULL << ((h)-(l)+1))-1) << (l))"),
    ("DIV_ROUND_UP",   "((n + d - 1) / d)"),
    ("min",            "((a)<(b)?(a):(b))"),
    ("max",            "((a)>(b)?(a):(b))"),
    ("clamp",          "((v)<(lo)?(lo):(v)>(hi)?(hi):(v))"),
    ("swap",           "do{ __typeof__(a) _t=(a);(a)=(b);(b)=_t; }while(0)"),
];

pub fn is_builtin(name: &str) -> bool {
    BUILTINS.iter().any(|(n, _, _)| *n == name)
        || COMPILER_TYPES.iter().any(|(n, _)| *n == name)
}

pub fn is_gcc_attribute(name: &str) -> bool {
    GCC_ATTRIBUTES.iter().any(|(n, _)| *n == name)
}

pub fn is_compat_macro(name: &str) -> bool {
    COMPAT_MACROS.iter().any(|(n, _)| *n == name)
}

/// Genera el header de compatibilidad que ADead-BIB inyecta
/// automáticamente en cada translation unit de kernel FastOS.
pub fn generate_asm_compat_h() -> String {
    let mut out = String::from("/* fastos_asm_compat.h — Generado por ADead-BIB */\n");
    out.push_str("#ifndef _FASTOS_ASM_COMPAT_H\n#define _FASTOS_ASM_COMPAT_H\n\n");

    out.push_str("/* Atributos GCC/Clang como macros simples */\n");
    for (macro_name, expansion) in COMPAT_MACROS {
        // Solo añadir los que son un __attribute__ directo
        if expansion.starts_with("__attribute__") {
            out.push_str(&format!("#ifndef {}\n", macro_name));
            out.push_str(&format!("#define {} {}\n", macro_name, expansion));
            out.push_str("#endif\n");
        }
    }

    out.push_str("\n/* va_list para kprintf sin libc */\n");
    out.push_str("typedef __builtin_va_list va_list;\n");
    out.push_str("#define va_start(ap, last)  __builtin_va_start((ap), (last))\n");
    out.push_str("#define va_arg(ap, type)    __builtin_va_arg((ap), type)\n");
    out.push_str("#define va_end(ap)          __builtin_va_end(ap)\n");
    out.push_str("#define va_copy(d,s)        __builtin_va_copy((d),(s))\n");

    out.push_str("\n/* Branch prediction hints */\n");
    out.push_str("#define likely(x)   __builtin_expect(!!(x), 1)\n");
    out.push_str("#define unlikely(x) __builtin_expect(!!(x), 0)\n");

    out.push_str("\n/* Memory barriers */\n");
    out.push_str("#define barrier() asm volatile(\"\" ::: \"memory\")\n");
    out.push_str("#define mb()      asm volatile(\"mfence\" ::: \"memory\")\n");
    out.push_str("#define rmb()     asm volatile(\"lfence\" ::: \"memory\")\n");
    out.push_str("#define wmb()     asm volatile(\"sfence\" ::: \"memory\")\n");

    out.push_str("\n/* Utility macros */\n");
    out.push_str("#define ARRAY_SIZE(a)  (sizeof(a)/sizeof((a)[0]))\n");
    out.push_str("#define BIT(n)         (1ULL << (n))\n");
    out.push_str("#define DIV_ROUND_UP(n,d) (((n)+(d)-1)/(d))\n");

    out.push_str("\n#endif /* _FASTOS_ASM_COMPAT_H */\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_recognition() {
        assert!(is_builtin("__builtin_va_list"));
        assert!(is_builtin("__builtin_va_start"));
        assert!(is_builtin("__builtin_va_arg"));
        assert!(is_builtin("__builtin_va_end"));
        assert!(is_builtin("__builtin_expect"));
        assert!(is_builtin("__builtin_offsetof"));
        assert!(!is_builtin("printf"));
    }

    #[test]
    fn test_gcc_attribute_recognition() {
        assert!(is_gcc_attribute("noreturn"));
        assert!(is_gcc_attribute("packed"));
        assert!(is_gcc_attribute("interrupt"));
        assert!(is_gcc_attribute("naked"));
        assert!(!is_gcc_attribute("static"));
    }

    #[test]
    fn test_compat_macros() {
        assert!(is_compat_macro("__packed"));
        assert!(is_compat_macro("__noreturn"));
        assert!(is_compat_macro("likely"));
        assert!(is_compat_macro("ARRAY_SIZE"));
    }

    #[test]
    fn test_generate_asm_compat_h() {
        let h = generate_asm_compat_h();
        assert!(h.contains("va_list"));
        assert!(h.contains("va_start"));
        assert!(h.contains("__builtin_va_start"));
        assert!(h.contains("likely"));
        assert!(h.contains("barrier"));
    }
}
