// ELF (Executable and Linkable Format) Generator
// Genera binarios Linux x86-64 PUROS sin dependencias
// HEX + Binario = ADead-BIB
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com

use std::fs::File;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// Constantes ELF
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1; // Little endian
const EV_CURRENT: u8 = 1;
const ELFOSABI_NONE: u8 = 0;
const ET_EXEC: u16 = 2; // Executable
const EM_X86_64: u16 = 62; // AMD x86-64
const PT_LOAD: u32 = 1; // Loadable segment
const PF_X: u32 = 1; // Execute
const PF_W: u32 = 2; // Write
const PF_R: u32 = 4; // Read

const BASE_ADDR: u64 = 0x400000;
const HEADER_SIZE: u64 = 64; // ELF header
const PHDR_SIZE: u64 = 56; // Program header entry
const PHDR_COUNT: u64 = 1; // Solo un segmento LOAD

/// Genera un binario ELF x86-64 ejecutable
pub fn generate_elf(
    opcodes: &[u8],
    data: &[u8],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut binary = Vec::new();

    // Calcular offsets
    let code_offset = HEADER_SIZE + (PHDR_SIZE * PHDR_COUNT);
    let data_offset = code_offset + opcodes.len() as u64;
    let total_size = data_offset + data.len() as u64;

    // Entry point = después de headers
    let entry_point = BASE_ADDR + code_offset;

    // ========================================
    // ELF Header (64 bytes)
    // ========================================

    // e_ident[16]
    binary.extend_from_slice(&ELF_MAGIC); // Magic
    binary.push(ELFCLASS64); // 64-bit
    binary.push(ELFDATA2LSB); // Little endian
    binary.push(EV_CURRENT); // ELF version
    binary.push(ELFOSABI_NONE); // OS/ABI
    binary.extend_from_slice(&[0u8; 8]); // Padding

    // e_type (2 bytes)
    binary.extend_from_slice(&ET_EXEC.to_le_bytes());

    // e_machine (2 bytes)
    binary.extend_from_slice(&EM_X86_64.to_le_bytes());

    // e_version (4 bytes)
    binary.extend_from_slice(&1u32.to_le_bytes());

    // e_entry (8 bytes) - Entry point
    binary.extend_from_slice(&entry_point.to_le_bytes());

    // e_phoff (8 bytes) - Program header offset
    binary.extend_from_slice(&HEADER_SIZE.to_le_bytes());

    // e_shoff (8 bytes) - Section header offset (0 = none)
    binary.extend_from_slice(&0u64.to_le_bytes());

    // e_flags (4 bytes)
    binary.extend_from_slice(&0u32.to_le_bytes());

    // e_ehsize (2 bytes) - ELF header size
    binary.extend_from_slice(&(HEADER_SIZE as u16).to_le_bytes());

    // e_phentsize (2 bytes) - Program header entry size
    binary.extend_from_slice(&(PHDR_SIZE as u16).to_le_bytes());

    // e_phnum (2 bytes) - Number of program headers
    binary.extend_from_slice(&(PHDR_COUNT as u16).to_le_bytes());

    // e_shentsize (2 bytes) - Section header entry size
    binary.extend_from_slice(&0u16.to_le_bytes());

    // e_shnum (2 bytes) - Number of section headers
    binary.extend_from_slice(&0u16.to_le_bytes());

    // e_shstrndx (2 bytes) - Section name string table index
    binary.extend_from_slice(&0u16.to_le_bytes());

    assert_eq!(binary.len(), 64, "ELF header debe ser 64 bytes");

    // ========================================
    // Program Header (56 bytes)
    // ========================================

    // p_type (4 bytes)
    binary.extend_from_slice(&PT_LOAD.to_le_bytes());

    // p_flags (4 bytes) - RWX
    binary.extend_from_slice(&(PF_R | PF_W | PF_X).to_le_bytes());

    // p_offset (8 bytes) - Offset in file
    binary.extend_from_slice(&0u64.to_le_bytes());

    // p_vaddr (8 bytes) - Virtual address
    binary.extend_from_slice(&BASE_ADDR.to_le_bytes());

    // p_paddr (8 bytes) - Physical address
    binary.extend_from_slice(&BASE_ADDR.to_le_bytes());

    // p_filesz (8 bytes) - Size in file
    binary.extend_from_slice(&total_size.to_le_bytes());

    // p_memsz (8 bytes) - Size in memory
    binary.extend_from_slice(&total_size.to_le_bytes());

    // p_align (8 bytes) - Alignment
    binary.extend_from_slice(&0x1000u64.to_le_bytes());

    assert_eq!(binary.len(), 120, "Headers deben ser 120 bytes");

    // ========================================
    // Code Section
    // ========================================
    binary.extend_from_slice(opcodes);

    // ========================================
    // Data Section
    // ========================================
    binary.extend_from_slice(data);

    // ========================================
    // Write to file
    // ========================================
    let mut file = File::create(output_path)?;
    file.write_all(&binary)?;

    // Hacer ejecutable en Unix
    #[cfg(unix)]
    {
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(output_path, perms)?;
    }

    Ok(())
}

/// Genera código de inicio para Linux (sin libc)
pub fn generate_linux_start(code: &mut Vec<u8>, main_offset: i32) {
    // _start:
    //   xor rbp, rbp          ; Clear frame pointer
    //   call main
    //   mov rdi, rax          ; Exit code from main
    //   mov rax, 60           ; sys_exit
    //   syscall

    // xor rbp, rbp
    code.extend_from_slice(&[0x48, 0x31, 0xED]);

    // call main (rel32)
    code.extend_from_slice(&[0xE8]);
    code.extend_from_slice(&main_offset.to_le_bytes());

    // mov rdi, rax
    code.extend_from_slice(&[0x48, 0x89, 0xC7]);

    // mov rax, 60 (sys_exit)
    code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00]);

    // syscall
    code.extend_from_slice(&[0x0F, 0x05]);
}

/// Genera código para print en Linux (syscall directo)
pub fn emit_linux_print(code: &mut Vec<u8>, string_addr: u64, length: u32) {
    // mov rax, 1 (sys_write)
    code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00]);

    // mov rdi, 1 (stdout)
    code.extend_from_slice(&[0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00]);

    // mov rsi, string_addr
    code.extend_from_slice(&[0x48, 0xBE]);
    code.extend_from_slice(&string_addr.to_le_bytes());

    // mov rdx, length
    code.extend_from_slice(&[0x48, 0xC7, 0xC2]);
    code.extend_from_slice(&length.to_le_bytes());

    // syscall
    code.extend_from_slice(&[0x0F, 0x05]);
}

/// Genera código para exit en Linux
pub fn emit_linux_exit(code: &mut Vec<u8>, exit_code: u32) {
    // mov rax, 60 (sys_exit)
    code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00]);

    // mov rdi, exit_code
    if exit_code == 0 {
        code.extend_from_slice(&[0x48, 0x31, 0xFF]); // xor rdi, rdi
    } else {
        code.extend_from_slice(&[0x48, 0xC7, 0xC7]);
        code.extend_from_slice(&exit_code.to_le_bytes());
    }

    // syscall
    code.extend_from_slice(&[0x0F, 0x05]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_header_size() {
        let mut binary = Vec::new();

        // Simular header
        binary.extend_from_slice(&ELF_MAGIC);
        binary.push(ELFCLASS64);
        binary.push(ELFDATA2LSB);
        binary.push(EV_CURRENT);
        binary.push(ELFOSABI_NONE);
        binary.extend_from_slice(&[0u8; 8]);
        binary.extend_from_slice(&ET_EXEC.to_le_bytes());
        binary.extend_from_slice(&EM_X86_64.to_le_bytes());
        binary.extend_from_slice(&1u32.to_le_bytes());
        binary.extend_from_slice(&0u64.to_le_bytes()); // entry
        binary.extend_from_slice(&64u64.to_le_bytes()); // phoff
        binary.extend_from_slice(&0u64.to_le_bytes()); // shoff
        binary.extend_from_slice(&0u32.to_le_bytes()); // flags
        binary.extend_from_slice(&64u16.to_le_bytes()); // ehsize
        binary.extend_from_slice(&56u16.to_le_bytes()); // phentsize
        binary.extend_from_slice(&1u16.to_le_bytes()); // phnum
        binary.extend_from_slice(&0u16.to_le_bytes()); // shentsize
        binary.extend_from_slice(&0u16.to_le_bytes()); // shnum
        binary.extend_from_slice(&0u16.to_le_bytes()); // shstrndx

        assert_eq!(binary.len(), 64);
    }
}
