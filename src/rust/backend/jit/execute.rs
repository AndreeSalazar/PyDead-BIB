use super::image::build_instant_image;
use super::dispatch::DISPATCH_TABLE;

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
pub fn execute_windows_v2(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)], iat_fixups: &[(u32, usize)]) -> Result<i32, String> {
    let (code, _stats) = execute_windows_v2_stats(text, data, entry_offset, data_fixups, data_labels, iat_fixups, 0)?;
    Ok(code)
}

#[cfg(target_os = "windows")]
pub fn execute_windows_v2_stats(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)], iat_fixups: &[(u32, usize)], source_hash: u64) -> Result<(i32, JitStats), String> {
    use std::ptr;

    const MEM_COMMIT: u32 = 0x1000;
    const MEM_RESERVE: u32 = 0x2000;
    const MEM_RELEASE: u32 = 0x8000;
    const PAGE_EXECUTE_READWRITE: u32 = 0x40;
    const PAGE_READWRITE: u32 = 0x04;  // MEJORA 5: .data non-executable

    extern "system" {
        pub fn VirtualAlloc(addr: *mut u8, size: usize, alloc_type: u32, protect: u32) -> *mut u8;
        pub fn VirtualFree(addr: *mut u8, size: usize, free_type: u32) -> i32;
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
pub fn execute_linux(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)]) -> Result<i32, String> {
    use std::ptr;

    const PROT_READ: i32 = 0x1;
    const PROT_WRITE: i32 = 0x2;
    const PROT_EXEC: i32 = 0x4;
    const MAP_PRIVATE: i32 = 0x02;
    const MAP_ANONYMOUS: i32 = 0x20;

    extern "C" {
        pub fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
        pub fn munmap(addr: *mut u8, len: usize) -> i32;
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
