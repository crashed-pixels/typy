use crate::object::{
    BOOL_TYPE, FUNCTION_TYPE, INT_TYPE, NONE_TYPE, OBJECT_TYPE, TYPE_TYPE, TypeObject,
};
use alloc::{boxed::Box, string::String, vec::Vec};

/// Nominal instance types and structural callable signatures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Builtin(crate::builtins::Builtin),
    Int,
    Bool,
    None,
    Named(String),
    Function(Signature),
    Constructor(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub parameters: Vec<Type>,
    pub result: Box<Type>,
}

impl Type {
    pub fn name(&self) -> &str {
        match self {
            Self::Builtin(_) => "builtin_function_or_method",
            Self::Int => "int",
            Self::Bool => "bool",
            Self::None => "NoneType",
            Self::Named(name) => name,
            Self::Function(_) => "function",
            Self::Constructor(_) => "type",
        }
    }
    /// Builtin category; user instances carry their precise descriptor at runtime.
    pub fn type_object(&self) -> &'static TypeObject {
        match self {
            Self::Builtin(_) => &crate::object::BUILTIN_FUNCTION_TYPE,
            Self::Int => &INT_TYPE,
            Self::Bool => &BOOL_TYPE,
            Self::None => &NONE_TYPE,
            Self::Named(_) => &OBJECT_TYPE,
            Self::Function(_) => &FUNCTION_TYPE,
            Self::Constructor(_) => &TYPE_TYPE,
        }
    }
}
