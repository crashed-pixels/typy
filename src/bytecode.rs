use crate::compiler::Instruction;
use alloc::{collections::BTreeMap, rc::Rc, string::String, vec::Vec};

/// A function's immutable code and fixed local layout. Calls allocate VM frames.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCode {
    pub name: String,
    pub arity: usize,
    pub locals: usize,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassCode {
    pub name: String,
    pub fields: Vec<String>,
    pub methods: BTreeMap<String, Rc<FunctionCode>>,
}
