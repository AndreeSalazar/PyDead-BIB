// ============================================================
// ADead-BIB — Flat Binary Generator
// ============================================================
// Genera binarios planos (sin headers PE/ELF) directamente
// desde código máquina. Esencial para:
//   - Boot sectors (512 bytes, firma 0x55AA)
//   - Bootloaders
//   - Bare-metal kernels
//   - ROM images
//
// Pipeline: AST → ADeadIR → Encoder → FlatBinaryGenerator → .bin
//
// Autor: Eddi Andreé Salazar Matos
// Email: eddi.salazar.dev@gmail.com
// ============================================================

use std::fs;
use std::io;
use std::path::Path;

/// Generador de binarios planos para ADead-BIB.
///
/// Produce archivos binarios sin ningún header de formato ejecutable.
/// El código se coloca directamente en la dirección de origen especificada.
pub struct FlatBinaryGenerator {
    /// Dirección de origen del binario (ej: 0x7C00 para boot sector)
    origin: u64,
    /// Bytes de código generados
    code: Vec<u8>,
    /// Bytes de datos
    data: Vec<u8>,
    /// Tamaño total fijo (0 = sin límite)
    fixed_size: usize,
}

impl FlatBinaryGenerator {
    /// Crea un nuevo generador con dirección de origen.
    ///
    /// # Arguments
    /// * `origin` - Dirección base donde se cargará el binario
    ///
    /// # Examples
    /// ```ignore
    /// let gen = FlatBinaryGenerator::new(0x7C00); // Boot sector
    /// let gen = FlatBinaryGenerator::new(0x0000); // ROM image
    /// ```
    pub fn new(origin: u64) -> Self {
        Self {
            origin,
            code: Vec::new(),
            data: Vec::new(),
            fixed_size: 0,
        }
    }

    /// Retorna la dirección de origen configurada.
    pub fn origin(&self) -> u64 {
        self.origin
    }

    /// Establece un tamaño fijo para el binario.
    /// El output será paddeado con ceros hasta alcanzar este tamaño.
    pub fn set_fixed_size(&mut self, size: usize) {
        self.fixed_size = size;
    }

    /// Genera un binario plano a partir de código y datos.
    ///
    /// El resultado es: [código][datos][padding si hay tamaño fijo]
    pub fn generate(&mut self, code: &[u8], data: &[u8]) -> Vec<u8> {
        self.code = code.to_vec();
        self.data = data.to_vec();

        let mut output = Vec::new();
        output.extend_from_slice(&self.code);
        output.extend_from_slice(&self.data);

        // Aplicar tamaño fijo si está configurado
        if self.fixed_size > 0 {
            if output.len() < self.fixed_size {
                output.resize(self.fixed_size, 0x00);
            } else if output.len() > self.fixed_size {
                output.truncate(self.fixed_size);
            }
        }

        output
    }

    /// Genera un boot sector válido de exactamente 512 bytes.
    ///
    /// Estructura del boot sector:
    /// ```text
    /// Offset  | Contenido
    /// --------|----------------------------------
    /// 0x000   | Código del boot sector
    /// ...     | (padded con 0x00 hasta byte 509)
    /// 0x1FE   | 0x55 (byte bajo de la firma)
    /// 0x1FF   | 0xAA (byte alto de la firma)
    /// ```
    ///
    /// La firma 0x55AA en los últimos 2 bytes indica al BIOS
    /// que este sector es bootable.
    pub fn generate_boot_sector(&mut self, code: &[u8]) -> Vec<u8> {
        const BOOT_SECTOR_SIZE: usize = 512;
        const SIGNATURE_OFFSET: usize = BOOT_SECTOR_SIZE - 2;

        let mut sector = vec![0u8; BOOT_SECTOR_SIZE];

        // Copiar código (máximo 510 bytes para dejar espacio a la firma)
        let code_len = code.len().min(SIGNATURE_OFFSET);
        sector[..code_len].copy_from_slice(&code[..code_len]);

        // Escribir firma de boot sector
        sector[SIGNATURE_OFFSET] = 0x55;
        sector[SIGNATURE_OFFSET + 1] = 0xAA;

        if code.len() > SIGNATURE_OFFSET {
            eprintln!(
                "⚠️  ADead-BIB: Boot sector code ({} bytes) exceeds 510 byte limit. Truncated.",
                code.len()
            );
        }

        sector
    }

    /// Genera un binario plano y lo escribe a disco.
    pub fn write_to_file(&mut self, path: &Path, code: &[u8], data: &[u8]) -> io::Result<()> {
        let binary = self.generate(code, data);
        fs::write(path, &binary)?;
        println!(
            "✅ Flat binary written: {} ({} bytes)",
            path.display(),
            binary.len()
        );
        Ok(())
    }

    /// Genera un boot sector y lo escribe a disco.
    pub fn write_boot_sector(&mut self, path: &Path, code: &[u8]) -> io::Result<()> {
        let sector = self.generate_boot_sector(code);
        fs::write(path, &sector)?;
        println!(
            "✅ Boot sector written: {} (512 bytes, signature: 0x{:02X}{:02X})",
            path.display(),
            sector[510],
            sector[511]
        );
        Ok(())
    }

    /// Genera un binario con secciones múltiples.
    ///
    /// Útil para bootloaders que necesitan código + datos en posiciones específicas.
    pub fn generate_sectioned(
        &mut self,
        sections: &[(&str, &[u8], u64)], // (nombre, datos, offset_relativo)
    ) -> Vec<u8> {
        // Calcular tamaño total necesario
        let total_size = sections
            .iter()
            .map(|(_, data, offset)| *offset as usize + data.len())
            .max()
            .unwrap_or(0);

        let size = if self.fixed_size > 0 {
            self.fixed_size.max(total_size)
        } else {
            total_size
        };

        let mut output = vec![0u8; size];

        for (name, data, offset) in sections {
            let off = *offset as usize;
            let end = (off + data.len()).min(output.len());
            let copy_len = end - off;
            output[off..end].copy_from_slice(&data[..copy_len]);
            println!(
                "  📦 Section '{}': {} bytes at offset 0x{:04X}",
                name,
                data.len(),
                off
            );
        }

        output
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_sector_size_and_signature() {
        let mut gen = FlatBinaryGenerator::new(0x7C00);
        // Simple infinite loop: jmp short -2 (EB FE)
        let code = vec![0xEB, 0xFE];
        let sector = gen.generate_boot_sector(&code);

        assert_eq!(sector.len(), 512);
        assert_eq!(sector[0], 0xEB);
        assert_eq!(sector[1], 0xFE);
        assert_eq!(sector[510], 0x55);
        assert_eq!(sector[511], 0xAA);
        // Padding should be zeros
        assert_eq!(sector[2], 0x00);
        assert_eq!(sector[509], 0x00);
    }

    #[test]
    fn test_flat_binary_no_headers() {
        let mut gen = FlatBinaryGenerator::new(0x0000);
        let code = vec![0xFA, 0xFB, 0xF4]; // cli; sti; hlt
        let data = vec![];
        let binary = gen.generate(&code, &data);

        // Should be EXACTLY the code bytes, no headers
        assert_eq!(binary, vec![0xFA, 0xFB, 0xF4]);
    }

    #[test]
    fn test_flat_binary_fixed_size() {
        let mut gen = FlatBinaryGenerator::new(0x0000);
        gen.set_fixed_size(16);
        let code = vec![0x90, 0x90]; // nop; nop
        let binary = gen.generate(&code, &[]);

        assert_eq!(binary.len(), 16);
        assert_eq!(binary[0], 0x90);
        assert_eq!(binary[1], 0x90);
        assert_eq!(binary[2], 0x00); // padding
    }

    #[test]
    fn test_boot_sector_max_code() {
        let mut gen = FlatBinaryGenerator::new(0x7C00);
        // 510 bytes of NOPs (maximum allowed)
        let code = vec![0x90; 510];
        let sector = gen.generate_boot_sector(&code);

        assert_eq!(sector.len(), 512);
        assert_eq!(sector[509], 0x90);
        assert_eq!(sector[510], 0x55);
        assert_eq!(sector[511], 0xAA);
    }

    #[test]
    fn test_sectioned_binary() {
        let mut gen = FlatBinaryGenerator::new(0x0000);
        gen.set_fixed_size(32);
        let sections = vec![
            ("code", &[0xEB, 0xFE][..], 0u64),
            ("data", &[0xDE, 0xAD][..], 16u64),
        ];
        let binary = gen.generate_sectioned(&sections);

        assert_eq!(binary.len(), 32);
        assert_eq!(binary[0], 0xEB);
        assert_eq!(binary[1], 0xFE);
        assert_eq!(binary[16], 0xDE);
        assert_eq!(binary[17], 0xAD);
    }

    #[test]
    fn test_origin() {
        let gen = FlatBinaryGenerator::new(0x7C00);
        assert_eq!(gen.origin(), 0x7C00);
    }
}
