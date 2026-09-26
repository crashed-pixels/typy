//! Hygienic name resolution: imports expose only selected names, while linked
//! definitions keep canonical globals and nominal class identities.
use crate::{
    ast::{Expr, FunctionDef, Parameter, Stmt},
    types::Type,
};
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    format,
    rc::Rc,
    string::{String, ToString},
    vec,
    vec::Vec,
};

pub(super) type Namespace = BTreeMap<String, Binding>;
#[derive(Clone)]
pub(super) enum Binding {
    Value(String),
    Module(Rc<Namespace>),
}
impl Binding {
    pub(super) fn value(&self) -> Result<String, String> {
        match self {
            Self::Value(name) => Ok(name.clone()),
            Self::Module(_) => Err("TypeError: a module namespace is not a runtime value".into()),
        }
    }
}
#[derive(Clone)]
pub(super) struct Names {
    module: String,
    scopes: Vec<Namespace>,
    imports: BTreeSet<String>,
}
impl Names {
    pub(super) fn new(module: &str) -> Self {
        Self {
            module: module.to_string(),
            scopes: vec![BTreeMap::new()],
            imports: BTreeSet::new(),
        }
    }
    pub(super) fn exports(self) -> Namespace {
        self.scopes.into_iter().next().unwrap_or_default()
    }
    pub(super) fn import(&mut self, alias: String, binding: Binding) -> Result<(), String> {
        self.insert(&alias, binding)?;
        self.imports.insert(alias);
        Ok(())
    }
    fn insert(&mut self, name: &str, binding: Binding) -> Result<(), String> {
        let scope = self
            .scopes
            .last_mut()
            .ok_or("SystemError: missing name scope")?;
        if scope.contains_key(name) {
            return Err(format!(
                "NameError: duplicate declaration or import '{name}'"
            ));
        }
        scope.insert(name.to_string(), binding);
        Ok(())
    }
    fn declare(&mut self, name: &str) -> Result<String, String> {
        let canonical = if self.scopes.len() == 1 {
            format!("{}::{name}", self.module)
        } else {
            name.to_string()
        };
        self.insert(name, Binding::Value(canonical.clone()))?;
        Ok(canonical)
    }
    fn find(&self, name: &str) -> Option<(usize, &Binding)> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, scope)| scope.get(name).map(|binding| (i, binding)))
    }
    fn lookup(&self, name: &str) -> Result<Binding, String> {
        self.find(name)
            .map(|(_, binding)| binding.clone())
            .or_else(|| crate::builtins::lookup(name).map(|_| Binding::Value(name.to_string())))
            .ok_or_else(|| format!("NameError: name '{name}' is not defined"))
    }
    /// A syntactic namespace chain is resolved before ordinary instance attributes.
    fn namespace_member(&self, expr: &Expr) -> Result<Option<Binding>, String> {
        match expr {
            Expr::Name(name) => Ok(Some(self.lookup(name)?)),
            Expr::Attribute { object, name } => {
                if let Some(Binding::Module(exports)) = self.namespace_member(object)? {
                    return exports
                        .get(name)
                        .cloned()
                        .map(Some)
                        .ok_or_else(|| format!("ImportError: module does not export '{name}'"));
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    fn expression(&self, expr: &Expr) -> Result<Expr, String> {
        if let Some(binding) = self.namespace_member(expr)? {
            return Ok(Expr::Name(binding.value()?));
        }
        Ok(match expr {
            Expr::Call { callee, arguments } => Expr::Call {
                callee: Box::new(self.expression(callee)?),
                arguments: arguments
                    .iter()
                    .map(|value| self.expression(value))
                    .collect::<Result<_, _>>()?,
            },
            Expr::Attribute { object, name } => Expr::Attribute {
                object: Box::new(self.expression(object)?),
                name: name.clone(),
            },
            Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
                left: Box::new(self.expression(left)?),
                op: *op,
                right: Box::new(self.expression(right)?),
            },
            _ => expr.clone(),
        })
    }
    fn annotation(&self, typ: &Type) -> Result<Type, String> {
        let Type::Named(path) = typ else {
            return Ok(typ.clone());
        };
        let mut parts = path.split('.');
        let first = parts.next().ok_or("TypeError: empty annotation")?;
        let mut expr = Expr::Name(first.to_string());
        for name in parts {
            expr = Expr::Attribute {
                object: Box::new(expr),
                name: name.to_string(),
            };
        }
        match self.namespace_member(&expr)? {
            Some(Binding::Value(name)) => Ok(Type::Named(name)),
            _ => Err(format!("TypeError: '{path}' is not a type")),
        }
    }
    fn function(&self, function: &FunctionDef, name: String) -> Result<FunctionDef, String> {
        let return_type = self.annotation(&function.return_type)?;
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| {
                Ok(Parameter {
                    name: parameter.name.clone(),
                    typ: parameter
                        .typ
                        .as_ref()
                        .map(|typ| self.annotation(typ))
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut scope = self.clone();
        scope.scopes.push(BTreeMap::new());
        for parameter in &parameters {
            scope.declare(&parameter.name)?;
        }
        let body = function
            .body
            .iter()
            .map(|stmt| scope.statement(stmt))
            .collect::<Result<_, _>>()?;
        Ok(FunctionDef {
            name,
            parameters,
            return_type,
            body,
        })
    }
    fn block(&self, body: &[Stmt]) -> Result<Vec<Stmt>, String> {
        let mut scope = self.clone();
        scope.scopes.push(BTreeMap::new());
        body.iter().map(|stmt| scope.statement(stmt)).collect()
    }
    pub(super) fn statement(&mut self, stmt: &Stmt) -> Result<Stmt, String> {
        Ok(match stmt {
            Stmt::Import { .. } | Stmt::FromImport { .. } => {
                return Err("ImportError: imports are allowed only at module scope".into());
            }
            Stmt::Function(function) => {
                if self.scopes.len() != 1 {
                    return Err("TypeError: function definitions require module scope".into());
                }
                let name = self.declare(&function.name)?;
                Stmt::Function(self.function(function, name)?)
            }
            Stmt::Class { name, body } => {
                if self.scopes.len() != 1 {
                    return Err("TypeError: class definitions require module scope".into());
                }
                if matches!(name.as_str(), "int" | "bool") {
                    return Err("TypeError: cannot redefine a builtin type".into());
                }
                let name = self.declare(name)?;
                let body = body
                    .iter()
                    .map(|member| match member {
                        Stmt::Function(function) => self
                            .function(function, function.name.clone())
                            .map(Stmt::Function),
                        Stmt::VariableDecl {
                            name,
                            typ,
                            initializer,
                        } => Ok(Stmt::VariableDecl {
                            name: name.clone(),
                            typ: self.annotation(typ)?,
                            initializer: initializer
                                .as_ref()
                                .map(|expr| self.expression(expr))
                                .transpose()?,
                        }),
                        Stmt::Pass => Ok(Stmt::Pass),
                        _ => Err("TypeError: expected field or method".into()),
                    })
                    .collect::<Result<_, String>>()?;
                Stmt::Class { name, body }
            }
            Stmt::VariableDecl {
                name,
                typ,
                initializer,
            } => {
                let typ = self.annotation(typ)?;
                let initializer = initializer
                    .as_ref()
                    .map(|value| self.expression(value))
                    .transpose()?;
                let name = self.declare(name)?;
                Stmt::VariableDecl {
                    name,
                    typ,
                    initializer,
                }
            }
            Stmt::Assign { name, value } => {
                let value = self.expression(value)?;
                let name = if let Some((scope, binding)) = self.find(name) {
                    if self.scopes.len() == 1 {
                        return Err("EntryError: module assignments must declare a new binding; move mutations into main".into());
                    }
                    if scope == 0 && self.imports.contains(name) {
                        return Err(format!(
                            "ImportError: imported binding '{name}' is read-only"
                        ));
                    }
                    binding.value()?
                } else {
                    self.declare(name)?
                };
                Stmt::Assign { name, value }
            }
            Stmt::SetAttribute {
                object,
                name,
                value,
            } => {
                if matches!(self.namespace_member(object)?, Some(Binding::Module(_))) {
                    return Err("ImportError: imported module bindings are read-only".into());
                }
                Stmt::SetAttribute {
                    object: self.expression(object)?,
                    name: name.clone(),
                    value: self.expression(value)?,
                }
            }
            Stmt::Return(value) => Stmt::Return(
                value
                    .as_ref()
                    .map(|value| self.expression(value))
                    .transpose()?,
            ),
            Stmt::Expr(value) => Stmt::Expr(self.expression(value)?),
            Stmt::Pass => Stmt::Pass,
            Stmt::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            } => Stmt::If {
                condition: self.expression(condition)?,
                then_branch: self.block(then_branch)?,
                elif_branches: elif_branches
                    .iter()
                    .map(|(expr, body)| Ok((self.expression(expr)?, self.block(body)?)))
                    .collect::<Result<_, String>>()?,
                else_branch: else_branch
                    .as_ref()
                    .map(|body| self.block(body))
                    .transpose()?,
            },
        })
    }
}
