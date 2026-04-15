
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
pub static DISPATCH_TABLE: std::sync::LazyLock<Vec<usize>> =
    std::sync::LazyLock::new(build_dispatch_table);

