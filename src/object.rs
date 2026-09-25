use std::fmt;

/// A runtime value in the interpreter.
///
/// This enum represents all possible values that can exist at runtime.
/// Currently supports integers, booleans, and None.
///
/// Future extensions may add floats, strings, lists, dictionaries,
/// functions, or user-defined objects.
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    /// A 64-bit signed integer.
    Int(i64),
    /// A boolean value.
    Bool(bool),
    /// The absence of a value.
    None,
}

impl Object {
    /// Returns the type name of this object as it appears in error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Object::Int(_) => "int",
            Object::Bool(_) => "bool",
            Object::None => "NoneType",
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::Int(n) => write!(f, "{}", n),
            Object::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Object::None => write!(f, "None"),
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
        match (self, other) {
            (Object::Int(_), Object::Int(b)) => {
                if *b == 0 {
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
        match (self, other) {
            (Object::Int(a), Object::Int(b)) => op(*a, *b)
                .map(Object::Int)
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
        match (self, other) {
            (Object::Int(a), Object::Int(b)) => Ok(op(*a, *b)),
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
        match (self, other) {
            (Object::Int(_), Object::Int(_)) => Ok(op(self, other)),
            (Object::Bool(_), Object::Bool(_)) => Ok(op(self, other)),
            _ => Err(Self::binary_op_error(operator, self, other)),
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
