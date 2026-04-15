use super::{PyTypeInferencer, TypeEnv};
use std::collections::HashMap;
use super::concrete::ConcreteType;
use crate::frontend::python::ast::*;

impl PyTypeInferencer {
    pub fn infer_expr(&self, expr: &PyExpr) -> ConcreteType {
        match expr {
            PyExpr::IntLiteral(_) => ConcreteType::Int64,
            PyExpr::FloatLiteral(_) => ConcreteType::Float64,
            PyExpr::BoolLiteral(_) => ConcreteType::Bool,
            PyExpr::StringLiteral(_) | PyExpr::FString { .. } => ConcreteType::Str,
            PyExpr::BytesLiteral(_) => ConcreteType::Bytes,
            PyExpr::NoneLiteral => ConcreteType::NoneType,
            PyExpr::Name(name) => {
                self.lookup_var(name).unwrap_or(ConcreteType::Dynamic)
            }
            PyExpr::BinOp { op, left, right } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                self.infer_binop(op, &lt, &rt)
            }
            PyExpr::UnaryOp { op: _, operand } => self.infer_expr(operand),
            PyExpr::BoolOp { .. } => ConcreteType::Bool,
            PyExpr::Compare { .. } => ConcreteType::Bool,
            PyExpr::Call { func, .. } => {
                if let PyExpr::Name(name) = func.as_ref() {
                    self.lookup_function_return(name).unwrap_or(ConcreteType::Dynamic)
                } else {
                    ConcreteType::Dynamic
                }
            }
            PyExpr::List(elts) => {
                let elem_type = elts.first()
                    .map(|e| self.infer_expr(e))
                    .unwrap_or(ConcreteType::Dynamic);
                ConcreteType::List(Box::new(elem_type))
            }
            PyExpr::Dict { keys, values } => {
                let kt = keys.first().and_then(|k| k.as_ref())
                    .map(|k| self.infer_expr(k))
                    .unwrap_or(ConcreteType::Dynamic);
                let vt = values.first()
                    .map(|v| self.infer_expr(v))
                    .unwrap_or(ConcreteType::Dynamic);
                ConcreteType::Dict(Box::new(kt), Box::new(vt))
            }
            PyExpr::Tuple(elts) => {
                let types: Vec<ConcreteType> = elts.iter().map(|e| self.infer_expr(e)).collect();
                ConcreteType::Tuple(types)
            }
            PyExpr::Subscript { value, .. } => {
                let vt = self.infer_expr(value);
                match vt {
                    ConcreteType::List(inner) => *inner,
                    ConcreteType::Dict(_, val) => *val,
                    ConcreteType::Str => ConcreteType::Str,
                    _ => ConcreteType::Dynamic,
                }
            }
            PyExpr::Attribute { .. } => ConcreteType::Dynamic,
            _ => ConcreteType::Dynamic,
        }
    }

    pub fn infer_binop(&self, op: &PyBinOp, left: &ConcreteType, right: &ConcreteType) -> ConcreteType {
        match (left, right) {
            (ConcreteType::Int64, ConcreteType::Int64) => match op {
                PyBinOp::Div => ConcreteType::Float64,
                _ => ConcreteType::Int64,
            },
            (ConcreteType::Float64, _) | (_, ConcreteType::Float64) => ConcreteType::Float64,
            (ConcreteType::Str, ConcreteType::Str) if *op == PyBinOp::Add => ConcreteType::Str,
            (ConcreteType::Str, ConcreteType::Int64) if *op == PyBinOp::Mul => ConcreteType::Str,
            (ConcreteType::List(_), ConcreteType::List(_)) if *op == PyBinOp::Add => left.clone(),
            _ => ConcreteType::Dynamic,
        }
    }

    pub fn annotation_to_concrete(&self, ann: &PyType) -> ConcreteType {
        match ann {
            PyType::Int => ConcreteType::Int64,
            PyType::Float => ConcreteType::Float64,
            PyType::Str => ConcreteType::Str,
            PyType::Bool => ConcreteType::Bool,
            PyType::None => ConcreteType::NoneType,
            PyType::Bytes => ConcreteType::Bytes,
            PyType::List(inner) => ConcreteType::List(Box::new(self.annotation_to_concrete(inner))),
            PyType::Dict(k, v) => ConcreteType::Dict(
                Box::new(self.annotation_to_concrete(k)),
                Box::new(self.annotation_to_concrete(v)),
            ),
            PyType::Set(inner) => ConcreteType::Set(Box::new(self.annotation_to_concrete(inner))),
            PyType::Tuple(elts) => ConcreteType::Tuple(elts.iter().map(|e| self.annotation_to_concrete(e)).collect()),
            PyType::Optional(inner) => self.annotation_to_concrete(inner),
            PyType::Any | PyType::Inferred => ConcreteType::Dynamic,
            PyType::Custom(name) => ConcreteType::Object(name.clone()),
            _ => ConcreteType::Dynamic,
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<ConcreteType> {
        for env in self.env_stack.iter().rev() {
            if let Some(t) = env.bindings.get(name) {
                return Some(t.clone());
            }
        }
        std::option::Option::None
    }

    pub fn lookup_function_return(&self, name: &str) -> Option<ConcreteType> {
        for env in self.env_stack.iter().rev() {
            if let Some(ConcreteType::Function { ret, .. }) = env.functions.get(name) {
                return Some(*ret.clone());
            }
        }
        std::option::Option::None
    }

    pub fn current_env_mut(&mut self) -> &mut TypeEnv {
        self.env_stack.last_mut().unwrap()
    }
}

