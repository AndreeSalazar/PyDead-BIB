// PE Generator simplificado usando librería object para generar PE válido
// Esto genera un PE funcional más fácilmente

use object::write::{Object, StandardSegment, Symbol, SymbolSection};
use object::{Architecture, BinaryFormat, Endianness, SymbolFlags, SymbolKind, SymbolScope};

pub fn generate_pe_simple(opcodes: &[u8], data: &[u8], output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    let mut obj = Object::new(BinaryFormat::Pe, Architecture::X86_64, Endianness::Little);
    
    // Agregar símbolo _start
    let text_section = obj.add_section(
        StandardSegment::Text.into(),
        b".text".to_vec(),
        object::SectionKind::Text,
    );
    
    // Escribir opcodes en .text
    obj.append_section_data(text_section, opcodes, 16);
    
    // Agregar .data si hay datos
    if !data.is_empty() {
        let data_section = obj.add_section(
            StandardSegment::Data.into(),
            b".data".to_vec(),
            object::SectionKind::Data,
        );
        obj.append_section_data(data_section, data, 16);
    }
    
    // Escribir archivo
    let bytes = obj.write()?;
    fs::write(output_path, bytes)?;
    
    Ok(())
}

