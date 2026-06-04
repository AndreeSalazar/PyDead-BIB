//! Tipos fundamentales compartidos por todos los módulos.

/// Representa cualquier objeto en tiempo de ejecución en PyDead-BIB.
#[derive(Debug, Clone)]
pub enum PyObject {
    Int(i64),
    Float(f64),
    Tensor(Tensor),
}

#[derive(Debug, Clone)]
pub struct Tensor {
    pub id: usize,
    pub shape: Vec<usize>,
    pub dtype: TensorDType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TensorDType {
    Float32,
}

/// Instrucciones de nuestro IR (Intermediate Representation)
#[derive(Debug)]
pub enum Instruction {
    CreateTensor { dest: usize, shape: Vec<usize> },
    MatMul { dest: usize, lhs: usize, rhs: usize },
}
