mod definitions;
mod value;
use crate::parser::{Expr, Operator, Stmt};
use crate::symbol::{Interner, SymbolId};
use alloc::{collections::BTreeMap, format, string::String, vec, vec::Vec};
use definitions::ClassInfo;
pub use value::{Signature, Type};

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
    scopes: Vec<BTreeMap<SymbolId, Type>>,
    classes: BTreeMap<String, ClassInfo>,
    return_type: Option<Type>,
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
            scopes: vec![BTreeMap::new()],
            classes: BTreeMap::new(),
            return_type: None,
        }
    }

    /// Returns a mutable reference to the current (innermost) scope.
    ///
    /// # Panics
    ///
    /// Panics if the scope stack is empty, which should never happen in
    /// practice since the global scope is always present.
    fn current_scope(&mut self) -> &mut BTreeMap<SymbolId, Type> {
        self.scopes
            .last_mut()
            .expect("scope stack should never be empty")
    }

    /// Enters a new nested scope.
    ///
    /// This is called when entering a block (e.g., the body of an if statement).
    fn enter_block(&mut self) {
        self.scopes.push(BTreeMap::new());
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
        self.check_and_resolve(stmts, interner).map(|_| ())
    }

    /// Checks transactionally and lowers first assignments into typed declarations.
    /// Compile the returned AST, so inferred block locals get frame slots rather
    /// than global stores. `check` alone is intended only for validation.
    pub fn check_and_resolve(
        &mut self,
        stmts: &[Stmt],
        interner: &mut Interner,
    ) -> Result<Vec<Stmt>, String> {
        let mut candidate = self.clone();
        let resolved = stmts
            .iter()
            .map(|stmt| candidate.check_stmt(stmt, interner))
            .collect::<Result<Vec<_>, _>>()?;
        *self = candidate;
        Ok(resolved)
    }

    fn check_stmt(&mut self, stmt: &Stmt, interner: &mut Interner) -> Result<Stmt, String> {
        match stmt {
            Stmt::Import { .. } | Stmt::FromImport { .. } => {
                return Err("ImportError: imports require a module loader and module scope".into());
            }
            Stmt::Function(function) => return self.check_function_definition(function, interner),
            Stmt::Class { name, body } => return self.check_class(name, body, interner),
            Stmt::Pass => {}
            Stmt::Return(value) => {
                let expected = self
                    .return_type
                    .clone()
                    .ok_or("TypeError: return outside a function")?;
                let actual = match value {
                    Some(expr) => self.check_expr(expr, interner)?,
                    None => Type::None,
                };
                if expected != actual {
                    return Err(format!(
                        "TypeError: return expected '{}', got '{}'",
                        expected.name(),
                        actual.name()
                    ));
                }
            }
            Stmt::SetAttribute {
                object,
                name,
                value,
            } => {
                let owner = self.check_expr(object, interner)?;
                let expected = self.field_type(&owner, name)?;
                let actual = self.check_expr(value, interner)?;
                if expected != actual {
                    return Err(format!(
                        "TypeError: field '{}' expected '{}', got '{}'",
                        name,
                        expected.name(),
                        actual.name()
                    ));
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, interner)?;
            }
            Stmt::VariableDecl {
                name,
                typ,
                initializer,
            } => {
                self.validate_type(typ)?;
                let sym_id = interner.intern(name);
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
                if initializer.is_none() && !matches!(typ, Type::Int | Type::Bool | Type::None) {
                    return Err(format!("TypeError: '{}' requires an initializer", name));
                }
                self.declare(sym_id, typ.clone(), interner)?;
            }
            Stmt::Assign { name, value } => {
                // Infer the RHS before introducing the binding: `x = x + 1`
                // cannot read a new, uninitialized x.
                let typ = self.check_expr(value, interner)?;
                let sym_id = interner.intern(name);
                if let Some(expected) = self.resolve(sym_id) {
                    if *expected != typ {
                        return Err(format!(
                            "TypeError: mismatched types '{}' expected to be '{}', but got '{}'",
                            name,
                            expected.name(),
                            typ.name()
                        ));
                    }
                } else {
                    self.declare(sym_id, typ.clone(), interner)?;
                    return Ok(Stmt::VariableDecl {
                        name: name.clone(),
                        typ,
                        initializer: Some(value.clone()),
                    });
                }
            }
            Stmt::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.check_condition(condition, "if", interner)?;
                let then_branch = self.check_block(then_branch, interner)?;
                let mut resolved_elifs = Vec::new();
                for (condition, branch) in elif_branches {
                    self.check_condition(condition, "elif", interner)?;
                    resolved_elifs.push((condition.clone(), self.check_block(branch, interner)?));
                }
                let else_branch = else_branch
                    .as_ref()
                    .map(|branch| self.check_block(branch, interner))
                    .transpose()?;
                return Ok(Stmt::If {
                    condition: condition.clone(),
                    then_branch,
                    elif_branches: resolved_elifs,
                    else_branch,
                });
            }
        }
        Ok(stmt.clone())
    }

    fn check_condition(
        &mut self,
        condition: &Expr,
        keyword: &str,
        interner: &mut Interner,
    ) -> Result<(), String> {
        let typ = self.check_expr(condition, interner)?;
        if typ != Type::Bool {
            return Err(format!(
                "TypeError: {} condition must be 'bool', got '{}'",
                keyword,
                typ.name()
            ));
        }
        Ok(())
    }

    fn check_block(
        &mut self,
        stmts: &[Stmt],
        interner: &mut Interner,
    ) -> Result<Vec<Stmt>, String> {
        self.enter_block();
        let result = stmts
            .iter()
            .map(|stmt| self.check_stmt(stmt, interner))
            .collect();
        self.exit_block();
        result
    }

    /// Type-checks an expression and returns its type.
    fn check_expr(&mut self, expr: &Expr, interner: &mut Interner) -> Result<Type, String> {
        match expr {
            Expr::None => Ok(Type::None),
            Expr::Attribute { object, name } => {
                let owner = self.check_expr(object, interner)?;
                self.attribute_type(&owner, name)
            }
            Expr::Call { callee, arguments } => {
                let typ = self.check_expr(callee, interner)?;
                if let Type::Builtin(builtin) = typ {
                    for argument in arguments {
                        self.check_expr(argument, interner)?;
                    }
                    return Ok(builtin.result_type());
                }
                let signature = self.call_signature(&typ)?;
                if signature.parameters.len() != arguments.len() {
                    return Err(format!(
                        "TypeError: expected {} arguments, got {}",
                        signature.parameters.len(),
                        arguments.len()
                    ));
                }
                for (argument, expected) in arguments.iter().zip(&signature.parameters) {
                    let actual = self.check_expr(argument, interner)?;
                    if &actual != expected {
                        return Err(format!(
                            "TypeError: argument expected '{}', got '{}'",
                            expected.name(),
                            actual.name()
                        ));
                    }
                }
                Ok(*signature.result)
            }
            Expr::Number(_) => Ok(Type::Int),
            Expr::Bool(_) => Ok(Type::Bool),

            Expr::Name(name) => {
                let sym_id = interner.intern(name);
                self.resolve(sym_id)
                    .cloned()
                    .or_else(|| crate::builtins::lookup(name).map(Type::Builtin))
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
