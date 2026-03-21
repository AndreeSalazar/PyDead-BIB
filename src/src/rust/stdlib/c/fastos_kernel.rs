// ============================================================
// fastos_kernel.rs — FastOS Kernel API Interna
// ============================================================
// Toda la API del kernel FastOS como stdlib interna de ADead-BIB.
// Cuando el compilador ve #include <kernel.h>, #include <fastos.h>
// o #include "../include/kernel.h", sirve estos símbolos.
//
// Sin GCC. Sin flags. Sin libc externa.
// El compilador conoce FastOS natively.
// ============================================================

// ─── Funciones VGA / Output ───────────────────────────────────
pub const KERNEL_OUTPUT_FNS: &[(&str, &str)] = &[
    ("term_init",        "void term_init(void)"),
    ("term_putchar",     "void term_putchar(char c)"),
    ("term_write",       "void term_write(const char *str)"),
    ("term_write_color", "void term_write_color(const char *str, unsigned char color)"),
    ("vga_putchar",      "void vga_putchar(char c)"),
    ("kprintf",          "void kprintf(const char *fmt, ...)"),
    ("kputs",            "void kputs(const char *s)"),
    ("heap_dump",        "void heap_dump(void)"),
];

// ─── Funciones de Memoria ─────────────────────────────────────
pub const KERNEL_MEMORY_FNS: &[(&str, &str)] = &[
    ("kmalloc",   "void *kmalloc(unsigned long long size)"),
    ("kzalloc",   "void *kzalloc(unsigned long long size)"),
    ("krealloc",  "void *krealloc(void *ptr, unsigned long long size)"),
    ("kfree",     "void kfree(void *ptr)"),
    ("kmemcpy",   "void *kmemcpy(void *dest, const void *src, unsigned long long n)"),
    ("kmemset",   "void *kmemset(void *s, int c, unsigned long long n)"),
    ("kmemcmp",   "int kmemcmp(const void *s1, const void *s2, unsigned long long n)"),
    ("kstrlen",   "unsigned long long kstrlen(const char *s)"),
    ("kstrcpy",   "char *kstrcpy(char *dest, const char *src)"),
    ("kstrncpy",  "char *kstrncpy(char *dest, const char *src, unsigned long long n)"),
    ("kstrcmp",   "int kstrcmp(const char *s1, const char *s2)"),
    ("kstrncmp",  "int kstrncmp(const char *s1, const char *s2, unsigned long long n)"),
    ("kstrcat",   "char *kstrcat(char *dest, const char *src)"),
    ("memory_init",    "void memory_init(void)"),
    ("memory_map_init","void memory_map_init(void)"),
];

// ─── Funciones del Scheduler ──────────────────────────────────
pub const KERNEL_SCHEDULER_FNS: &[(&str, &str)] = &[
    ("scheduler_init",    "void scheduler_init(void)"),
    ("scheduler_tick",    "void scheduler_tick(void)"),
    ("scheduler_list",    "void scheduler_list(void)"),
    ("process_create",    "int process_create(const char *name, void (*entry)(void), unsigned char security_level)"),
    ("process_exit",      "void process_exit(int code)"),
    ("process_block",     "void process_block(void)"),
    ("process_unblock",   "void process_unblock(unsigned int pid)"),
    ("yield",             "void yield(void)"),
    ("get_current_process", "void *get_current_process(void)"),
    ("get_process",       "void *get_process(unsigned int pid)"),
];

// ─── Funciones de Interrupciones ──────────────────────────────
pub const KERNEL_INTERRUPT_FNS: &[(&str, &str)] = &[
    ("interrupts_init",    "void interrupts_init(void)"),
    ("interrupts_enable",  "void interrupts_enable(void)"),
    ("interrupts_disable", "void interrupts_disable(void)"),
    ("idt_init",           "void idt_init(void)"),
];

// ─── Funciones del Kernel Panic ───────────────────────────────
pub const KERNEL_PANIC_FNS: &[(&str, &str)] = &[
    ("kernel_panic",       "void kernel_panic(unsigned int code, const char *message, const char *file, int line)"),
    ("kernel_assert_fail", "void kernel_assert_fail(const char *expr, const char *file, int line)"),
];

// ─── Binary Guardian (BG) ─────────────────────────────────────
pub const KERNEL_BG_FNS: &[(&str, &str)] = &[
    ("bg_init",                "void bg_init(void)"),
    ("bg_verify_binary",       "int bg_verify_binary(const unsigned char *binary, unsigned long long size, unsigned int caps)"),
    ("bg_get_violations",      "unsigned int bg_get_violations(void)"),
    ("bg_get_verified",        "unsigned int bg_get_verified(void)"),
    ("bg_level1_rebuild_check","int bg_level1_rebuild_check(const char *path, unsigned long long expected_hash)"),
    ("bg_level2_capability_check","int bg_level2_capability_check(unsigned int pid, unsigned int req, unsigned int allowed)"),
    ("bg_level3_preexec",      "int bg_level3_preexec(const unsigned char *binary, unsigned long long size, unsigned int caps)"),
    ("bg_level4_heartbeat",    "void bg_level4_heartbeat(void)"),
    ("bg_level4_integrity_check","int bg_level4_integrity_check(void)"),
    ("bg_preexec_gate",        "int bg_preexec_gate(const unsigned char *binary, unsigned long long size, unsigned int caps, unsigned int pid)"),
    ("bg_preexec_invalidate",  "void bg_preexec_invalidate(unsigned long long hash)"),
    ("bg_preexec_cache_hits",  "unsigned int bg_preexec_cache_hits(void)"),
];

// ─── Hotplug ──────────────────────────────────────────────────
pub const KERNEL_HOTPLUG_FNS: &[(&str, &str)] = &[
    ("hotplug_init",         "void hotplug_init(void)"),
    ("hotplug_tick",         "void hotplug_tick(void)"),
    ("hotplug_on_pci_device","void hotplug_on_pci_device(unsigned short vendor, unsigned short device, unsigned char bus, unsigned char slot, unsigned char func)"),
];

// ─── Init / Shell ─────────────────────────────────────────────
pub const KERNEL_USERSPACE_FNS: &[(&str, &str)] = &[
    ("init_main",    "void init_main(void)"),
    ("shell_start",  "void shell_start(void)"),
    ("vfs_init",     "void vfs_init(void)"),
    ("kernel_main",  "void kernel_main(void)"),
];

// ─── Macros del Kernel ────────────────────────────────────────
pub const KERNEL_MACROS: &[(&str, &str)] = &[
    ("KERNEL_PANIC",    "KERNEL_PANIC(code, msg)  → kernel_panic(code, msg, __FILE__, __LINE__)"),
    ("KERNEL_ASSERT",   "KERNEL_ASSERT(expr)       → assert with file+line"),
    ("VGA_COLOR",       "VGA_COLOR(fg, bg)         → (uint8_t)((bg<<4)|fg)"),
    ("ALIGN_UP",        "ALIGN_UP(x, a)            → ((x+(a-1))&~(a-1))"),
    ("ALIGN_DOWN",      "ALIGN_DOWN(x, a)          → (x&~(a-1))"),
    ("BG_HW_NONE",      "BG_HW_NONE                → 0x00000000"),
    ("BG_HW_MEMORY",    "BG_HW_MEMORY              → 0x00000001"),
    ("BG_HW_PORTS",     "BG_HW_PORTS               → 0x00000002"),
    ("BG_CAP_NONE",     "BG_CAP_NONE               → 0x00000000"),
    ("BG_CAP_ALL",      "BG_CAP_ALL                → 0xFFFFFFFF"),
    ("MAX_PROCESSES",   "MAX_PROCESSES             → 64"),
    ("KERNEL_STACK_SIZE","KERNEL_STACK_SIZE         → 4096"),
    ("USER_STACK_SIZE", "USER_STACK_SIZE           → 65536"),
    ("VGA_BUFFER",      "VGA_BUFFER                → ((volatile uint16_t*)0xB8000)"),
    ("VGA_WIDTH",       "VGA_WIDTH                 → 80"),
    ("VGA_HEIGHT",      "VGA_HEIGHT                → 25"),
    ("FASTOS_PO_MAGIC", "FASTOS_PO_MAGIC           → \"FASTOS\""),
    ("FASTOS_PO_HDRSIZE","FASTOS_PO_HDRSIZE         → 24"),
];

// ─── Tipos del Kernel ─────────────────────────────────────────
pub const KERNEL_TYPES: &[(&str, &str)] = &[
    ("vga_color_t",      "enum: VGA_BLACK=0 .. VGA_WHITE=15"),
    ("proc_state_t",     "enum: PROC_UNUSED=0, PROC_READY, PROC_RUNNING, PROC_BLOCKED, PROC_ZOMBIE"),
    ("cpu_context_t",    "struct: rax,rbx,...,r15,rip,rflags,cs,ss,cr3"),
    ("process_t",        "struct: pid,ppid,state,priority,security_level,context,kernel_stack,user_stack,page_table,time_slice,total_time,name[32]"),
    ("bg_result_t",      "enum: BG_RESULT_OK=0 .. BG_RESULT_INTEGRITY_FAILURE=9"),
    ("bg_capability_t",  "typedef uint32_t"),
    ("bg_level_t",       "enum: BG_LEVEL_1..BG_LEVEL_MAX"),
    ("bg_state_t",       "struct: initialized, level, violations, verified"),
    ("fastos_po_header_t","struct: magic[6],version,code_offset,code_size,data_offset,data_size"),
    ("idt_entry_t",      "struct: offset_low,selector,ist,flags,offset_mid,offset_high,reserved"),
    ("idt_ptr_t",        "struct: limit,base"),
];

// ─── Lista plana de todos los símbolos conocidos ───────────────

pub fn all_kernel_symbols() -> Vec<(&'static str, &'static str)> {
    let mut symbols = Vec::new();
    for &s in KERNEL_OUTPUT_FNS   { symbols.push(s); }
    for &s in KERNEL_MEMORY_FNS   { symbols.push(s); }
    for &s in KERNEL_SCHEDULER_FNS { symbols.push(s); }
    for &s in KERNEL_INTERRUPT_FNS { symbols.push(s); }
    for &s in KERNEL_PANIC_FNS    { symbols.push(s); }
    for &s in KERNEL_BG_FNS       { symbols.push(s); }
    for &s in KERNEL_HOTPLUG_FNS  { symbols.push(s); }
    for &s in KERNEL_USERSPACE_FNS { symbols.push(s); }
    for &s in KERNEL_MACROS       { symbols.push(s); }
    symbols
}

pub fn is_kernel_symbol(name: &str) -> bool {
    all_kernel_symbols().iter().any(|(n, _)| *n == name)
        || KERNEL_TYPES.iter().any(|(n, _)| *n == name)
}

/// Genera el contenido C del header kernel.h para inyectarlo en el
/// preprocessor cuando se encuentre cualquier variante de kernel.h
pub fn generate_kernel_h() -> String {
    let mut out = String::from("/* kernel.h — Generado internamente por ADead-BIB FastOS stdlib */\n");
    out.push_str("#ifndef _FASTOS_KERNEL_H\n#define _FASTOS_KERNEL_H\n\n");

    // Types
    out.push_str("/* --- Tipos basicos --- */\n");
    out.push_str("typedef unsigned char  uint8_t;\n");
    out.push_str("typedef unsigned short uint16_t;\n");
    out.push_str("typedef unsigned int   uint32_t;\n");
    out.push_str("typedef unsigned long long uint64_t;\n");
    out.push_str("typedef signed char    int8_t;\n");
    out.push_str("typedef short          int16_t;\n");
    out.push_str("typedef int            int32_t;\n");
    out.push_str("typedef long long      int64_t;\n");
    out.push_str("typedef unsigned long long size_t;\n");
    out.push_str("typedef long long      ssize_t;\n");

    // VGA
    out.push_str("\n/* --- VGA Colors --- */\n");
    out.push_str("typedef enum { VGA_BLACK=0,VGA_BLUE=1,VGA_GREEN=2,VGA_CYAN=3,VGA_RED=4,\n");
    out.push_str("  VGA_MAGENTA=5,VGA_BROWN=6,VGA_LGRAY=7,VGA_DGRAY=8,VGA_LBLUE=9,\n");
    out.push_str("  VGA_LGREEN=10,VGA_LCYAN=11,VGA_LRED=12,VGA_LMAGENTA=13,\n");
    out.push_str("  VGA_YELLOW=14,VGA_WHITE=15 } vga_color_t;\n");
    out.push_str("#define VGA_COLOR(fg,bg) ((uint8_t)(((bg)<<4)|((fg)&0xF)))\n");
    out.push_str("#define VGA_BUFFER  ((volatile uint16_t*)0xB8000)\n");
    out.push_str("#define VGA_WIDTH   80\n");
    out.push_str("#define VGA_HEIGHT  25\n");

    // Alignment
    out.push_str("\n/* --- Alignment --- */\n");
    out.push_str("#define ALIGN_UP(x,a)   (((x)+((a)-1))&~((a)-1))\n");
    out.push_str("#define ALIGN_DOWN(x,a) ((x)&~((a)-1))\n");

    // Process types
    out.push_str("\n/* --- Process --- */\n");
    out.push_str("#define MAX_PROCESSES    64\n");
    out.push_str("#define KERNEL_STACK_SIZE 4096\n");
    out.push_str("#define USER_STACK_SIZE   65536\n");
    out.push_str("typedef enum { PROC_UNUSED=0,PROC_READY,PROC_RUNNING,PROC_BLOCKED,PROC_ZOMBIE } proc_state_t;\n");
    out.push_str("typedef struct __attribute__((packed)) {\n");
    out.push_str("  uint64_t rax,rbx,rcx,rdx,rsi,rdi,rbp,rsp;\n");
    out.push_str("  uint64_t r8,r9,r10,r11,r12,r13,r14,r15;\n");
    out.push_str("  uint64_t rip,rflags,cs,ss,cr3;\n");
    out.push_str("} cpu_context_t;\n");
    out.push_str("typedef struct {\n");
    out.push_str("  uint32_t pid,ppid; proc_state_t state;\n");
    out.push_str("  uint8_t priority,security_level;\n");
    out.push_str("  cpu_context_t context;\n");
    out.push_str("  uint64_t kernel_stack,user_stack,page_table;\n");
    out.push_str("  uint64_t time_slice,total_time;\n");
    out.push_str("  char name[32];\n");
    out.push_str("} process_t;\n");

    // BG types
    out.push_str("\n/* --- Binary Guardian --- */\n");
    out.push_str("typedef uint32_t bg_capability_t;\n");
    out.push_str("#define BG_CAP_NONE    0x00000000\n");
    out.push_str("#define BG_CAP_SYSCALL 0x00000001\n");
    out.push_str("#define BG_CAP_IO_DIRECT 0x00000002\n");
    out.push_str("#define BG_CAP_DRIVER  0x00000004\n");
    out.push_str("#define BG_CAP_ALL     0xFFFFFFFF\n");
    out.push_str("typedef enum { BG_RESULT_OK=0,BG_RESULT_CORRUPT=1,BG_RESULT_INVALID_MAGIC=2,\n");
    out.push_str("  BG_RESULT_VERSION_MISMATCH=3,BG_RESULT_OVERFLOW=4,\n");
    out.push_str("  BG_RESULT_UNAUTHORIZED_SYSCALL=5,BG_RESULT_UNAUTHORIZED_IO=6,\n");
    out.push_str("  BG_RESULT_NULL_INPUT=7,BG_RESULT_NOT_INITIALIZED=8,\n");
    out.push_str("  BG_RESULT_INTEGRITY_FAILURE=9 } bg_result_t;\n");
    out.push_str("typedef enum { BG_LEVEL_1=1,BG_LEVEL_2=2,BG_LEVEL_3=3,BG_LEVEL_MAX=4 } bg_level_t;\n");

    // Panic macros
    out.push_str("\n/* --- Panic --- */\n");
    out.push_str("__attribute__((noreturn)) void kernel_panic(uint32_t code,const char *msg,const char *file,int line);\n");
    out.push_str("void kernel_assert_fail(const char *expr,const char *file,int line);\n");
    out.push_str("#define KERNEL_PANIC(code,msg) kernel_panic((code),(msg),__FILE__,__LINE__)\n");
    out.push_str("#define KERNEL_ASSERT(expr) ((expr)?(void)0:kernel_assert_fail(#expr,__FILE__,__LINE__))\n");

    // Functions
    out.push_str("\n/* --- Output --- */\n");
    for (_, sig) in KERNEL_OUTPUT_FNS { out.push_str(sig); out.push_str(";\n"); }
    out.push_str("\n/* --- Memory --- */\n");
    for (_, sig) in KERNEL_MEMORY_FNS { out.push_str(sig); out.push_str(";\n"); }
    out.push_str("\n/* --- Scheduler --- */\n");
    for (_, sig) in KERNEL_SCHEDULER_FNS { out.push_str(sig); out.push_str(";\n"); }
    out.push_str("\n/* --- Interrupts --- */\n");
    for (_, sig) in KERNEL_INTERRUPT_FNS { out.push_str(sig); out.push_str(";\n"); }
    out.push_str("\n/* --- BG --- */\n");
    for (_, sig) in KERNEL_BG_FNS { out.push_str(sig); out.push_str(";\n"); }
    out.push_str("\n/* --- Hotplug --- */\n");
    for (_, sig) in KERNEL_HOTPLUG_FNS { out.push_str(sig); out.push_str(";\n"); }
    out.push_str("\n/* --- Userspace --- */\n");
    for (_, sig) in KERNEL_USERSPACE_FNS { out.push_str(sig); out.push_str(";\n"); }

    out.push_str("\n#endif /* _FASTOS_KERNEL_H */\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_symbol_recognition() {
        assert!(is_kernel_symbol("kprintf"));
        assert!(is_kernel_symbol("kmalloc"));
        assert!(is_kernel_symbol("scheduler_init"));
        assert!(is_kernel_symbol("bg_level4_heartbeat"));
        assert!(is_kernel_symbol("process_t"));
        assert!(is_kernel_symbol("VGA_COLOR"));
        assert!(!is_kernel_symbol("printf"));   // stdio, not kernel
        assert!(!is_kernel_symbol("malloc"));   // stdlib, not kernel
    }

    #[test]
    fn test_generate_kernel_h() {
        let h = generate_kernel_h();
        assert!(h.contains("uint64_t"));
        assert!(h.contains("vga_color_t"));
        assert!(h.contains("process_t"));
        assert!(h.contains("bg_result_t"));
        assert!(h.contains("KERNEL_PANIC"));
        assert!(h.contains("kprintf"));
        assert!(h.contains("kmalloc"));
    }

    #[test]
    fn test_all_symbols_count() {
        let syms = all_kernel_symbols();
        assert!(syms.len() > 40, "Should have at least 40 kernel symbols, got {}", syms.len());
    }
}
