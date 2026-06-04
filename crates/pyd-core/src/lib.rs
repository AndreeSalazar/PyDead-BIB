//! Tipos fundamentales enfocados 100% en Tensores e IA.

#[derive(Debug, Clone, PartialEq)]
pub enum TensorDType {
    Float32,
}

#[derive(Debug, Clone)]
pub struct TensorMeta {
    pub name: String,
    pub shape: Vec<usize>,
    pub dtype: TensorDType,
}

/// Instrucciones de nuestro IR (Intermediate Representation)
#[derive(Debug)]
pub enum Instruction {
    /// Crea un tensor y reserva su espacio en la GPU
    CreateTensor { meta: TensorMeta },
    /// Multiplicación de matrices: C = A @ B
    MatMul { dest_name: String, lhs_name: String, rhs_name: String },
}