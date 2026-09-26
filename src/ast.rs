use crate::types::Type;
use alloc::{boxed::Box, string::String, vec::Vec};

/// An expression node in the abstract syntax tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    None,
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Attribute {
        object: Box<Expr>,
        name: String,
    },
    /// An integer literal.
    Number(i64),

    /// A boolean literal.
    Bool(bool),

    /// A variable reference by name.
    Name(String),

    /// A binary operation combining two subexpressions.
    BinaryOp {
        /// The left operand.
        left: Box<Expr>,
        /// The operator.
        op: Operator,
        /// The right operand.
        right: Box<Expr>,
    },
}

/// A statement node in the abstract syntax tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Function(FunctionDef),
    Class {
        name: String,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    SetAttribute {
        object: Expr,
        name: String,
        value: Expr,
    },
    Pass,
    /// An expression used as a statement.
    Expr(Expr),

    /// A variable declaration with an optional initializer.
    ///
    /// Syntax: `name: type [= initializer]`
    VariableDecl {
        /// The variable name.
        name: String,
        /// The declared type.
        typ: Type,
        /// An optional initial value.
        initializer: Option<Expr>,
    },

    /// An assignment to an existing variable.
    ///
    /// Syntax: `name = value`
    Assign {
        /// The variable name.
        name: String,
        /// The value to assign.
        value: Expr,
    },

    /// A conditional statement with optional elif and else branches.
    ///
    /// Syntax:
    /// ```text
    /// if condition:
    ///     statements
    /// elif condition:
    ///     statements
    /// else:
    ///     statements
    /// ```
    If {
        /// The condition for the main if branch.
        condition: Expr,
        /// The statements to execute if the condition is true.
        then_branch: Vec<Stmt>,
        /// Zero or more elif branches, each with a condition and statements.
        elif_branches: Vec<(Expr, Vec<Stmt>)>,
        /// An optional else branch.
        else_branch: Option<Vec<Stmt>>,
    },
}

/// A binary operator.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Operator {
    /// Addition: `+`
    Plus,
    /// Subtraction: `-`
    Minus,
    /// Multiplication: `*`
    Star,
    /// Division: `/`
    Slash,
    /// Equality: `==`
    Eq,
    /// Inequality: `!=`
    NotEq,
    /// Less than: `<`
    Less,
    /// Greater than: `>`
    Greater,
    /// Less than or equal: `<=`
    LessEq,
    /// Greater than or equal: `>=`
    GreaterEq,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    /// Only a method receiver may omit its annotation.
    pub typ: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Stmt>,
}
