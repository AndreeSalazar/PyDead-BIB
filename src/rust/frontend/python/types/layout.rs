use super::concrete::ConcreteType;
// ══════════════════════════════════════════════════════════
// v4.0 — FASE 3: StructLayout & Deep Type Inference
// ══════════════════════════════════════════════════════════

/// Field in a struct layout — name, type, byte offset
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: ConcreteType,
    pub byte_offset: usize,
}

/// Struct layout for a class — ordered fields with offsets
#[derive(Debug, Clone)]
pub struct StructLayout {
    pub class_name: String,
    pub parent: Option<String>,
    pub fields: Vec<StructField>,
    pub total_size: usize,
    pub dynamic_warnings: Vec<String>,
}

impl StructLayout {
    pub fn new(name: &str) -> Self {
        Self {
            class_name: name.to_string(),
            parent: None,
            fields: Vec::new(),
            total_size: 8, // 8 bytes for class_id at offset 0
            dynamic_warnings: Vec::new(),
        }
    }

    /// Add a field to the layout, computing byte offset
    pub fn add_field(&mut self, name: &str, field_type: ConcreteType) {
        // Skip if field already exists (from parent)
        if self.fields.iter().any(|f| f.name == name) {
            return;
        }
        let offset = self.total_size;
        self.fields.push(StructField {
            name: name.to_string(),
            field_type: field_type.clone(),
            byte_offset: offset,
        });
        let field_size = match &field_type {
            ConcreteType::Int64 | ConcreteType::Float64 | ConcreteType::Str
            | ConcreteType::Bool | ConcreteType::NoneType => 8,
            ConcreteType::Object(_) => 8, // pointer
            ConcreteType::List(_) | ConcreteType::Dict(_, _) => 8, // pointer
            ConcreteType::Dynamic => 8, // pointer-sized fallback
            _ => 8,
        };
        self.total_size += field_size;
    }

    /// Get field offset by name
    pub fn field_offset(&self, name: &str) -> Option<usize> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.byte_offset)
    }

    /// Get field type by name
    pub fn field_type(&self, name: &str) -> Option<&ConcreteType> {
        self.fields.iter().find(|f| f.name == name).map(|f| &f.field_type)
    }
}
