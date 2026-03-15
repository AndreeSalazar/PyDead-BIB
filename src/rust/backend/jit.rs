// ============================================================
// PyDead-BIB JIT Executor v4.0 — FASE 2
// ============================================================
// In-memory execution via VirtualAlloc (Windows) / mmap (Linux)
// No .exe written to disk — compile → execute in RAM
// Pipeline: .py → IR → ISA → VirtualAlloc → JMP → result
// ============================================================

/// Execute compiled machine code directly in memory
/// Returns the exit code from the executed code
pub fn execute_in_memory(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)], iat_fixups: &[(u32, usize)]) -> Result<i32, String> {
    #[cfg(target_os = "windows")]
    {
        execute_windows(text, data, entry_offset, data_fixups, data_labels, iat_fixups)
    }
    #[cfg(target_os = "linux")]
    {
        execute_linux(text, data, entry_offset, data_fixups, data_labels)
    }
}

#[cfg(target_os = "windows")]
fn execute_windows(text: &[u8], data: &[u8], entry_offset: u32, data_fixups: &[(u32, String)], data_labels: &[(String, u32)], iat_fixups: &[(u32, usize)]) -> Result<i32, String> {
    use std::ptr;

    // Windows API constants
    const MEM_COMMIT: u32 = 0x1000;
    const MEM_RESERVE: u32 = 0x2000;
    const MEM_RELEASE: u32 = 0x8000;
    const PAGE_EXECUTE_READWRITE: u32 = 0x40;

    extern "system" {
        fn VirtualAlloc(addr: *mut u8, size: usize, alloc_type: u32, protect: u32) -> *mut u8;
        fn VirtualFree(addr: *mut u8, size: usize, free_type: u32) -> i32;
        fn GetStdHandle(std_handle: i32) -> *mut u8;
        fn WriteFile(handle: *mut u8, buffer: *const u8, len: u32, written: *mut u32, overlapped: *mut u8) -> i32;
        fn GetProcessHeap() -> *mut u8;
        fn HeapAlloc(heap: *mut u8, flags: u32, size: usize) -> *mut u8;
        fn ExitProcess(code: u32) -> !;
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

    // Build IAT function pointer table
    let iat_ptrs: Vec<usize> = unsafe {
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
    };

    // Allocate executable memory for text
    let text_size = text.len() + 4096; // extra padding
    let data_size = data.len() + 4096;
    let total_size = text_size + data_size;

    let base_ptr = unsafe {
        VirtualAlloc(
            ptr::null_mut(),
            total_size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };

    if base_ptr.is_null() {
        return Err("VirtualAlloc failed".to_string());
    }

    let text_ptr = base_ptr;
    let data_ptr = unsafe { base_ptr.add(text_size) };

    // Copy text and data sections
    unsafe {
        ptr::copy_nonoverlapping(text.as_ptr(), text_ptr, text.len());
        ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data.len());
    }

    // Apply data fixups (RIP-relative LEA instructions → patch displacement)
    for &(text_offset, ref label) in data_fixups {
        if let Some((_, data_offset)) = data_labels.iter().find(|(l, _)| l == label) {
            // RIP-relative: displacement = target - (instruction_addr + 4)
            // target = data_ptr + data_offset
            // instruction_addr = text_ptr + text_offset
            let target_addr = data_ptr as usize + *data_offset as usize;
            let instr_addr = text_ptr as usize + text_offset as usize + 4; // +4 for disp32 size
            let displacement = (target_addr as i64 - instr_addr as i64) as i32;
            unsafe {
                let patch_ptr = text_ptr.add(text_offset as usize) as *mut i32;
                *patch_ptr = displacement;
            }
        }
    }

    // Apply IAT fixups (indirect call via function pointers)
    // IAT fixups in the .exe use [RIP+disp] to read from IAT in .data
    // For JIT, we write the function pointer directly into the data section
    // We need to allocate IAT slots in data and patch the references
    let iat_base_offset = data.len();
    // Write IAT entries into data section
    for (i, &fptr) in iat_ptrs.iter().enumerate() {
        let slot_offset = iat_base_offset + i * 8;
        if slot_offset + 8 <= data_size {
            unsafe {
                let slot_ptr = data_ptr.add(slot_offset) as *mut usize;
                *slot_ptr = fptr;
            }
        }
    }

    // Patch IAT fixups in text to point to our IAT slots
    for &(text_offset, iat_slot) in iat_fixups {
        if iat_slot < iat_ptrs.len() {
            let iat_addr = data_ptr as usize + iat_base_offset + iat_slot * 8;
            let instr_addr = text_ptr as usize + text_offset as usize + 4;
            let displacement = (iat_addr as i64 - instr_addr as i64) as i32;
            unsafe {
                let patch_ptr = text_ptr.add(text_offset as usize) as *mut i32;
                *patch_ptr = displacement;
            }
        }
    }

    // Call the entry point
    let entry_addr = unsafe { text_ptr.add(entry_offset as usize) };
    let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(entry_addr) };

    // Use catch_unwind to handle ExitProcess calls gracefully
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        func();
    }));

    // Free memory
    unsafe {
        VirtualFree(base_ptr, 0, MEM_RELEASE);
    }

    match result {
        Ok(_) => Ok(0),
        Err(_) => Ok(1),
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
        mmap(
            ptr::null_mut(),
            total_size,
            PROT_READ | PROT_WRITE | PROT_EXEC,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    if base_ptr.is_null() || base_ptr as isize == -1 {
        return Err("mmap failed".to_string());
    }

    let text_ptr = base_ptr;
    let data_ptr = unsafe { base_ptr.add(text_size) };

    unsafe {
        ptr::copy_nonoverlapping(text.as_ptr(), text_ptr, text.len());
        ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data.len());
    }

    // Apply data fixups
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

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        func();
    }));

    unsafe {
        munmap(base_ptr, total_size);
    }

    match result {
        Ok(_) => Ok(0),
        Err(_) => Ok(1),
    }
}
