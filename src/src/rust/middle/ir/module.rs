// ============================================================
// ADead-BIB IR Module
// ============================================================
// Top-level container for IR - like LLVM Module
// Contains functions, globals, types, and metadata
// ============================================================

use super::{Function, Type};
use std::collections::HashMap;
use std::fmt;

/// Global variable
#[derive(Debug, Clone)]
pub struct GlobalVariable {
    #[allow(dead_code)]
    /// Variable name
    pub name: String,
    /// Type
    pub ty: Type,
    /// Initializer (if any)
    pub initializer: Option<Vec<u8>>,
    /// Is constant
    pub is_constant: bool,
    /// Alignment
    pub alignment: usize,
    /// Is external
    pub is_external: bool,
}

impl GlobalVariable {
    pub fn new(name: &str, ty: Type) -> Self {
        let alignment = ty.alignment();
        GlobalVariable {
            name: name.to_string(),
            ty,
            initializer: None,
            is_constant: false,
            alignment,
            is_external: false,
        }
    }

    pub fn constant(name: &str, ty: Type, init: Vec<u8>) -> Self {
        let alignment = ty.alignment();
        GlobalVariable {
            name: name.to_string(),
            ty,
            initializer: Some(init),
            is_constant: true,
            alignment,
            is_external: false,
        }
    }

    pub fn external(name: &str, ty: Type) -> Self {
        let alignment = ty.alignment();
        GlobalVariable {
            name: name.to_string(),
            ty,
            initializer: None,
            is_constant: false,
            alignment,
            is_external: true,
        }
    }
}

impl fmt::Display for GlobalVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_external {
            write!(f, "@{} = external global {}", self.name, self.ty)
        } else if self.is_constant {
            write!(f, "@{} = constant {}", self.name, self.ty)?;
            if let Some(init) = &self.initializer {
                write!(f, " [")?;
                for (i, byte) in init.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "0x{:02X}", byte)?;
                }
                write!(f, "]")?;
            }
            Ok(())
        } else {
            write!(f, "@{} = global {}", self.name, self.ty)?;
            if let Some(init) = &self.initializer {
                write!(f, " [")?;
                for (i, byte) in init.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "0x{:02X}", byte)?;
                }
                write!(f, "]")?;
            }
            Ok(())
        }
    }
}

/// Type alias
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub ty: Type,
}

/// IR Module - Top-level container
#[derive(Debug, Clone)]
pub struct Module {
    /// Module name
    pub name: String,

    /// Source filename
    pub source_filename: Option<String>,

    /// Target triple (e.g., "x86_64-pc-windows-msvc")
    pub target_triple: Option<String>,

    /// Data layout string
    pub data_layout: Option<String>,

    /// Functions
    pub functions: Vec<Function>,

    /// Global variables
    pub globals: Vec<GlobalVariable>,

    /// Type aliases
    pub type_aliases: Vec<TypeAlias>,

    /// String table (for string literals)
    pub strings: Vec<String>,

    /// Function name to index mapping
    function_map: HashMap<String, usize>,

    /// Global name to index mapping
    global_map: HashMap<String, usize>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Module {
            name: name.to_string(),
            source_filename: None,
            target_triple: None,
            data_layout: None,
            functions: Vec::new(),
            globals: Vec::new(),
            type_aliases: Vec::new(),
            strings: Vec::new(),
            function_map: HashMap::new(),
            global_map: HashMap::new(),
        }
    }

    /// Set target triple
    pub fn set_target(&mut self, triple: &str) {
        self.target_triple = Some(triple.to_string());
    }

    /// Set data layout
    pub fn set_data_layout(&mut self, layout: &str) {
        self.data_layout = Some(layout.to_string());
    }

    /// Add a function
    pub fn add_function(&mut self, func: Function) -> usize {
        let index = self.functions.len();
        self.function_map.insert(func.name.clone(), index);
        self.functions.push(func);
        index
    }

    /// Get function by name
    pub fn get_function(&self, name: &str) -> Option<&Function> {
        self.function_map.get(name).map(|&i| &self.functions[i])
    }

    /// Get function by name mutable
    pub fn get_function_mut(&mut self, name: &str) -> Option<&mut Function> {
        if let Some(&i) = self.function_map.get(name) {
            Some(&mut self.functions[i])
        } else {
            None
        }
    }

    /// Add a global variable
    pub fn add_global(&mut self, global: GlobalVariable) -> usize {
        let index = self.globals.len();
        self.global_map.insert(global.name.clone(), index);
        self.globals.push(global);
        index
    }

    /// Get global by name
    pub fn get_global(&self, name: &str) -> Option<&GlobalVariable> {
        self.global_map.get(name).map(|&i| &self.globals[i])
    }

    /// Add a string literal and return its index
    pub fn add_string(&mut self, s: &str) -> usize {
        // Check if string already exists
        if let Some(idx) = self.strings.iter().position(|x| x == s) {
            return idx;
        }
        let index = self.strings.len();
        self.strings.push(s.to_string());
        index
    }

    /// Get string by index
    pub fn get_string(&self, index: usize) -> Option<&str> {
        self.strings.get(index).map(|s| s.as_str())
    }

    /// Add a type alias
    pub fn add_type_alias(&mut self, name: &str, ty: Type) {
        self.type_aliases.push(TypeAlias {
            name: name.to_string(),
            ty,
        });
    }

    /// Iterate over functions
    pub fn iter_functions(&self) -> impl Iterator<Item = &Function> {
        self.functions.iter()
    }

    /// Iterate mutably over functions
    pub fn iter_functions_mut(&mut self) -> impl Iterator<Item = &mut Function> {
        self.functions.iter_mut()
    }

    /// Get number of functions
    pub fn num_functions(&self) -> usize {
        self.functions.len()
    }

    /// Get number of globals
    pub fn num_globals(&self) -> usize {
        self.globals.len()
    }

    /// Verify module integrity
    pub fn verify(&self) -> Result<(), String> {
        // Check all functions have terminators
        for func in &self.functions {
            if !func.is_declaration {
                for block in &func.blocks {
                    if !block.has_terminator() {
                        return Err(format!(
                            "Block '{}' in function '{}' has no terminator",
                            block.display_name(),
                            func.name
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Module header
        writeln!(f, "; ModuleID = '{}'", self.name)?;

        if let Some(src) = &self.source_filename {
            writeln!(f, "source_filename = \"{}\"", src)?;
        }

        if let Some(triple) = &self.target_triple {
            writeln!(f, "target triple = \"{}\"", triple)?;
        }

        if let Some(layout) = &self.data_layout {
            writeln!(f, "target datalayout = \"{}\"", layout)?;
        }

        writeln!(f)?;

        // Type aliases
        for alias in &self.type_aliases {
            writeln!(f, "%{} = type {}", alias.name, alias.ty)?;
        }
        if !self.type_aliases.is_empty() {
            writeln!(f)?;
        }

        // Global variables
        for global in &self.globals {
            writeln!(f, "{}", global)?;
        }
        if !self.globals.is_empty() {
            writeln!(f)?;
        }

        // String constants
        for (i, s) in self.strings.iter().enumerate() {
            writeln!(
                f,
                "@.str.{} = private constant [{} x i8] c\"{}\\00\"",
                i,
                s.len() + 1,
                s.escape_default()
            )?;
        }
        if !self.strings.is_empty() {
            writeln!(f)?;
        }

        // Functions
        for func in &self.functions {
            write!(f, "{}", func)?;
            writeln!(f)?;
        }

        Ok(())
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new("module")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let mut module = Module::new("test");
        module.set_target("x86_64-pc-windows-msvc");

        assert_eq!(module.name, "test");
        assert_eq!(
            module.target_triple,
            Some("x86_64-pc-windows-msvc".to_string())
        );
    }

    #[test]
    fn test_module_functions() {
        let mut module = Module::new("test");

        let func = Function::new("main", Type::I32);
        module.add_function(func);

        assert_eq!(module.num_functions(), 1);
        assert!(module.get_function("main").is_some());
    }

    #[test]
    fn test_module_globals() {
        let mut module = Module::new("test");

        let global = GlobalVariable::new("counter", Type::I32);
        module.add_global(global);

        assert_eq!(module.num_globals(), 1);
        assert!(module.get_global("counter").is_some());
    }

    #[test]
    fn test_module_strings() {
        let mut module = Module::new("test");

        let idx1 = module.add_string("Hello");
        let idx2 = module.add_string("World");
        let idx3 = module.add_string("Hello"); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Should return existing index
        assert_eq!(module.strings.len(), 2);
    }
}
