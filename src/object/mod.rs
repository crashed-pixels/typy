mod descriptors;
mod instances;
mod operations;
use crate::bytecode::FunctionCode;
use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
};
use core::fmt;
pub use descriptors::*;
pub(crate) use instances::Callable;
use instances::{BoundMethod, ClassObject, InstanceObject};

/// Immutable fixed-width integer payload. The host primitive is an implementation detail.
#[derive(Debug, PartialEq)]
pub struct IntObject {
    value: i64,
}
impl IntObject {
    pub fn value(&self) -> i64 {
        self.value
    }
}

#[derive(Debug, PartialEq)]
pub struct BoolObject {
    value: bool,
}
impl BoolObject {
    pub fn value(&self) -> bool {
        self.value
    }
}

#[derive(Debug, PartialEq)]
pub struct NoneObject;

enum Payload {
    Function(Rc<FunctionCode>),
    Class(Rc<ClassObject>),
    Instance(InstanceObject),
    BoundMethod(BoundMethod),
    Int(IntObject),
    Bool(BoolObject),
    None(NoneObject),
}

struct ObjectData {
    header: ObjectHeader,
    payload: Payload,
}

/// An owning handle to a TyPy object with immutable type identity.
/// Cloning shares identity and increments a non-atomic reference count. Dropping
/// the final handle releases the allocation. No unsafe casts or GC are needed for
/// the acyclic class-field graph. Instance field writes are journaled by the VM.
/// Handles are intentionally neither Send nor Sync.
#[derive(Clone)]
pub struct Object(Rc<ObjectData>);

impl Object {
    fn new(typ: &'static TypeObject, payload: Payload) -> Self {
        Self(Rc::new(ObjectData {
            header: ObjectHeader::builtin(typ),
            payload,
        }))
    }
    pub fn int(value: i64) -> Self {
        Self::new(&INT_TYPE, Payload::Int(IntObject { value }))
    }
    pub fn bool(value: bool) -> Self {
        Self::new(&BOOL_TYPE, Payload::Bool(BoolObject { value }))
    }
    pub fn none() -> Self {
        Self::new(&NONE_TYPE, Payload::None(NoneObject))
    }
    pub fn header(&self) -> &ObjectHeader {
        &self.0.header
    }
    pub fn type_object(&self) -> &TypeObject {
        self.header().type_object()
    }
    pub fn type_name(&self) -> &str {
        self.type_object().name()
    }
    pub fn as_int(&self) -> Option<i64> {
        match &self.0.payload {
            Payload::Int(value) => Some(value.value()),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match &self.0.payload {
            Payload::Bool(value) => Some(value.value()),
            _ => None,
        }
    }
    pub fn is_none(&self) -> bool {
        matches!(self.0.payload, Payload::None(_))
    }
    pub fn is_identical(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.0)
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0.payload, &other.0.payload) {
            (Payload::Int(a), Payload::Int(b)) => a == b,
            (Payload::Bool(a), Payload::Bool(b)) => a == b,
            (Payload::None(_), Payload::None(_)) => true,
            _ => self.is_identical(other),
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.payload {
            Payload::Int(value) => write!(f, "{}", value.value()),
            Payload::Bool(value) => f.write_str(if value.value() { "True" } else { "False" }),
            Payload::None(_) => f.write_str("None"),
            Payload::Function(code) => write!(f, "<function {}>", code.name),
            Payload::Class(class) => write!(f, "<class {}>", class.code.name),
            Payload::Instance(instance) => write!(f, "<{} instance>", instance.class.code.name),
            Payload::BoundMethod(method) => write!(f, "<bound method {}>", method.function.name),
        }
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.payload {
            Payload::Int(value) => f.debug_tuple("Int").field(&value.value()).finish(),
            Payload::Bool(value) => f.debug_tuple("Bool").field(&value.value()).finish(),
            Payload::None(_) => f.write_str("None"),
            Payload::Function(code) => write!(f, "<function {}>", code.name),
            Payload::Class(class) => write!(f, "<class {}>", class.code.name),
            Payload::Instance(instance) => write!(f, "<{} instance>", instance.class.code.name),
            Payload::BoundMethod(method) => write!(f, "<bound method {}>", method.function.name),
        }
    }
}
