// ADead-BIB - Syscalls Directos
// Elimina dependencia de DLLs externas
// HEX + Binario = Sin Límites
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com

/// Syscalls para Windows NT (x64)
pub mod windows {
    /// Números de syscall de Windows NT
    /// NOTA: Estos números pueden variar entre versiones de Windows
    pub const NT_WRITE_FILE: u32 = 0x08;
    pub const NT_READ_FILE: u32 = 0x06;
    pub const NT_ALLOCATE_VIRTUAL_MEMORY: u32 = 0x18;
    pub const NT_FREE_VIRTUAL_MEMORY: u32 = 0x1E;
    pub const NT_TERMINATE_PROCESS: u32 = 0x2C;
    pub const NT_CLOSE: u32 = 0x0F;

    /// Handles estándar
    pub const STD_INPUT_HANDLE: i32 = -10;
    pub const STD_OUTPUT_HANDLE: i32 = -11;
    pub const STD_ERROR_HANDLE: i32 = -12;

    /// Genera opcodes para escribir a consola (sin DLL)
    /// Usa GetStdHandle + WriteFile inline
    pub fn emit_write_console(code: &mut Vec<u8>, string_rva: u64, length: u32) {
        // Método 1: Usando direcciones de kernel32 (requiere resolver en runtime)
        // Método 2: Syscall directo (más portable pero menos estable)

        // Por ahora usamos el método tradicional pero preparado para syscall
        // mov rcx, string_addr
        code.extend_from_slice(&[0x48, 0xB9]);
        code.extend_from_slice(&string_rva.to_le_bytes());

        // mov rdx, length
        code.extend_from_slice(&[0x48, 0xC7, 0xC2]);
        code.extend_from_slice(&length.to_le_bytes());
    }

    /// Genera opcodes para exit del proceso
    pub fn emit_exit_process(code: &mut Vec<u8>, exit_code: u32) {
        // xor ecx, ecx (exit code 0) o mov ecx, exit_code
        if exit_code == 0 {
            code.extend_from_slice(&[0x31, 0xC9]);
        } else {
            code.extend_from_slice(&[0xB9]);
            code.extend_from_slice(&exit_code.to_le_bytes());
        }
    }
}

/// Syscalls para Linux (x64)
pub mod linux {
    /// Números de syscall de Linux x86_64
    pub const SYS_READ: u64 = 0;
    pub const SYS_WRITE: u64 = 1;
    pub const SYS_OPEN: u64 = 2;
    pub const SYS_CLOSE: u64 = 3;
    pub const SYS_MMAP: u64 = 9;
    pub const SYS_MUNMAP: u64 = 11;
    pub const SYS_BRK: u64 = 12;
    pub const SYS_EXIT: u64 = 60;
    pub const SYS_EXIT_GROUP: u64 = 231;

    /// File descriptors estándar
    pub const STDIN: u64 = 0;
    pub const STDOUT: u64 = 1;
    pub const STDERR: u64 = 2;

    /// Genera opcodes para sys_write
    /// write(fd, buf, count)
    pub fn emit_write(code: &mut Vec<u8>, fd: u64, buf_addr: u64, count: u64) {
        // mov rax, SYS_WRITE (1)
        code.extend_from_slice(&[0x48, 0xC7, 0xC0]);
        code.extend_from_slice(&(SYS_WRITE as u32).to_le_bytes());

        // mov rdi, fd
        code.extend_from_slice(&[0x48, 0xC7, 0xC7]);
        code.extend_from_slice(&(fd as u32).to_le_bytes());

        // mov rsi, buf_addr
        code.extend_from_slice(&[0x48, 0xBE]);
        code.extend_from_slice(&buf_addr.to_le_bytes());

        // mov rdx, count
        code.extend_from_slice(&[0x48, 0xC7, 0xC2]);
        code.extend_from_slice(&(count as u32).to_le_bytes());

        // syscall
        code.extend_from_slice(&[0x0F, 0x05]);
    }

    /// Genera opcodes para sys_write con buffer en registro
    pub fn emit_write_reg(code: &mut Vec<u8>, fd: u64, count: u64) {
        // Asume que rsi ya tiene la dirección del buffer

        // mov rax, SYS_WRITE (1)
        code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00]);

        // mov rdi, fd
        code.extend_from_slice(&[0x48, 0xC7, 0xC7]);
        code.extend_from_slice(&(fd as u32).to_le_bytes());

        // mov rdx, count
        code.extend_from_slice(&[0x48, 0xC7, 0xC2]);
        code.extend_from_slice(&(count as u32).to_le_bytes());

        // syscall
        code.extend_from_slice(&[0x0F, 0x05]);
    }

    /// Genera opcodes para sys_exit
    pub fn emit_exit(code: &mut Vec<u8>, exit_code: u64) {
        // mov rax, SYS_EXIT (60)
        code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00]);

        // mov rdi, exit_code
        if exit_code == 0 {
            code.extend_from_slice(&[0x48, 0x31, 0xFF]); // xor rdi, rdi
        } else {
            code.extend_from_slice(&[0x48, 0xC7, 0xC7]);
            code.extend_from_slice(&(exit_code as u32).to_le_bytes());
        }

        // syscall
        code.extend_from_slice(&[0x0F, 0x05]);
    }

    /// Genera opcodes para sys_mmap (allocate memory)
    pub fn emit_mmap(code: &mut Vec<u8>, size: u64) {
        // mmap(NULL, size, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0)

        // mov rax, SYS_MMAP (9)
        code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x09, 0x00, 0x00, 0x00]);

        // mov rdi, 0 (addr = NULL)
        code.extend_from_slice(&[0x48, 0x31, 0xFF]);

        // mov rsi, size
        code.extend_from_slice(&[0x48, 0xBE]);
        code.extend_from_slice(&size.to_le_bytes());

        // mov rdx, 3 (PROT_READ | PROT_WRITE)
        code.extend_from_slice(&[0x48, 0xC7, 0xC2, 0x03, 0x00, 0x00, 0x00]);

        // mov r10, 0x22 (MAP_PRIVATE | MAP_ANONYMOUS)
        code.extend_from_slice(&[0x49, 0xC7, 0xC2, 0x22, 0x00, 0x00, 0x00]);

        // mov r8, -1 (fd = -1)
        code.extend_from_slice(&[0x49, 0xC7, 0xC0, 0xFF, 0xFF, 0xFF, 0xFF]);

        // mov r9, 0 (offset = 0)
        code.extend_from_slice(&[0x4D, 0x31, 0xC9]);

        // syscall
        code.extend_from_slice(&[0x0F, 0x05]);
    }

    /// Genera opcodes para sys_munmap (free memory)
    pub fn emit_munmap(code: &mut Vec<u8>) {
        // Asume rdi = addr, rsi = size

        // mov rax, SYS_MUNMAP (11)
        code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x0B, 0x00, 0x00, 0x00]);

        // syscall
        code.extend_from_slice(&[0x0F, 0x05]);
    }
}

/// Utilidades para generar código independiente de plataforma
pub mod portable {
    use super::*;

    #[derive(Clone, Copy, PartialEq)]
    pub enum Target {
        Windows,
        Linux,
        Raw,
    }

    /// Genera código para imprimir string según target
    pub fn emit_print_string(code: &mut Vec<u8>, target: Target, string_addr: u64, length: u32) {
        match target {
            Target::Linux => {
                linux::emit_write(code, linux::STDOUT, string_addr, length as u64);
            }
            Target::Windows | Target::Raw => {
                // Para Windows, necesitamos el IAT o syscalls
                // Por ahora, placeholder
                windows::emit_write_console(code, string_addr, length);
            }
        }
    }

    /// Genera código para exit según target
    pub fn emit_exit(code: &mut Vec<u8>, target: Target, exit_code: u32) {
        match target {
            Target::Linux => {
                linux::emit_exit(code, exit_code as u64);
            }
            Target::Windows | Target::Raw => {
                windows::emit_exit_process(code, exit_code);
            }
        }
    }

    /// Genera código para allocar memoria según target
    pub fn emit_alloc(code: &mut Vec<u8>, target: Target, size: u64) {
        match target {
            Target::Linux => {
                linux::emit_mmap(code, size);
            }
            Target::Windows | Target::Raw => {
                // VirtualAlloc via syscall o IAT
                // Placeholder
            }
        }
    }
}

/// Opcodes x86-64 comunes
pub mod opcodes {
    // Registros
    pub const RAX: u8 = 0;
    pub const RCX: u8 = 1;
    pub const RDX: u8 = 2;
    pub const RBX: u8 = 3;
    pub const RSP: u8 = 4;
    pub const RBP: u8 = 5;
    pub const RSI: u8 = 6;
    pub const RDI: u8 = 7;
    pub const R8: u8 = 8;
    pub const R9: u8 = 9;
    pub const R10: u8 = 10;
    pub const R11: u8 = 11;

    /// MOV reg64, imm64
    pub fn mov_reg_imm64(code: &mut Vec<u8>, reg: u8, imm: u64) {
        let rex = if reg >= 8 { 0x49 } else { 0x48 };
        let reg_code = reg & 0x07;
        code.push(rex);
        code.push(0xB8 + reg_code);
        code.extend_from_slice(&imm.to_le_bytes());
    }

    /// MOV reg64, imm32 (sign-extended)
    pub fn mov_reg_imm32(code: &mut Vec<u8>, reg: u8, imm: i32) {
        let rex = if reg >= 8 { 0x49 } else { 0x48 };
        let reg_code = reg & 0x07;
        code.push(rex);
        code.push(0xC7);
        code.push(0xC0 + reg_code);
        code.extend_from_slice(&imm.to_le_bytes());
    }

    /// XOR reg64, reg64 (zero register)
    pub fn xor_reg_reg(code: &mut Vec<u8>, reg: u8) {
        let rex = if reg >= 8 { 0x4D } else { 0x48 };
        let reg_code = reg & 0x07;
        code.push(rex);
        code.push(0x31);
        code.push(0xC0 + reg_code * 9); // reg, reg
    }

    /// PUSH reg64
    pub fn push_reg(code: &mut Vec<u8>, reg: u8) {
        if reg >= 8 {
            code.push(0x41);
        }
        code.push(0x50 + (reg & 0x07));
    }

    /// POP reg64
    pub fn pop_reg(code: &mut Vec<u8>, reg: u8) {
        if reg >= 8 {
            code.push(0x41);
        }
        code.push(0x58 + (reg & 0x07));
    }

    /// RET
    pub fn ret(code: &mut Vec<u8>) {
        code.push(0xC3);
    }

    /// SYSCALL
    pub fn syscall(code: &mut Vec<u8>) {
        code.extend_from_slice(&[0x0F, 0x05]);
    }

    /// NOP
    pub fn nop(code: &mut Vec<u8>) {
        code.push(0x90);
    }

    /// INT3 (breakpoint)
    pub fn int3(code: &mut Vec<u8>) {
        code.push(0xCC);
    }

    /// CALL rel32
    pub fn call_rel32(code: &mut Vec<u8>, offset: i32) {
        code.push(0xE8);
        code.extend_from_slice(&offset.to_le_bytes());
    }

    /// JMP rel32
    pub fn jmp_rel32(code: &mut Vec<u8>, offset: i32) {
        code.push(0xE9);
        code.extend_from_slice(&offset.to_le_bytes());
    }

    /// JMP rel8
    pub fn jmp_rel8(code: &mut Vec<u8>, offset: i8) {
        code.push(0xEB);
        code.push(offset as u8);
    }

    /// ADD reg64, imm32
    pub fn add_reg_imm32(code: &mut Vec<u8>, reg: u8, imm: i32) {
        let rex = if reg >= 8 { 0x49 } else { 0x48 };
        code.push(rex);
        code.push(0x81);
        code.push(0xC0 + (reg & 0x07));
        code.extend_from_slice(&imm.to_le_bytes());
    }

    /// SUB reg64, imm32
    pub fn sub_reg_imm32(code: &mut Vec<u8>, reg: u8, imm: i32) {
        let rex = if reg >= 8 { 0x49 } else { 0x48 };
        code.push(rex);
        code.push(0x81);
        code.push(0xE8 + (reg & 0x07));
        code.extend_from_slice(&imm.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_write() {
        let mut code = Vec::new();
        linux::emit_write(&mut code, linux::STDOUT, 0x400000, 14);
        assert!(!code.is_empty());
        // Verificar que termina con syscall
        assert_eq!(&code[code.len() - 2..], &[0x0F, 0x05]);
    }

    #[test]
    fn test_linux_exit() {
        let mut code = Vec::new();
        linux::emit_exit(&mut code, 0);
        assert!(!code.is_empty());
        assert_eq!(&code[code.len() - 2..], &[0x0F, 0x05]);
    }

    #[test]
    fn test_mov_reg_imm64() {
        let mut code = Vec::new();
        opcodes::mov_reg_imm64(&mut code, opcodes::RAX, 0x12345678);
        assert_eq!(code[0], 0x48); // REX.W
        assert_eq!(code[1], 0xB8); // MOV RAX
    }
}
