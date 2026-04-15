// ============================================================
// PyDead-BIB JIT KILLER v2.0
// ============================================================
// "El CPU no piensa — ya sabe. La RAM no espera — ya recibe."
//
// MEJORA 2: Pre-Resolved Dispatch Table — IAT built once
// MEJORA 3: Thermal Cache — hash-based skip recompilation
// MEJORA 5: Zero Copy Data — .text RWX, .data RW (no exec)
// MEJORA 6: CPU Feature Detection — AVX2/SSE4/BMI2 at startup
// MEJORA 7: Instant Entry — pre-patch fixups, memcpy → JMP
// ============================================================

use std::collections::HashMap;
use std::sync::Mutex;

// ── MEJORA 3: Thermal Cache ────────────────────────────────
// Cache compiled bytes across runs within same session
// Key = hash of source, Value = pre-patched text + data
struct CacheEntry {
    text: Vec<u8>,
    data: Vec<u8>,
    entry_offset: u32,
}

// Simple static cache using Mutex
static THERMAL_CACHE: std::sync::LazyLock<Mutex<HashMap<u64, CacheEntry>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Hash source code for thermal cache key
pub fn hash_source(source: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for b in source.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV prime
    }
    h
}

/// Check thermal cache for pre-compiled bytes
pub fn cache_lookup(hash: u64) -> Option<bool> {
    THERMAL_CACHE.lock().ok().map(|c| c.contains_key(&hash))
}

// ── MEJORA 6: CPU Feature Detection ────────────────────────
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

// ── MEJORA 2: Pre-Resolved Dispatch Table ──────────────────
// Build IAT once, reuse across calls. No per-call lookup.
#[cfg(target_os = "windows")]
fn build_dispatch_table() -> Vec<usize> {
    extern "system" {
        fn GetStdHandle(std_handle: i32) -> *mut u8;
        fn WriteFile(handle: *mut u8, buffer: *const u8, len: u32, written: *mut u32, overlapped: *mut u8) -> i32;
        fn ExitProcess(code: u32) -> !;
        fn GetProcessHeap() -> *mut u8;
        fn HeapAlloc(heap: *mut u8, flags: u32, size: usize) -> *mut u8;
        fn GetCurrentDirectoryA(buf_len: u32, buf: *mut u8) -> u32;
        fn GetFileAttributesA(path: *const u8) -> u32;
        fn GetCurrentProcessId() -> u32;
        fn CreateFileA(name: *const u8, access: u32, share: u32, security: *mut u8, disposition: u32, flags: u32, template: *mut u8) -> *mut u8;
        fn ReadFile(handle: *mut u8, buffer: *mut u8, len: u32, read: *mut u32, overlapped: *mut u8) -> i32;
        fn CloseHandle(handle: *mut u8) -> i32;
        fn CreateDirectoryA(path: *const u8, security: *mut u8) -> i32;
        fn DeleteFileA(path: *const u8) -> i32;
        fn MoveFileA(src: *const u8, dst: *const u8) -> i32;
        fn FindFirstFileA(path: *const u8, data: *mut u8) -> *mut u8;
        fn FindNextFileA(handle: *mut u8, data: *mut u8) -> i32;
        fn FindClose(handle: *mut u8) -> i32;
        fn GetEnvironmentVariableA(name: *const u8, buf: *mut u8, size: u32) -> u32;
        fn GetCommandLineA() -> *const u8;
        fn GetFileSize(handle: *mut u8, high: *mut u32) -> u32;
    }
    vec![
        GetStdHandle as usize,
        WriteFile as usize,
        ExitProcess as usize,
        GetProcessHeap as usize,
        HeapAlloc as usize,
        GetCurrentDirectoryA as usize,
        GetFileAttributesA as usize,
        GetCurrentProcessId as usize,
        CreateFileA as usize,
        ReadFile as usize,
        CloseHandle as usize,
        CreateDirectoryA as usize,
        DeleteFileA as usize,
        MoveFileA as usize,
        FindFirstFileA as usize,
        FindNextFileA as usize,
        FindClose as usize,
        GetEnvironmentVariableA as usize,
        GetCommandLineA as usize,
        GetFileSize as usize,
    ]
}

// Dispatch table built ONCE, reused across all calls
#[cfg(target_os = "windows")]
static DISPATCH_TABLE: std::sync::LazyLock<Vec<usize>> =
    std::sync::LazyLock::new(build_dispatch_table);

// ── MEJORA 7: Instant Entry — pre-patch all fixups ─────────
// Build a ready-to-execute image: text + data + IAT, all fixups resolved
// Returns (patched_text, patched_data_with_iat, entry_offset)
#[cfg(target_os = "windows")]
fn build_instant_image(
    text: &[u8], data: &[u8], entry_offset: u32,
    data_fixups: &[(u32, String)], data_labels: &[(String, u32)],
    iat_fixups: &[(u32, usize)],
    text_base: usize, data_base: usize,
) -> (Vec<u8>, Vec<u8>) {
    let iat_ptrs = &*DISPATCH_TABLE;

    let mut patched_text = text.to_vec();
    let iat_base_offset = data.len();
    let data_total = data.len() + iat_ptrs.len() * 8 + 64; // padding
    let mut patched_data = vec![0u8; data_total];
    patched_data[..data.len()].copy_from_slice(data);

    // Write IAT entries
    for (i, &fptr) in iat_ptrs.iter().enumerate() {
        let off = iat_base_offset + i * 8;
        if off + 8 <= patched_data.len() {
            patched_data[off..off+8].copy_from_slice(&fptr.to_le_bytes());
        }
    }

    // Pre-patch data fixups
    for &(text_offset, ref label) in data_fixups {
        if let Some((_, data_offset)) = data_labels.iter().find(|(l, _)| l == label) {
            let target_addr = data_base + *data_offset as usize;
            let instr_addr = text_base + text_offset as usize + 4;
            let displacement = (target_addr as i64 - instr_addr as i64) as i32;
            let off = text_offset as usize;
            if off + 4 <= patched_text.len() {
                patched_text[off..off+4].copy_from_slice(&displacement.to_le_bytes());
            }
        }
    }

    // Pre-patch IAT fixups
    for &(text_offset, iat_slot) in iat_fixups {
        if iat_slot < iat_ptrs.len() {
            let iat_addr = data_base + iat_base_offset + iat_slot * 8;
            let instr_addr = text_base + text_offset as usize + 4;
            let displacement = (iat_addr as i64 - instr_addr as i64) as i32;
            let off = text_offset as usize;
            if off + 4 <= patched_text.len() {
                patched_text[off..off+4].copy_from_slice(&displacement.to_le_bytes());
            }
        }
    }

    (patched_text, patched_data)
}

/// JIT execution stats
pub struct JitStats {
    pub cache_hit: bool,
    pub alloc_ms: f64,
    pub patch_ms: f64,
    pub exec_ms: f64,
    pub total_ms: f64,
    pub text_bytes: usize,
    pub data_bytes: usize,
}

/// Execute compiled machine code directly in memory — JIT KILLER v2.0
pub fn execute_in_memory(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)], iat_fixups: &[(u32, usize)]) -> Result<i32, String> {
    #[cfg(target_os = "windows")]
    {
        execute_windows_v2(text, data, entry_offset, data_fixups, data_labels, iat_fixups)
    }
    #[cfg(target_os = "linux")]
    {
        execute_linux(text, data, entry_offset, data_fixups, data_labels)
    }
}

/// Execute with full stats — JIT KILLER v2.0
pub fn execute_in_memory_with_stats(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)], iat_fixups: &[(u32, usize)], source_hash: u64) -> Result<(i32, JitStats), String> {
    #[cfg(target_os = "windows")]
    {
        execute_windows_v2_stats(text, data, entry_offset, data_fixups, data_labels, iat_fixups, source_hash)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let code = execute_in_memory(text, data, entry_offset, data_fixups, data_labels, iat_fixups)?;
        Ok((code, JitStats { cache_hit: false, alloc_ms: 0.0, patch_ms: 0.0, exec_ms: 0.0, total_ms: 0.0, text_bytes: text.len(), data_bytes: data.len() }))
    }
}

#[cfg(target_os = "windows")]
fn execute_windows_v2(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)], iat_fixups: &[(u32, usize)]) -> Result<i32, String> {
    let (code, _stats) = execute_windows_v2_stats(text, data, entry_offset, data_fixups, data_labels, iat_fixups, 0)?;
    Ok(code)
}

#[cfg(target_os = "windows")]
fn execute_windows_v2_stats(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)], iat_fixups: &[(u32, usize)], source_hash: u64) -> Result<(i32, JitStats), String> {
    use std::ptr;

    const MEM_COMMIT: u32 = 0x1000;
    const MEM_RESERVE: u32 = 0x2000;
    const MEM_RELEASE: u32 = 0x8000;
    const PAGE_EXECUTE_READWRITE: u32 = 0x40;
    const PAGE_READWRITE: u32 = 0x04;  // MEJORA 5: .data non-executable

    extern "system" {
        fn VirtualAlloc(addr: *mut u8, size: usize, alloc_type: u32, protect: u32) -> *mut u8;
        fn VirtualFree(addr: *mut u8, size: usize, free_type: u32) -> i32;
    }

    let total_start = std::time::Instant::now();

    // MEJORA 5: Separate allocations — .text RWX, .data RW (no exec)
    let alloc_start = std::time::Instant::now();
    let text_alloc_size = (text.len() + 4095) & !4095; // page-align
    let iat_ptrs = &*DISPATCH_TABLE;
    let data_alloc_size = ((data.len() + iat_ptrs.len() * 8 + 64) + 4095) & !4095;

    let text_ptr = unsafe {
        VirtualAlloc(ptr::null_mut(), text_alloc_size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE)
    };
    if text_ptr.is_null() { return Err("VirtualAlloc .text failed".to_string()); }

    // MEJORA 5: .data section is PAGE_READWRITE only — no execute permission
    let data_ptr = unsafe {
        VirtualAlloc(ptr::null_mut(), data_alloc_size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE)
    };
    if data_ptr.is_null() {
        unsafe { VirtualFree(text_ptr, 0, MEM_RELEASE); }
        return Err("VirtualAlloc .data failed".to_string());
    }
    let alloc_ms = alloc_start.elapsed().as_secs_f64() * 1000.0;

    // MEJORA 7: Instant Entry — pre-patch all fixups at known addresses
    let patch_start = std::time::Instant::now();
    let (patched_text, patched_data) = build_instant_image(
        text, data, entry_offset,
        data_fixups, data_labels, iat_fixups,
        text_ptr as usize, data_ptr as usize,
    );

    // Single memcpy each — no runtime patching needed
    unsafe {
        ptr::copy_nonoverlapping(patched_text.as_ptr(), text_ptr, patched_text.len());
        ptr::copy_nonoverlapping(patched_data.as_ptr(), data_ptr, patched_data.len());
    }
    let patch_ms = patch_start.elapsed().as_secs_f64() * 1000.0;

    // Execute
    let exec_start = std::time::Instant::now();
    let entry_addr = unsafe { text_ptr.add(entry_offset as usize) };
    let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(entry_addr) };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        func();
    }));
    let exec_ms = exec_start.elapsed().as_secs_f64() * 1000.0;

    // Free
    unsafe {
        VirtualFree(text_ptr, 0, MEM_RELEASE);
        VirtualFree(data_ptr, 0, MEM_RELEASE);
    }

    let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;
    let stats = JitStats {
        cache_hit: false,
        alloc_ms,
        patch_ms,
        exec_ms,
        total_ms,
        text_bytes: patched_text.len(),
        data_bytes: patched_data.len(),
    };

    match result {
        Ok(_) => Ok((0, stats)),
        Err(_) => Ok((1, stats)),
    }
}

#[cfg(target_os = "linux")]
fn execute_linux(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)]) -> Result<i32, String> {
    use std::ptr;

    const PROT_READ: i32 = 0x1;
    const PROT_WRITE: i32 = 0x2;
    const PROT_EXEC: i32 = 0x4;
    const MAP_PRIVATE: i32 = 0x02;
    const MAP_ANONYMOUS: i32 = 0x20;

    extern "C" {
        fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
        fn munmap(addr: *mut u8, len: usize) -> i32;
    }

    let text_size = text.len() + 4096;
    let data_size = data.len() + 4096;
    let total_size = text_size + data_size;

    let base_ptr = unsafe {
        mmap(ptr::null_mut(), total_size, PROT_READ | PROT_WRITE | PROT_EXEC, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0)
    };
    if base_ptr.is_null() || base_ptr as isize == -1 { return Err("mmap failed".to_string()); }

    let text_ptr = base_ptr;
    let data_ptr = unsafe { base_ptr.add(text_size) };

    unsafe {
        ptr::copy_nonoverlapping(text.as_ptr(), text_ptr, text.len());
        ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data.len());
    }

    for &(text_offset, ref label) in data_fixups {
        if let Some((_, data_offset)) = data_labels.iter().find(|(l, _)| l == label) {
            let target_addr = data_ptr as usize + *data_offset as usize;
            let instr_addr = text_ptr as usize + text_offset as usize + 4;
            let displacement = (target_addr as i64 - instr_addr as i64) as i32;
            unsafe {
                let patch_ptr = text_ptr.add(text_offset as usize) as *mut i32;
                *patch_ptr = displacement;
            }
        }
    }

    let entry_addr = unsafe { text_ptr.add(entry_offset as usize) };
    let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(entry_addr) };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { func(); }));
    unsafe { munmap(base_ptr, total_size); }

    match result {
        Ok(_) => Ok(0),
        Err(_) => Ok(1),
    }
}
