#[derive(Debug, Clone)]
pub struct CpuFeatures {
    pub has_avx2: bool,
    pub has_sse42: bool,
    pub has_bmi2: bool,
    pub brand: String,
}

pub fn detect_cpu_features() -> CpuFeatures {
    #[cfg(target_arch = "x86_64")]
    {
        let has_avx2;
        let has_sse42;
        let has_bmi2;
        let ebx_feat1: u32;
        let ebx_feat7: u32;
        unsafe {
            // CPUID EAX=1 — must save/restore rbx (LLVM reserved)
            let mut eax_out: u32;
            let mut ebx_out: u32;
            let mut ecx_out: u32;
            let mut edx_out: u32;
            std::arch::asm!(
                "push rbx",
                "cpuid",
                "mov {ebx_out:e}, ebx",
                "pop rbx",
                inout("eax") 1u32 => eax_out,
                ebx_out = out(reg) ebx_out,
                out("ecx") ecx_out,
                out("edx") edx_out,
            );
            has_sse42 = (ecx_out & (1 << 20)) != 0;
            ebx_feat1 = ebx_out;

            // CPUID EAX=7, ECX=0 — AVX2/BMI2 in EBX
            std::arch::asm!(
                "push rbx",
                "cpuid",
                "mov {ebx_out:e}, ebx",
                "pop rbx",
                inout("eax") 7u32 => eax_out,
                ebx_out = out(reg) ebx_out,
                inout("ecx") 0u32 => ecx_out,
                out("edx") edx_out,
            );
            has_avx2 = (ebx_out & (1 << 5)) != 0;
            has_bmi2 = (ebx_out & (1 << 8)) != 0;
            ebx_feat7 = ebx_out;
        }

        // Get CPU brand string via CPUID 0x80000002-0x80000004
        let mut brand_bytes = [0u8; 48];
        unsafe {
            for i in 0u32..3 {
                let mut eax_out: u32;
                let mut ebx_out: u32;
                let mut ecx_out: u32;
                let mut edx_out: u32;
                std::arch::asm!(
                    "push rbx",
                    "cpuid",
                    "mov {ebx_out:e}, ebx",
                    "pop rbx",
                    inout("eax") (0x80000002u32 + i) => eax_out,
                    ebx_out = out(reg) ebx_out,
                    out("ecx") ecx_out,
                    out("edx") edx_out,
                );
                let off = (i as usize) * 16;
                brand_bytes[off..off+4].copy_from_slice(&eax_out.to_le_bytes());
                brand_bytes[off+4..off+8].copy_from_slice(&ebx_out.to_le_bytes());
                brand_bytes[off+8..off+12].copy_from_slice(&ecx_out.to_le_bytes());
                brand_bytes[off+12..off+16].copy_from_slice(&edx_out.to_le_bytes());
            }
        }
        let brand = String::from_utf8_lossy(&brand_bytes)
            .trim_end_matches('\0')
            .trim()
            .to_string();

        let _ = (ebx_feat1, ebx_feat7); // suppress unused warnings
        CpuFeatures { has_avx2, has_sse42, has_bmi2, brand }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        CpuFeatures { has_avx2: false, has_sse42: false, has_bmi2: false, brand: "unknown".to_string() }
    }
}

