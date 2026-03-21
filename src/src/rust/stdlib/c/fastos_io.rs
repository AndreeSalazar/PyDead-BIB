// ============================================================
// fastos_io.rs — I/O y Control de Hardware x86-64
// ============================================================
// Instrucciones de bajo nivel que el kernel FastOS usa:
// puertos I/O, control de CPU, registros especiales.
//
// El compilador reconoce estas funciones como built-in cuando
// compilando para el target fastos, evitando undefined symbols
// en archivos como interrupts.c y hotplug.c.
// ============================================================

// ─── Funciones de Puertos I/O ────────────────────────────────
pub const IO_PORT_FNS: &[(&str, &str)] = &[
    ("inb",  "unsigned char inb(unsigned short port)"),
    ("inw",  "unsigned short inw(unsigned short port)"),
    ("inl",  "unsigned int inl(unsigned short port)"),
    ("outb", "void outb(unsigned short port, unsigned char val)"),
    ("outw", "void outw(unsigned short port, unsigned short val)"),
    ("outl", "void outl(unsigned short port, unsigned int val)"),
    ("io_wait", "void io_wait(void)"),
    ("inb_delay","void inb_delay(unsigned short port, unsigned char *val)"),
];

// ─── Control de CPU ──────────────────────────────────────────
pub const CPU_CONTROL_FNS: &[(&str, &str)] = &[
    ("cli",    "void cli(void)"),
    ("sti",    "void sti(void)"),
    ("hlt",    "void hlt(void)"),
    ("nop",    "void nop(void)"),
    ("pause",  "void pause(void)"),
    ("cpuid",  "void cpuid(unsigned int leaf, unsigned int *eax, unsigned int *ebx, unsigned int *ecx, unsigned int *edx)"),
    ("rdtsc",  "unsigned long long rdtsc(void)"),
    ("rdmsr",  "unsigned long long rdmsr(unsigned int msr)"),
    ("wrmsr",  "void wrmsr(unsigned int msr, unsigned long long val)"),
];

// ─── Registros de Control ────────────────────────────────────
pub const CPU_REGISTER_FNS: &[(&str, &str)] = &[
    ("read_cr0",  "unsigned long long read_cr0(void)"),
    ("write_cr0", "void write_cr0(unsigned long long val)"),
    ("read_cr2",  "unsigned long long read_cr2(void)"),
    ("read_cr3",  "unsigned long long read_cr3(void)"),
    ("write_cr3", "void write_cr3(unsigned long long val)"),
    ("read_cr4",  "unsigned long long read_cr4(void)"),
    ("write_cr4", "void write_cr4(unsigned long long val)"),
    ("read_rflags","unsigned long long read_rflags(void)"),
    ("read_rsp",  "unsigned long long read_rsp(void)"),
    ("read_rbp",  "unsigned long long read_rbp(void)"),
];

// ─── Descriptores (GDT/IDT) ──────────────────────────────────
pub const CPU_DESCRIPTOR_FNS: &[(&str, &str)] = &[
    ("lgdt",     "void lgdt(void *gdt_ptr)"),
    ("lidt",     "void lidt(void *idt_ptr)"),
    ("sgdt",     "void sgdt(void *gdt_ptr)"),
    ("sidt",     "void sidt(void *idt_ptr)"),
    ("ltr",      "void ltr(unsigned short sel)"),
    ("flush_tss","void flush_tss(void)"),
    ("invlpg",   "void invlpg(unsigned long long addr)"),
    ("flush_tlb","void flush_tlb(void)"),
];

// ─── Constantes de Hardware ──────────────────────────────────
pub const HW_CONSTANTS: &[(&str, &str)] = &[
    // PIC
    ("PIC1_CMD",     "0x0020  /* Master PIC command port */"),
    ("PIC1_DATA",    "0x0021  /* Master PIC data port */"),
    ("PIC2_CMD",     "0x00A0  /* Slave PIC command port */"),
    ("PIC2_DATA",    "0x00A1  /* Slave PIC data port */"),
    ("PIC_EOI",      "0x0020  /* End of Interrupt */"),
    ("PIC_ICW1_ICW4","0x01"),
    ("PIC_ICW1_INIT","0x10"),
    ("PIC_ICW4_8086","0x01"),
    // PIT
    ("PIT_CH0",      "0x0040  /* PIT Channel 0 */"),
    ("PIT_CMD",      "0x0043  /* PIT Command port */"),
    ("PIT_FREQ",     "1193182 /* PIT base frequency Hz */"),
    // VGA
    ("VGA_CRTC_ADDR","0x03D4"),
    ("VGA_CRTC_DATA","0x03D5"),
    // PS/2
    ("PS2_DATA",     "0x0060"),
    ("PS2_STATUS",   "0x0064"),
    ("PS2_CMD",      "0x0064"),
    // CMOS/RTC
    ("CMOS_ADDR",    "0x0070"),
    ("CMOS_DATA",    "0x0071"),
];

pub fn all_io_symbols() -> Vec<(&'static str, &'static str)> {
    let mut v = Vec::new();
    for &s in IO_PORT_FNS       { v.push(s); }
    for &s in CPU_CONTROL_FNS   { v.push(s); }
    for &s in CPU_REGISTER_FNS  { v.push(s); }
    for &s in CPU_DESCRIPTOR_FNS { v.push(s); }
    v
}

pub fn is_io_symbol(name: &str) -> bool {
    all_io_symbols().iter().any(|(n, _)| *n == name)
        || HW_CONSTANTS.iter().any(|(n, _)| *n == name)
}

/// Genera el contenido inline de las funciones I/O usando asm volatile.
/// ADead-BIB inyecta estas definiciones cuando compila para --target fastos.
pub fn generate_io_h() -> String {
    let mut out = String::from("/* fastos_io.h — Generado internamente por ADead-BIB */\n");
    out.push_str("#ifndef _FASTOS_IO_H\n#define _FASTOS_IO_H\n\n");

    out.push_str("/* Puerto I/O lectura */\n");
    out.push_str("static inline unsigned char inb(unsigned short port) {\n");
    out.push_str("  unsigned char val;\n");
    out.push_str("  asm volatile(\"inb %1, %0\" : \"=a\"(val) : \"Nd\"(port));\n");
    out.push_str("  return val;\n}\n\n");

    out.push_str("static inline unsigned short inw(unsigned short port) {\n");
    out.push_str("  unsigned short val;\n");
    out.push_str("  asm volatile(\"inw %1, %0\" : \"=a\"(val) : \"Nd\"(port));\n");
    out.push_str("  return val;\n}\n\n");

    out.push_str("static inline unsigned int inl(unsigned short port) {\n");
    out.push_str("  unsigned int val;\n");
    out.push_str("  asm volatile(\"inl %1, %0\" : \"=a\"(val) : \"Nd\"(port));\n");
    out.push_str("  return val;\n}\n\n");

    out.push_str("/* Puerto I/O escritura */\n");
    out.push_str("static inline void outb(unsigned short port, unsigned char val) {\n");
    out.push_str("  asm volatile(\"outb %0, %1\" : : \"a\"(val), \"Nd\"(port));\n}\n\n");
    out.push_str("static inline void outw(unsigned short port, unsigned short val) {\n");
    out.push_str("  asm volatile(\"outw %0, %1\" : : \"a\"(val), \"Nd\"(port));\n}\n\n");
    out.push_str("static inline void outl(unsigned short port, unsigned int val) {\n");
    out.push_str("  asm volatile(\"outl %0, %1\" : : \"a\"(val), \"Nd\"(port));\n}\n\n");

    out.push_str("/* Control CPU */\n");
    out.push_str("static inline void cli(void) { asm volatile(\"cli\" ::: \"memory\"); }\n");
    out.push_str("static inline void sti(void) { asm volatile(\"sti\" ::: \"memory\"); }\n");
    out.push_str("static inline void hlt(void) { asm volatile(\"hlt\" ::: \"memory\"); }\n");
    out.push_str("static inline void nop(void) { asm volatile(\"nop\"); }\n\n");

    out.push_str("/* Registros de control */\n");
    out.push_str("static inline unsigned long long read_cr0(void) {\n");
    out.push_str("  unsigned long long v; asm volatile(\"mov %%cr0,%0\":\"=r\"(v)); return v;\n}\n");
    out.push_str("static inline void write_cr0(unsigned long long v) {\n");
    out.push_str("  asm volatile(\"mov %0,%%cr0\"::\"+r\"(v):\"memory\");\n}\n");
    out.push_str("static inline unsigned long long read_cr2(void) {\n");
    out.push_str("  unsigned long long v; asm volatile(\"mov %%cr2,%0\":\"=r\"(v)); return v;\n}\n");
    out.push_str("static inline unsigned long long read_cr3(void) {\n");
    out.push_str("  unsigned long long v; asm volatile(\"mov %%cr3,%0\":\"=r\"(v)); return v;\n}\n");
    out.push_str("static inline void write_cr3(unsigned long long v) {\n");
    out.push_str("  asm volatile(\"mov %0,%%cr3\"::\"+r\"(v):\"memory\");\n}\n\n");

    out.push_str("/* Constantes de hardware */\n");
    out.push_str("#define PIC1_CMD  0x20\n#define PIC1_DATA 0x21\n");
    out.push_str("#define PIC2_CMD  0xA0\n#define PIC2_DATA 0xA1\n");
    out.push_str("#define PIC_EOI   0x20\n");
    out.push_str("#define PIT_CH0   0x40\n#define PIT_CMD   0x43\n#define PIT_FREQ  1193182\n");
    out.push_str("#define PS2_DATA  0x60\n#define PS2_CMD   0x64\n");

    out.push_str("\n#endif /* _FASTOS_IO_H */\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_symbol_recognition() {
        assert!(is_io_symbol("inb"));
        assert!(is_io_symbol("outb"));
        assert!(is_io_symbol("cli"));
        assert!(is_io_symbol("read_cr3"));
        assert!(is_io_symbol("write_cr3"));
        assert!(is_io_symbol("lgdt"));
        assert!(!is_io_symbol("printf"));
    }

    #[test]
    fn test_generate_io_h() {
        let h = generate_io_h();
        assert!(h.contains("asm volatile"));
        assert!(h.contains("inb"));
        assert!(h.contains("outb"));
        assert!(h.contains("read_cr3"));
        assert!(h.contains("PIC1_CMD"));
    }

    #[test]
    fn test_hw_constants_count() {
        assert!(HW_CONSTANTS.len() >= 10);
    }
}
