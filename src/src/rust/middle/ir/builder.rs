// ============================================================
// ADead-BIB IR Builder
// ============================================================
// Helper for constructing IR - Inspired by LLVM IRBuilder
// Provides convenient methods for creating instructions
// ============================================================

use super::basicblock::BasicBlockId;
use super::{
    BasicBlock, BinaryOp, CastOp, CompareOp, Constant, Function, Instruction, Module, Type, Value,
    ValueId,
};

/// IR Builder - Helper for constructing IR
pub struct IRBuilder<'a> {
    /// Current module
    module: &'a mut Module,

    /// Current function index
    current_function: Option<usize>,

    /// Current block ID
    current_block: Option<BasicBlockId>,

    /// Insertion point within block
    insert_point: Option<usize>,
}

impl<'a> IRBuilder<'a> {
    pub fn new(module: &'a mut Module) -> Self {
        IRBuilder {
            module,
            current_function: None,
            current_block: None,
            insert_point: None,
        }
    }

    // ============================================================
    // Position management
    // ============================================================

    /// Set current function
    pub fn set_function(&mut self, name: &str) -> bool {
        // Find function by name
        for (idx, func) in self.module.functions.iter().enumerate() {
            if func.name == name {
                self.current_function = Some(idx);
                self.current_block = None;
                self.insert_point = None;
                return true;
            }
        }
        false
    }

    /// Set current block
    pub fn set_block(&mut self, block_id: BasicBlockId) {
        self.current_block = Some(block_id);
        self.insert_point = None; // Insert at end
    }

    /// Set insertion point
    pub fn set_insert_point(&mut self, index: usize) {
        self.insert_point = Some(index);
    }

    /// Get current function
    fn get_current_function(&mut self) -> Option<&mut Function> {
        self.current_function.map(|i| &mut self.module.functions[i])
    }

    /// Get current block
    fn get_current_block(&mut self) -> Option<&mut BasicBlock> {
        let func_idx = self.current_function?;
        let block_id = self.current_block?;
        self.module.functions[func_idx]
            .blocks
            .iter_mut()
            .find(|b| b.id == block_id)
    }

    /// Allocate a new value ID
    fn new_value_id(&mut self) -> ValueId {
        let func_idx = self.current_function.expect("No current function");
        self.module.functions[func_idx].new_value_id()
    }

    /// Insert instruction at current position
    fn insert(&mut self, inst: Instruction) {
        let func_idx = match self.current_function {
            Some(idx) => idx,
            None => return,
        };
        let block_id = match self.current_block {
            Some(id) => id,
            None => return,
        };

        let block = match self.module.functions[func_idx]
            .blocks
            .iter_mut()
            .find(|b| b.id == block_id)
        {
            Some(b) => b,
            None => return,
        };

        if let Some(pos) = self.insert_point {
            block.insert(pos, inst);
            self.insert_point = Some(pos + 1);
        } else {
            block.push(inst);
        }
    }

    // ============================================================
    // Function/Block creation
    // ============================================================

    /// Create a new function
    pub fn create_function(&mut self, name: &str, return_type: Type) -> usize {
        let func = Function::new(name, return_type);
        let idx = self.module.add_function(func);
        self.current_function = Some(idx);
        idx
    }

    /// Add parameter to current function
    pub fn add_param(&mut self, name: &str, ty: Type) -> usize {
        if let Some(func) = self.get_current_function() {
            func.add_param(name, ty)
        } else {
            0
        }
    }

    /// Create a new basic block
    pub fn create_block(&mut self, name: Option<&str>) -> BasicBlockId {
        if let Some(func) = self.get_current_function() {
            func.create_block(name)
        } else {
            BasicBlockId(0)
        }
    }

    // ============================================================
    // Memory instructions
    // ============================================================

    /// Create alloca instruction
    pub fn build_alloca(&mut self, ty: Type, name: Option<&str>) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::alloca(ty, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create load instruction
    pub fn build_load(&mut self, ty: Type, ptr: Value) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::load(ty, ptr, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create store instruction
    pub fn build_store(&mut self, value: Value, ptr: Value) {
        let inst = Instruction::store(value, ptr);
        self.insert(inst);
    }

    /// Create GEP instruction
    pub fn build_gep(&mut self, base_ty: Type, ptr: Value, indices: Vec<Value>) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::gep(base_ty, ptr, indices, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    // ============================================================
    // Arithmetic instructions
    // ============================================================

    /// Create add instruction
    pub fn build_add(&mut self, lhs: Value, rhs: Value, ty: Type) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::add(ty, lhs, rhs, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create sub instruction
    pub fn build_sub(&mut self, lhs: Value, rhs: Value, ty: Type) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::sub(ty, lhs, rhs, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create mul instruction
    pub fn build_mul(&mut self, lhs: Value, rhs: Value, ty: Type) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::mul(ty, lhs, rhs, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create sdiv instruction
    pub fn build_sdiv(&mut self, lhs: Value, rhs: Value, ty: Type) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::sdiv(ty, lhs, rhs, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create generic binary instruction
    pub fn build_binary(&mut self, op: BinaryOp, lhs: Value, rhs: Value, ty: Type) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::binary(op, ty, lhs, rhs, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    // ============================================================
    // Comparison instructions
    // ============================================================

    /// Create icmp instruction
    pub fn build_icmp(&mut self, pred: CompareOp, lhs: Value, rhs: Value) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::icmp(pred, lhs, rhs, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create fcmp instruction
    pub fn build_fcmp(&mut self, pred: CompareOp, lhs: Value, rhs: Value) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::fcmp(pred, lhs, rhs, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    // ============================================================
    // Cast instructions
    // ============================================================

    /// Create cast instruction
    pub fn build_cast(&mut self, op: CastOp, value: Value, dest_ty: Type) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::cast(op, dest_ty, value, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create zext instruction
    pub fn build_zext(&mut self, value: Value, dest_ty: Type) -> Value {
        self.build_cast(CastOp::ZExt, value, dest_ty)
    }

    /// Create sext instruction
    pub fn build_sext(&mut self, value: Value, dest_ty: Type) -> Value {
        self.build_cast(CastOp::SExt, value, dest_ty)
    }

    /// Create trunc instruction
    pub fn build_trunc(&mut self, value: Value, dest_ty: Type) -> Value {
        self.build_cast(CastOp::Trunc, value, dest_ty)
    }

    /// Create bitcast instruction
    pub fn build_bitcast(&mut self, value: Value, dest_ty: Type) -> Value {
        self.build_cast(CastOp::Bitcast, value, dest_ty)
    }

    // ============================================================
    // Control flow instructions
    // ============================================================

    /// Create return instruction
    pub fn build_ret(&mut self, value: Option<Value>) {
        let inst = Instruction::ret(value);
        self.insert(inst);
    }

    /// Create void return
    pub fn build_ret_void(&mut self) {
        self.build_ret(None);
    }

    /// Create unconditional branch
    pub fn build_br(&mut self, target: BasicBlockId) {
        let inst = Instruction::br(target.0);
        self.insert(inst);
    }

    /// Create conditional branch
    pub fn build_cond_br(&mut self, cond: Value, true_bb: BasicBlockId, false_bb: BasicBlockId) {
        let inst = Instruction::cond_br(cond, true_bb.0, false_bb.0);
        self.insert(inst);
    }

    /// Create unreachable instruction
    pub fn build_unreachable(&mut self) {
        let inst = Instruction::unreachable();
        self.insert(inst);
    }

    // ============================================================
    // Call instructions
    // ============================================================

    /// Create call instruction
    pub fn build_call(&mut self, ret_ty: Type, name: &str, args: Vec<Value>) -> Option<Value> {
        let result = if ret_ty.is_void() {
            None
        } else {
            Some(self.new_value_id())
        };
        let inst = Instruction::call(ret_ty.clone(), name, args, result);
        self.insert(inst);
        result.map(Value::Instruction)
    }

    // ============================================================
    // Phi and Select
    // ============================================================

    /// Create phi instruction
    pub fn build_phi(&mut self, ty: Type, incoming: Vec<(Value, BasicBlockId)>) -> Value {
        let id = self.new_value_id();
        let incoming: Vec<(Value, u32)> = incoming.into_iter().map(|(v, b)| (v, b.0)).collect();
        let inst = Instruction::phi(ty, incoming, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    /// Create select instruction
    pub fn build_select(
        &mut self,
        cond: Value,
        true_val: Value,
        false_val: Value,
        ty: Type,
    ) -> Value {
        let id = self.new_value_id();
        let inst = Instruction::select(ty, cond, true_val, false_val, id);
        self.insert(inst);
        Value::Instruction(id)
    }

    // ============================================================
    // Constants
    // ============================================================

    /// Create i32 constant
    pub fn const_i32(&self, value: i32) -> Value {
        Value::Constant(Constant::i32(value))
    }

    /// Create i64 constant
    pub fn const_i64(&self, value: i64) -> Value {
        Value::Constant(Constant::i64(value))
    }

    /// Create f32 constant
    pub fn const_f32(&self, value: f32) -> Value {
        Value::Constant(Constant::f32(value))
    }

    /// Create f64 constant
    pub fn const_f64(&self, value: f64) -> Value {
        Value::Constant(Constant::f64(value))
    }

    /// Create bool constant
    pub fn const_bool(&self, value: bool) -> Value {
        Value::Constant(Constant::bool(value))
    }

    /// Create null pointer constant
    pub fn const_null(&self, pointee: Type) -> Value {
        Value::Constant(Constant::null(pointee))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let mut module = Module::new("test");
        let mut builder = IRBuilder::new(&mut module);

        // Create function
        builder.create_function("main", Type::I32);
        builder.add_param("argc", Type::I32);

        // Create entry block
        let entry = builder.create_block(Some("entry"));
        builder.set_block(entry);

        // Build instructions
        let x = builder.build_alloca(Type::I32, Some("x"));
        builder.build_store(builder.const_i32(42), x.clone());
        let loaded = builder.build_load(Type::I32, x);
        builder.build_ret(Some(loaded));

        // Verify
        assert_eq!(module.num_functions(), 1);
        let func = module.get_function("main").unwrap();
        assert_eq!(func.blocks.len(), 1);
        assert_eq!(func.blocks[0].instructions.len(), 4);
    }

    #[test]
    fn test_builder_arithmetic() {
        let mut module = Module::new("test");
        let mut builder = IRBuilder::new(&mut module);

        builder.create_function("add", Type::I32);
        let entry = builder.create_block(Some("entry"));
        builder.set_block(entry);

        let a = builder.const_i32(5);
        let b = builder.const_i32(3);
        let sum = builder.build_add(a, b, Type::I32);
        builder.build_ret(Some(sum));

        let func = module.get_function("add").unwrap();
        assert!(func.blocks[0].has_terminator());
    }

    #[test]
    fn test_builder_control_flow() {
        let mut module = Module::new("test");
        let mut builder = IRBuilder::new(&mut module);

        builder.create_function("test", Type::Void);
        let entry = builder.create_block(Some("entry"));
        let then_bb = builder.create_block(Some("then"));
        let else_bb = builder.create_block(Some("else"));
        let merge = builder.create_block(Some("merge"));

        // Entry block
        builder.set_block(entry);
        let cond = builder.const_bool(true);
        builder.build_cond_br(cond, then_bb, else_bb);

        // Then block
        builder.set_block(then_bb);
        builder.build_br(merge);

        // Else block
        builder.set_block(else_bb);
        builder.build_br(merge);

        // Merge block
        builder.set_block(merge);
        builder.build_ret_void();

        let func = module.get_function("test").unwrap();
        assert_eq!(func.blocks.len(), 4);
    }
}
