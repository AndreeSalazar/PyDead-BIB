// ============================================================
// ADead-BIB — IAT Registry (Shared between PE builder + ISA)
// ============================================================
// Defines the canonical Import Address Table layout for all
// DLLs and functions. Both pe.rs and isa_compiler.rs reference
// this to keep IAT slot RVAs consistent.
//
// Each IAT slot is 8 bytes (PE32+ / x64).
// ============================================================

/// One imported function
#[derive(Debug, Clone)]
pub struct IatEntry {
    pub dll: &'static str,
    pub name: &'static str,
    pub slot_index: usize,
}

/// The canonical IAT table — order matters! slot_index = position in IAT.
/// Inspired by Rust windows-rs: use ANSI (A) variants to avoid wide strings.
pub const IAT_ENTRIES: &[IatEntry] = &[
    // msvcrt.dll (slots 0-4)
    IatEntry { dll: "msvcrt.dll",   name: "printf",               slot_index: 0  },
    IatEntry { dll: "msvcrt.dll",   name: "scanf",                slot_index: 1  },
    IatEntry { dll: "msvcrt.dll",   name: "malloc",               slot_index: 2  },
    IatEntry { dll: "msvcrt.dll",   name: "free",                 slot_index: 3  },
    IatEntry { dll: "msvcrt.dll",   name: "memset",               slot_index: 4  },
    // kernel32.dll (slots 5-11)
    IatEntry { dll: "kernel32.dll", name: "GetModuleHandleA",     slot_index: 5  },
    IatEntry { dll: "kernel32.dll", name: "GetModuleHandleW",     slot_index: 6  },
    IatEntry { dll: "kernel32.dll", name: "ExitProcess",          slot_index: 7  },
    IatEntry { dll: "kernel32.dll", name: "CreateEventA",         slot_index: 8  },
    IatEntry { dll: "kernel32.dll", name: "WaitForSingleObject",  slot_index: 9  },
    IatEntry { dll: "kernel32.dll", name: "CloseHandle",          slot_index: 10 },
    IatEntry { dll: "kernel32.dll", name: "Sleep",                slot_index: 11 },
    // user32.dll (slots 12-27) — both A and W variants
    IatEntry { dll: "user32.dll",   name: "RegisterClassExA",     slot_index: 12 },
    IatEntry { dll: "user32.dll",   name: "RegisterClassExW",     slot_index: 13 },
    IatEntry { dll: "user32.dll",   name: "CreateWindowExA",      slot_index: 14 },
    IatEntry { dll: "user32.dll",   name: "CreateWindowExW",      slot_index: 15 },
    IatEntry { dll: "user32.dll",   name: "ShowWindow",           slot_index: 16 },
    IatEntry { dll: "user32.dll",   name: "UpdateWindow",         slot_index: 17 },
    IatEntry { dll: "user32.dll",   name: "PeekMessageA",         slot_index: 18 },
    IatEntry { dll: "user32.dll",   name: "GetMessageW",          slot_index: 19 },
    IatEntry { dll: "user32.dll",   name: "TranslateMessage",     slot_index: 20 },
    IatEntry { dll: "user32.dll",   name: "DispatchMessageA",     slot_index: 21 },
    IatEntry { dll: "user32.dll",   name: "DispatchMessageW",     slot_index: 22 },
    IatEntry { dll: "user32.dll",   name: "PostQuitMessage",      slot_index: 23 },
    IatEntry { dll: "user32.dll",   name: "DefWindowProcA",       slot_index: 24 },
    IatEntry { dll: "user32.dll",   name: "DefWindowProcW",       slot_index: 25 },
    IatEntry { dll: "user32.dll",   name: "LoadCursorW",          slot_index: 26 },
    IatEntry { dll: "user32.dll",   name: "AdjustWindowRect",     slot_index: 27 },
    // gdi32.dll (slots 28-36) — GDI rendering functions
    IatEntry { dll: "gdi32.dll",    name: "SetPixel",                slot_index: 28 },
    IatEntry { dll: "gdi32.dll",    name: "CreateSolidBrush",       slot_index: 29 },
    IatEntry { dll: "gdi32.dll",    name: "DeleteObject",           slot_index: 30 },
    IatEntry { dll: "gdi32.dll",    name: "SelectObject",           slot_index: 31 },
    IatEntry { dll: "gdi32.dll",    name: "Rectangle",              slot_index: 32 },
    IatEntry { dll: "gdi32.dll",    name: "CreatePen",              slot_index: 33 },
    IatEntry { dll: "gdi32.dll",    name: "MoveToEx",               slot_index: 34 },
    IatEntry { dll: "gdi32.dll",    name: "LineTo",                 slot_index: 35 },
    IatEntry { dll: "gdi32.dll",    name: "Polygon",                slot_index: 36 },
    // user32.dll extras (slots 37-40) — DC + painting
    IatEntry { dll: "user32.dll",   name: "GetDC",                  slot_index: 37 },
    IatEntry { dll: "user32.dll",   name: "ReleaseDC",              slot_index: 38 },
    IatEntry { dll: "user32.dll",   name: "InvalidateRect",         slot_index: 39 },
    IatEntry { dll: "user32.dll",   name: "FillRect",               slot_index: 40 },
    // d3d12.dll (slots 41-43)
    IatEntry { dll: "d3d12.dll",    name: "D3D12CreateDevice",              slot_index: 41 },
    IatEntry { dll: "d3d12.dll",    name: "D3D12GetDebugInterface",         slot_index: 42 },
    IatEntry { dll: "d3d12.dll",    name: "D3D12SerializeRootSignature",    slot_index: 43 },
    // dxgi.dll (slots 44-45)
    IatEntry { dll: "dxgi.dll",     name: "CreateDXGIFactory1",   slot_index: 44 },
    IatEntry { dll: "dxgi.dll",     name: "CreateDXGIFactory2",   slot_index: 45 },
    // msvcrt.dll extras (slot 46)
    IatEntry { dll: "msvcrt.dll",   name: "memcpy",               slot_index: 46 },
];

pub const IAT_SLOT_COUNT: usize = 47;

/// Get the unique DLL names in order of first appearance
pub fn dll_names() -> Vec<&'static str> {
    let mut dlls: Vec<&'static str> = Vec::new();
    for e in IAT_ENTRIES {
        if !dlls.contains(&e.dll) {
            dlls.push(e.dll);
        }
    }
    dlls
}

/// Get entries for a specific DLL
pub fn entries_for_dll(dll: &str) -> Vec<&'static IatEntry> {
    IAT_ENTRIES.iter().filter(|e| e.dll == dll).collect()
}

/// Lookup slot index by function name. Returns None if not found.
pub fn slot_for_function(name: &str) -> Option<usize> {
    IAT_ENTRIES.iter().find(|e| e.name == name).map(|e| e.slot_index)
}

/// Given the idata_rva and the IAT offset within idata, compute the
/// absolute IAT RVA for a given slot index.
pub fn iat_rva_for_slot(idata_rva: u32, iat_offset: u32, slot_index: usize) -> u32 {
    idata_rva + iat_offset + (slot_index as u32) * 8
}

/// Build the complete .idata section bytes.
/// Returns (idata_bytes, iat_offset_within_idata, idt_size, iat_size, program_strings_offset).
///
/// Layout:
///   IDT: (num_dlls + 1) * 20 bytes (null-terminated)
///   ILTs: per-DLL ILT arrays (each null-terminated, 8 bytes per entry)
///   IAT: all slots contiguous, (IAT_SLOT_COUNT + padding_nulls) * 8 bytes
///   Strings: DLL names + hint/name entries
///   Program data: appended after strings
pub fn build_idata(idata_rva: u32, program_data: &[u8]) -> IdataResult {
    let dlls = dll_names();
    let num_dlls = dlls.len();

    // Phase 1: compute sizes
    let idt_size = (num_dlls + 1) * 20; // +1 for null terminator

    // ILT: one array per DLL, each entry is 8 bytes + 8-byte null terminator
    let mut ilt_sizes: Vec<usize> = Vec::new();
    for dll in &dlls {
        let count = entries_for_dll(dll).len();
        ilt_sizes.push((count + 1) * 8); // +1 null terminator
    }
    let total_ilt_size: usize = ilt_sizes.iter().sum();

    // IAT: contiguous block for ALL slots + null terminators per DLL
    // We lay out IAT as: [dll0 slots + null][dll1 slots + null][...]
    let mut iat_sizes: Vec<usize> = Vec::new();
    for dll in &dlls {
        let count = entries_for_dll(dll).len();
        iat_sizes.push((count + 1) * 8); // +1 null terminator
    }
    let total_iat_size: usize = iat_sizes.iter().sum();

    // Strings area: DLL names + Hint/Name entries
    let mut strings_size = 0usize;
    for dll in &dlls {
        strings_size += dll.len() + 1; // +null
        if strings_size % 2 != 0 { strings_size += 1; } // align to 2
    }
    for entry in IAT_ENTRIES {
        strings_size += 2 + entry.name.len() + 1; // hint(2) + name + null
        if strings_size % 2 != 0 { strings_size += 1; } // align to 2
    }

    let ilt_offset = idt_size;
    let iat_offset = ilt_offset + total_ilt_size;
    let strings_offset = iat_offset + total_iat_size;
    let program_data_offset = strings_offset + strings_size;

    let total_size = program_data_offset + program_data.len();
    let aligned_size = (total_size + 0x1FF) & !0x1FF; // align to 0x200
    let mut idata = vec![0u8; aligned_size];

    // Phase 2: write strings first (we need RVAs for ILT/IAT)
    let mut str_pos = strings_offset;

    // DLL name RVAs
    let mut dll_name_rvas: Vec<u32> = Vec::new();
    for dll in &dlls {
        dll_name_rvas.push(idata_rva + str_pos as u32);
        let name_bytes = dll.as_bytes();
        idata[str_pos..str_pos + name_bytes.len()].copy_from_slice(name_bytes);
        str_pos += name_bytes.len() + 1; // +null
        if str_pos % 2 != 0 { str_pos += 1; } // align
    }

    // Hint/Name RVAs per entry
    let mut hint_name_rvas: Vec<u32> = vec![0; IAT_SLOT_COUNT];
    for entry in IAT_ENTRIES {
        hint_name_rvas[entry.slot_index] = idata_rva + str_pos as u32;
        // Hint (2 bytes) = 0
        idata[str_pos] = 0;
        idata[str_pos + 1] = 0;
        str_pos += 2;
        let name_bytes = entry.name.as_bytes();
        idata[str_pos..str_pos + name_bytes.len()].copy_from_slice(name_bytes);
        str_pos += name_bytes.len() + 1; // +null
        if str_pos % 2 != 0 { str_pos += 1; } // align
    }

    // Phase 3: write ILT arrays
    let mut ilt_pos = ilt_offset;
    let mut dll_ilt_rvas: Vec<u32> = Vec::new();
    for dll in &dlls {
        dll_ilt_rvas.push(idata_rva + ilt_pos as u32);
        let entries = entries_for_dll(dll);
        for entry in &entries {
            let hint_rva = hint_name_rvas[entry.slot_index] as u64;
            idata[ilt_pos..ilt_pos + 8].copy_from_slice(&hint_rva.to_le_bytes());
            ilt_pos += 8;
        }
        ilt_pos += 8; // null terminator
    }

    // Phase 4: write IAT (same layout as ILT initially — Windows loader overwrites)
    let mut iat_pos = iat_offset;
    let mut dll_iat_rvas: Vec<u32> = Vec::new();
    // We also need to track absolute IAT RVA per slot_index for the ISA compiler
    let mut slot_to_iat_rva: Vec<u32> = vec![0; IAT_SLOT_COUNT];
    for (di, dll) in dlls.iter().enumerate() {
        dll_iat_rvas.push(idata_rva + iat_pos as u32);
        let entries = entries_for_dll(dll);
        for entry in &entries {
            let hint_rva = hint_name_rvas[entry.slot_index] as u64;
            idata[iat_pos..iat_pos + 8].copy_from_slice(&hint_rva.to_le_bytes());
            slot_to_iat_rva[entry.slot_index] = idata_rva + iat_pos as u32;
            iat_pos += 8;
        }
        iat_pos += 8; // null terminator
        let _ = di;
    }

    // Phase 5: write IDT
    for (di, _dll) in dlls.iter().enumerate() {
        let idt_entry_offset = di * 20;
        // OriginalFirstThunk (ILT RVA)
        idata[idt_entry_offset..idt_entry_offset + 4]
            .copy_from_slice(&dll_ilt_rvas[di].to_le_bytes());
        // TimeDateStamp = 0 (already)
        // ForwarderChain = 0 (already)
        // Name (DLL name RVA)
        idata[idt_entry_offset + 12..idt_entry_offset + 16]
            .copy_from_slice(&dll_name_rvas[di].to_le_bytes());
        // FirstThunk (IAT RVA)
        idata[idt_entry_offset + 16..idt_entry_offset + 20]
            .copy_from_slice(&dll_iat_rvas[di].to_le_bytes());
    }
    // null terminator IDT entry (already zeros)

    // Phase 6: append program data
    if !program_data.is_empty() {
        idata[program_data_offset..program_data_offset + program_data.len()]
            .copy_from_slice(program_data);
    }

    IdataResult {
        idata,
        iat_offset: iat_offset as u32,
        idt_size: idt_size as u32,
        total_iat_size: total_iat_size as u32,
        program_strings_offset: program_data_offset as u32,
        slot_to_iat_rva,
    }
}

/// Result of build_idata
pub struct IdataResult {
    pub idata: Vec<u8>,
    pub iat_offset: u32,
    pub idt_size: u32,
    pub total_iat_size: u32,
    pub program_strings_offset: u32,
    /// IAT RVA for each slot index (used by ISA compiler for CallIAT)
    pub slot_to_iat_rva: Vec<u32>,
}
