// ============================================================
// ADead-BIB IR Function
// ============================================================
// A function contains basic blocks organized as a CFG
// Inspired by LLVM Function
// ============================================================

use super::basicblock::BasicBlockId;
use super::{BasicBlock, Type, ValueId};
use std::collections::HashMap;
use std::fmt;

/// Function linkage type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    /// Externally visible
    External,
    /// Internal to module
    Internal,
    /// Available externally (inline)
    AvailableExternally,
    /// Weak linkage
    Weak,
    /// Link once (discardable)
    LinkOnce,
    /// Private (not visible outside module)
    Private,
}

impl Default for Linkage {
    fn default() -> Self {
        Linkage::External
    }
}

impl fmt::Display for Linkage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Linkage::External => "",
            Linkage::Internal => "internal ",
            Linkage::AvailableExternally => "available_externally ",
            Linkage::Weak => "weak ",
            Linkage::LinkOnce => "linkonce ",
            Linkage::Private => "private ",
        };
        write!(f, "{}", name)
    }
}

/// Calling convention
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallingConv {
    /// C calling convention (default)
    C,
    /// Fast call (register-based)
    Fast,
    /// Cold (rarely called)
    Cold,
    /// Windows x64
    Win64,
    /// System V AMD64
    SysV64,
    /// Preserve all registers
    PreserveAll,
}

impl Default for CallingConv {
    fn default() -> Self {
        CallingConv::C
    }
}

impl fmt::Display for CallingConv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            CallingConv::C => "",
            CallingConv::Fast => "fastcc ",
            CallingConv::Cold => "coldcc ",
            CallingConv::Win64 => "win64cc ",
            CallingConv::SysV64 => "x86_64_sysvcc ",
            CallingConv::PreserveAll => "preserve_allcc ",
        };
        write!(f, "{}", name)
    }
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub ty: Type,
    /// Parameter index
    pub index: usize,
}

impl Parameter {
    pub fn new(name: &str, ty: Type, index: usize) -> Self {
        Parameter {
            name: name.to_string(),
            ty,
            index,
        }
    }
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} %{}", self.ty, self.name)
    }
}

/// Function attributes
#[derive(Debug, Clone, Default)]
pub struct FunctionAttrs {
    /// No unwind
    pub nounwind: bool,
    /// Read-only (no memory writes)
    pub readonly: bool,
    /// No return
    pub noreturn: bool,
    /// Always inline
    pub alwaysinline: bool,
    /// Never inline
    pub noinline: bool,
    /// Optimize for size
    pub optsize: bool,
    /// Naked function (no prologue/epilogue)
    pub naked: bool,
}

/// IR Function
#[derive(Debug, Clone)]
pub struct Function {
    /// Function name
    pub name: String,

    /// Return type
    pub return_type: Type,

    /// Parameters
    pub params: Vec<Parameter>,

    /// Is variadic
    pub variadic: bool,

    /// Linkage
    pub linkage: Linkage,

    /// Calling convention
    pub calling_conv: CallingConv,

    /// Attributes
    pub attrs: FunctionAttrs,

    /// Basic blocks (CFG)
    pub blocks: Vec<BasicBlock>,

    /// Block name to ID mapping
    block_map: HashMap<String, BasicBlockId>,

    /// Next value ID
    next_value_id: u32,

    /// Next block ID
    next_block_id: u32,

    /// Is declaration only (no body)
    pub is_declaration: bool,
}

impl Function {
    pub fn new(name: &str, return_type: Type) -> Self {
        Function {
            name: name.to_string(),
            return_type,
            params: Vec::new(),
            variadic: false,
            linkage: Linkage::default(),
            calling_conv: CallingConv::default(),
            attrs: FunctionAttrs::default(),
            blocks: Vec::new(),
            block_map: HashMap::new(),
            next_value_id: 0,
            next_block_id: 0,
            is_declaration: false,
        }
    }

    /// Create a declaration (no body)
    pub fn declaration(
        name: &str,
        return_type: Type,
        params: Vec<(String, Type)>,
        variadic: bool,
    ) -> Self {
        let mut func = Self::new(name, return_type);
        func.is_declaration = true;
        func.variadic = variadic;
        for (i, (pname, ty)) in params.into_iter().enumerate() {
            func.params.push(Parameter::new(&pname, ty, i));
        }
        func
    }

    /// Add a parameter
    pub fn add_param(&mut self, name: &str, ty: Type) -> usize {
        let index = self.params.len();
        self.params.push(Parameter::new(name, ty, index));
        index
    }

    /// Create a new basic block
    pub fn create_block(&mut self, name: Option<&str>) -> BasicBlockId {
        let id = BasicBlockId(self.next_block_id);
        self.next_block_id += 1;

        let mut block = BasicBlock::new(id.0);
        if let Some(n) = name {
            block = block.with_name(n);
            self.block_map.insert(n.to_string(), id);
        }

        self.blocks.push(block);
        id
    }

    /// Get entry block
    pub fn entry_block(&self) -> Option<&BasicBlock> {
        self.blocks.first()
    }

    /// Get entry block mutable
    pub fn entry_block_mut(&mut self) -> Option<&mut BasicBlock> {
        self.blocks.first_mut()
    }

    /// Get block by ID
    pub fn get_block(&self, id: BasicBlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

    /// Get block by ID mutable
    pub fn get_block_mut(&mut self, id: BasicBlockId) -> Option<&mut BasicBlock> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }

    /// Get block by name
    pub fn get_block_by_name(&self, name: &str) -> Option<&BasicBlock> {
        self.block_map.get(name).and_then(|id| self.get_block(*id))
    }

    /// Allocate a new value ID
    pub fn new_value_id(&mut self) -> ValueId {
        let id = ValueId(self.next_value_id);
        self.next_value_id += 1;
        id
    }

    /// Get function type
    pub fn get_type(&self) -> Type {
        Type::function(
            self.return_type.clone(),
            self.params.iter().map(|p| p.ty.clone()).collect(),
            self.variadic,
        )
    }

    /// Check if function has body
    pub fn has_body(&self) -> bool {
        !self.is_declaration && !self.blocks.is_empty()
    }

    /// Get number of blocks
    pub fn num_blocks(&self) -> usize {
        self.blocks.len()
    }

    /// Iterate over blocks
    pub fn iter_blocks(&self) -> impl Iterator<Item = &BasicBlock> {
        self.blocks.iter()
    }

    /// Iterate mutably over blocks
    pub fn iter_blocks_mut(&mut self) -> impl Iterator<Item = &mut BasicBlock> {
        self.blocks.iter_mut()
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Declaration or definition
        if self.is_declaration {
            write!(f, "declare ")?;
        } else {
            write!(f, "define ")?;
        }

        // Linkage
        write!(f, "{}", self.linkage)?;

        // Calling convention
        write!(f, "{}", self.calling_conv)?;

        // Return type and name
        write!(f, "{} @{}(", self.return_type, self.name)?;

        // Parameters
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?
            }
            write!(f, "{}", param)?;
        }
        if self.variadic {
            if !self.params.is_empty() {
                write!(f, ", ")?
            }
            write!(f, "...")?;
        }
        write!(f, ")")?;

        // Attributes
        if self.attrs.nounwind {
            write!(f, " nounwind")?
        }
        if self.attrs.readonly {
            write!(f, " readonly")?
        }
        if self.attrs.noreturn {
            write!(f, " noreturn")?
        }
        if self.attrs.alwaysinline {
            write!(f, " alwaysinline")?
        }
        if self.attrs.noinline {
            write!(f, " noinline")?
        }

        // Body
        if self.is_declaration {
            writeln!(f)?;
        } else {
            writeln!(f, " {{")?;
            for block in &self.blocks {
                write!(f, "{}", block)?;
            }
            writeln!(f, "}}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Constant, Instruction, Value};
    use super::*;

    #[test]
    fn test_function_creation() {
        let mut func = Function::new("main", Type::I32);
        func.add_param("argc", Type::I32);
        func.add_param("argv", Type::ptr(Type::ptr(Type::I8)));

        assert_eq!(func.name, "main");
        assert_eq!(func.params.len(), 2);
    }

    #[test]
    fn test_function_blocks() {
        let mut func = Function::new("test", Type::Void);
        let entry = func.create_block(Some("entry"));
        let loop_bb = func.create_block(Some("loop"));

        assert_eq!(func.num_blocks(), 2);
        assert!(func.get_block(entry).is_some());
        assert!(func.get_block_by_name("loop").is_some());

        // Add instructions
        if let Some(block) = func.get_block_mut(entry) {
            block.push(Instruction::br(loop_bb.0));
        }
    }

    #[test]
    fn test_function_display() {
        let mut func = Function::new("add", Type::I32);
        func.add_param("a", Type::I32);
        func.add_param("b", Type::I32);

        let entry = func.create_block(Some("entry"));
        if let Some(block) = func.get_block_mut(entry) {
            block.push(Instruction::ret(Some(Value::Constant(Constant::i32(0)))));
        }

        let output = format!("{}", func);
        assert!(output.contains("define"));
        assert!(output.contains("@add"));
        assert!(output.contains("i32 %a"));
    }
}
