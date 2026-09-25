use crate::parser::{Expr, Operator, Stmt};
use crate::symbol::{Interner, SymbolId};
use std::collections::HashMap;

/// A type in the type system.
///
/// This enum represents the primitive types supported by the interpreter.
/// Future extensions may add floating point numbers, strings, functions,
/// or user-defined types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// A 64-bit signed integer.
    Int,
    /// A boolean value.
    Bool,
}

impl Type {
    /// Returns the name of this type as it appears in source code.
    ///
    /// This is used for error messages and type annotations.
    pub fn name(&self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::Bool => "bool",
        }
    }
}

/// A static type checker.
///
/// The type checker walks the abstract syntax tree and verifies that all
/// expressions and statements are well-typed. It maintains a stack of
/// scopes to support nested blocks and lexical scoping.
///
/// Variables must be declared before use, and assignments must match the
/// declared type.
#[derive(Clone)]
pub struct TypeChecker {
    /// A stack of lexical scopes, each mapping symbol IDs to types.
    /// The last element is the current (innermost) scope.
    scopes: Vec<HashMap<SymbolId, Type>>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    /// Creates a new type checker with an empty global scope.
    pub fn new() -> Self {
        TypeChecker {
            scopes: vec![HashMap::new()],
        }
    }

    /// Returns a mutable reference to the current (innermost) scope.
    ///
    /// # Panics
    ///
    /// Panics if the scope stack is empty, which should never happen in
    /// practice since the global scope is always present.
    fn current_scope(&mut self) -> &mut HashMap<SymbolId, Type> {
        self.scopes
            .last_mut()
            .expect("scope stack should never be empty")
    }

    /// Enters a new nested scope.
    ///
    /// This is called when entering a block (e.g., the body of an if statement).
    fn enter_block(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Exits the current scope and returns to the parent scope.
    ///
    /// This is called when leaving a block.
    fn exit_block(&mut self) {
        self.scopes.pop();
    }

    /// Resolves a symbol to its type by searching scopes from innermost to outermost.
    ///
    /// Returns `None` if the symbol is not declared in any accessible scope.
    fn resolve(&self, sym_id: SymbolId) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(typ) = scope.get(&sym_id) {
                return Some(typ);
            }
        }
        None
    }

    /// Declares a variable in the current scope.
    ///
    /// Returns an error if the variable is already declared in this scope.
    fn declare(&mut self, sym_id: SymbolId, typ: Type, interner: &Interner) -> Result<(), String> {
        let current = self.current_scope();
        if current.contains_key(&sym_id) {
            return Err(format!(
                "Variable '{}' already declared in this scope",
                interner.resolve(sym_id)
            ));
        }
        current.insert(sym_id, typ);
        Ok(())
    }

    /// Type-checks a list of statements.
    ///
    /// This is the main entry point for type checking. It processes each
    /// statement in order. On error, all declarations and scopes are unchanged.
    pub fn check(&mut self, stmts: &[Stmt], interner: &mut Interner) -> Result<(), String> {
        let mut candidate = self.clone();
        for stmt in stmts {
            candidate.check_stmt(stmt, interner)?;
        }
        *self = candidate;
        Ok(())
    }

    /// Type-checks a single statement.
    fn check_stmt(&mut self, stmt: &Stmt, interner: &mut Interner) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => {
                self.check_expr(expr, interner)?;
                Ok(())
            }

            Stmt::VariableDecl {
                name,
                typ,
                initializer,
            } => {
                let sym_id = interner.intern(name);

                // If there's an initializer, verify its type matches the declaration
                if let Some(init) = initializer {
                    let init_type = self.check_expr(init, interner)?;
                    if init_type != *typ {
                        return Err(format!(
                            "TypeError: mismatched types '{}' expected to be '{}', got '{}'",
                            name,
                            typ.name(),
                            init_type.name()
                        ));
                    }
                }

                self.declare(sym_id, *typ, interner)?;
                Ok(())
            }

            Stmt::Assign { name, value } => {
                let sym_id = interner.intern(name);

                // Look up the declared type
                let expected_type = match self.resolve(sym_id) {
                    Some(expected_type) => *expected_type,
                    None => return Err(format!("TypeError: variable '{}' not defined", name)),
                };

                // Verify the assigned value has the correct type
                let val_type = self.check_expr(value, interner)?;
                if expected_type != val_type {
                    return Err(format!(
                        "TypeError: mismatched types '{}' expected to be '{}', but got '{}'",
                        name,
                        expected_type.name(),
                        val_type.name()
                    ));
                }
                Ok(())
            }

            Stmt::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                // Check the main condition
                let cond_type = self.check_expr(condition, interner)?;
                if cond_type != Type::Bool {
                    return Err(format!(
                        "TypeError: if condition must be 'bool', got '{}'",
                        cond_type.name()
                    ));
                }

                // Check the then branch in a new scope
                self.check_block(then_branch, interner)?;

                // Check each elif branch
                for (elif_cond, elif_branch) in elif_branches {
                    let elif_cond_type = self.check_expr(elif_cond, interner)?;
                    if elif_cond_type != Type::Bool {
                        return Err(format!(
                            "TypeError: elif condition must be 'bool', got '{}'",
                            elif_cond_type.name()
                        ));
                    }

                    self.check_block(elif_branch, interner)?;
                }

                // Check the else branch if present
                if let Some(else_block) = else_branch {
                    self.check_block(else_block, interner)?;
                }

                Ok(())
            }
        }
    }

    /// Type-checks a block of statements in a new scope.
    ///
    /// This helper eliminates duplication when checking the bodies of
    /// if/elif/else branches.
    fn check_block(&mut self, stmts: &[Stmt], interner: &mut Interner) -> Result<(), String> {
        self.enter_block();
        let result = stmts
            .iter()
            .try_for_each(|stmt| self.check_stmt(stmt, interner));
        self.exit_block();
        result
    }

    /// Type-checks an expression and returns its type.
    fn check_expr(&mut self, expr: &Expr, interner: &mut Interner) -> Result<Type, String> {
        match expr {
            Expr::Number(_) => Ok(Type::Int),
            Expr::Bool(_) => Ok(Type::Bool),

            Expr::Name(name) => {
                let sym_id = interner.intern(name);
                self.resolve(sym_id)
                    .copied()
                    .ok_or_else(|| format!("NameError: name '{}' is not defined", name))
            }

            Expr::BinaryOp { left, op, right } => {
                let l_type = self.check_expr(left, interner)?;
                let r_type = self.check_expr(right, interner)?;

                self.check_binary_op(*op, l_type, r_type)
            }
        }
    }

    /// Type-checks a binary operation.
    ///
    /// Arithmetic operators require both operands to be integers and produce
    /// an integer result. Equality requires matching types; ordering requires
    /// integers. Both kinds of comparison produce a boolean result.
    fn check_binary_op(&self, op: Operator, l_type: Type, r_type: Type) -> Result<Type, String> {
        match op {
            // Arithmetic operators
            Operator::Plus | Operator::Minus | Operator::Star | Operator::Slash => {
                if l_type == Type::Int && r_type == Type::Int {
                    Ok(Type::Int)
                } else {
                    Err(format!(
                        "TypeError: unsupported operand types for arithmetic: '{}' and '{}'",
                        l_type.name(),
                        r_type.name()
                    ))
                }
            }

            // Comparison operators
            Operator::Eq
            | Operator::NotEq
            | Operator::Less
            | Operator::Greater
            | Operator::LessEq
            | Operator::GreaterEq => {
                let equality = matches!(op, Operator::Eq | Operator::NotEq);
                if l_type == r_type && (equality || l_type == Type::Int) {
                    Ok(Type::Bool)
                } else {
                    Err(format!(
                        "TypeError: cannot compare '{}' and '{}'",
                        l_type.name(),
                        r_type.name()
                    ))
                }
            }
        }
    }
}
