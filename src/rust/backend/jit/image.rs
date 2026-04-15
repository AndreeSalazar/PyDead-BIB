use super::dispatch::DISPATCH_TABLE;

// Build a ready-to-execute image: text + data + IAT, all fixups resolved
// Returns (patched_text, patched_data_with_iat, entry_offset)
#[cfg(target_os = "windows")]
pub fn build_instant_image(
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

