use alloc::{
    format,
    rc::Rc,
    string::{String, ToString},
};
use core::fmt;

/// Common object header. Reference counts are owned safely by `Rc`, not raw fields.
/// Type identity cannot be changed independently from an object's payload.
pub struct ObjectHeader {
    typ: &'static TypeObject,
}

impl ObjectHeader {
    pub fn type_object(&self) -> &'static TypeObject {
        self.typ
    }
}

/// A TyPy type object. Built-in descriptors are immutable and statically allocated.
/// Like instances, type objects have a header pointing at their metatype.
/// This is the metadata foundation for classes, not an implementation of MRO,
/// dynamic attributes, or user-defined heap types.
pub struct TypeObject {
    header: ObjectHeader,
    name: &'static str,
    base: Option<&'static TypeObject>,
}

impl TypeObject {
    pub fn header(&self) -> &ObjectHeader {
        &self.header
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn base(&self) -> Option<&'static TypeObject> {
        self.base
    }
}

impl fmt::Debug for TypeObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeObject")
            .field("name", &self.name)
            .finish()
    }
}

/// The metatype is its own type. Static references need neither atomics nor locks.
pub static TYPE_TYPE: TypeObject = TypeObject {
    header: ObjectHeader { typ: &TYPE_TYPE },
    name: "type",
    base: Some(&OBJECT_TYPE),
};
pub static OBJECT_TYPE: TypeObject = TypeObject {
    header: ObjectHeader { typ: &TYPE_TYPE },
    name: "object",
    base: None,
};
pub static INT_TYPE: TypeObject = TypeObject {
    header: ObjectHeader { typ: &TYPE_TYPE },
    name: "int",
    base: Some(&OBJECT_TYPE),
};
// TyPy deliberately keeps bool separate from int, preserving static semantics.
pub static BOOL_TYPE: TypeObject = TypeObject {
    header: ObjectHeader { typ: &TYPE_TYPE },
    name: "bool",
    base: Some(&OBJECT_TYPE),
};
pub static NONE_TYPE: TypeObject = TypeObject {
    header: ObjectHeader { typ: &TYPE_TYPE },
    name: "NoneType",
    base: Some(&OBJECT_TYPE),
};

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

#[derive(PartialEq)]
enum Payload {
    Int(IntObject),
    Bool(BoolObject),
    None(NoneObject),
}

struct ObjectData {
    header: ObjectHeader,
    payload: Payload,
}

/// An owning handle to an immutable TyPy object.
/// Cloning shares identity and increments a non-atomic reference count. Dropping
/// the final handle releases the allocation. No unsafe casts or GC are needed for
/// the current acyclic payloads. Handles are intentionally neither Send nor Sync.
#[derive(Clone)]
pub struct Object(Rc<ObjectData>);

impl Object {
    fn new(typ: &'static TypeObject, payload: Payload) -> Self {
        Self(Rc::new(ObjectData {
            header: ObjectHeader { typ },
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
    pub fn type_object(&self) -> &'static TypeObject {
        self.header().type_object()
    }
    pub fn type_name(&self) -> &'static str {
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
        core::ptr::eq(self.type_object(), other.type_object()) && self.0.payload == other.0.payload
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.payload {
            Payload::Int(value) => write!(f, "{}", value.value()),
            Payload::Bool(value) => f.write_str(if value.value() { "True" } else { "False" }),
            Payload::None(_) => f.write_str("None"),
        }
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.payload {
            Payload::Int(value) => f.debug_tuple("Int").field(&value.value()).finish(),
            Payload::Bool(value) => f.debug_tuple("Bool").field(&value.value()).finish(),
            Payload::None(_) => f.write_str("None"),
        }
    }
}

impl Object {
    /// Performs addition on two objects.
    ///
    /// Returns an error if either operand is not an integer.
    pub fn add(&self, other: &Object) -> Result<Object, String> {
        self.arithmetic_op("+", other, i64::checked_add)
    }

    /// Performs subtraction on two objects.
    ///
    /// Returns an error if either operand is not an integer.
    pub fn sub(&self, other: &Object) -> Result<Object, String> {
        self.arithmetic_op("-", other, i64::checked_sub)
    }

    /// Performs multiplication on two objects.
    ///
    /// Returns an error if either operand is not an integer.
    pub fn mul(&self, other: &Object) -> Result<Object, String> {
        self.arithmetic_op("*", other, i64::checked_mul)
    }

    /// Performs division on two objects.
    ///
    /// Returns an error if either operand is not an integer, or if the
    /// divisor is zero.
    pub fn div(&self, other: &Object) -> Result<Object, String> {
        match (self.as_int(), other.as_int()) {
            (Some(_), Some(b)) => {
                if b == 0 {
                    Err("ZeroDivisionError: division by zero".to_string())
                } else {
                    self.arithmetic_op("/", other, i64::checked_div)
                }
            }
            _ => Err(Self::binary_op_error("/", self, other)),
        }
    }

    /// Performs equality comparison on two objects.
    ///
    /// Returns an error if the operands have incompatible types.
    pub fn eq(&self, other: &Object) -> Result<bool, String> {
        self.comparison_op("==", other, |a, b| a == b)
    }

    /// Performs less-than comparison on two objects.
    ///
    /// Returns an error if either operand is not an integer.
    pub fn lt(&self, other: &Object) -> Result<bool, String> {
        self.int_comparison_op("<", other, |a, b| a < b)
    }

    /// Performs greater-than comparison on two objects.
    ///
    /// Returns an error if either operand is not an integer.
    pub fn gt(&self, other: &Object) -> Result<bool, String> {
        self.int_comparison_op(">", other, |a, b| a > b)
    }

    /// Performs less-than-or-equal comparison on two objects.
    ///
    /// Returns an error if either operand is not an integer.
    pub fn le(&self, other: &Object) -> Result<bool, String> {
        self.int_comparison_op("<=", other, |a, b| a <= b)
    }

    /// Performs greater-than-or-equal comparison on two objects.
    ///
    /// Returns an error if either operand is not an integer.
    pub fn ge(&self, other: &Object) -> Result<bool, String> {
        self.int_comparison_op(">=", other, |a, b| a >= b)
    }

    /// Performs inequality comparison on two objects.
    ///
    /// Returns an error if the operands have incompatible types.
    pub fn ne(&self, other: &Object) -> Result<bool, String> {
        self.comparison_op("!=", other, |a, b| a != b)
    }

    /// Helper for arithmetic operations that require both operands to be integers.
    ///
    /// This eliminates duplication across add, sub, and mul methods.
    fn arithmetic_op<F>(&self, operator: &str, other: &Object, op: F) -> Result<Object, String>
    where
        F: FnOnce(i64, i64) -> Option<i64>,
    {
        match (self.as_int(), other.as_int()) {
            (Some(a), Some(b)) => op(a, b)
                .map(Object::int)
                .ok_or_else(|| format!("OverflowError: integer overflow in '{}'", operator)),
            _ => Err(Self::binary_op_error(operator, self, other)),
        }
    }

    /// Helper for comparison operations that require both operands to be integers.
    ///
    /// This eliminates duplication across lt, gt, le, and ge methods.
    fn int_comparison_op<F>(&self, operator: &str, other: &Object, op: F) -> Result<bool, String>
    where
        F: FnOnce(i64, i64) -> bool,
    {
        match (self.as_int(), other.as_int()) {
            (Some(a), Some(b)) => Ok(op(a, b)),
            _ => Err(Self::binary_op_error(operator, self, other)),
        }
    }

    /// Helper for comparison operations that work on matching types.
    ///
    /// This eliminates duplication across eq and ne methods.
    fn comparison_op<F>(&self, operator: &str, other: &Object, op: F) -> Result<bool, String>
    where
        F: FnOnce(&Object, &Object) -> bool,
    {
        if core::ptr::eq(self.type_object(), other.type_object()) && !self.is_none() {
            Ok(op(self, other))
        } else {
            Err(Self::binary_op_error(operator, self, other))
        }
    }

    /// Formats a binary operation type error message.
    ///
    /// This follows Python's error message format for unsupported operations.
    fn binary_op_error(operator: &str, left: &Object, right: &Object) -> String {
        format!(
            "TypeError: '{}' not supported between instances of '{}' and '{}'",
            operator,
            left.type_name(),
            right.type_name()
        )
    }
}
