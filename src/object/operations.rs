use super::*;
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
        if core::ptr::eq(self.type_object(), other.type_object()) {
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
