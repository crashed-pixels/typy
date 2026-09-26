use super::*;
use crate::ast::{FunctionDef, Parameter};
use alloc::{boxed::Box, collections::BTreeSet, string::ToString};

#[derive(Clone)]
pub(super) struct ClassInfo {
    fields: BTreeMap<String, Type>,
    methods: BTreeMap<String, Signature>,
}

impl TypeChecker {
    pub(super) fn validate_type(&self, typ: &Type) -> Result<(), String> {
        match typ {
            Type::Int | Type::Bool | Type::None => Ok(()),
            Type::Named(name) if self.classes.contains_key(name) => Ok(()),
            _ => Err(format!("TypeError: unknown annotation '{}'", typ.name())),
        }
    }

    fn signature(&self, function: &FunctionDef, owner: Option<&str>) -> Result<Signature, String> {
        self.validate_type(&function.return_type)?;
        let mut names = BTreeSet::new();
        let mut parameters = Vec::new();
        for (index, parameter) in function.parameters.iter().enumerate() {
            if !names.insert(&parameter.name) {
                return Err(format!(
                    "TypeError: duplicate parameter '{}'",
                    parameter.name
                ));
            }
            if index == 0
                && let Some(class) = owner
            {
                if parameter.name != "self"
                    || parameter
                        .typ
                        .as_ref()
                        .is_some_and(|typ| typ != &Type::Named(class.to_string()))
                {
                    return Err("TypeError: a method's first parameter must be self".to_string());
                }
                parameters.push(Type::Named(class.to_string()));
            } else {
                let typ = parameter.typ.as_ref().ok_or_else(|| {
                    format!(
                        "TypeError: parameter '{}' needs an annotation",
                        parameter.name
                    )
                })?;
                self.validate_type(typ)?;
                parameters.push(typ.clone());
            }
        }
        if owner.is_some() && parameters.is_empty() {
            return Err("TypeError: method requires self".to_string());
        }
        Ok(Signature {
            parameters,
            result: Box::new(function.return_type.clone()),
        })
    }

    fn check_function_body(
        &mut self,
        function: &FunctionDef,
        signature: &Signature,
        interner: &mut Interner,
    ) -> Result<FunctionDef, String> {
        let saved_scopes = self.scopes.clone();
        let saved_return = self.return_type.replace(function.return_type.clone());
        self.enter_block();
        let result = (|| {
            let mut parameters = Vec::new();
            for (parameter, typ) in function.parameters.iter().zip(&signature.parameters) {
                self.declare(interner.intern(&parameter.name), typ.clone(), interner)?;
                parameters.push(Parameter {
                    name: parameter.name.clone(),
                    typ: Some(typ.clone()),
                });
            }
            let body = function
                .body
                .iter()
                .map(|stmt| self.check_stmt(stmt, interner))
                .collect::<Result<Vec<_>, _>>()?;
            if function.return_type != Type::None && !always_returns(&body) {
                return Err(format!(
                    "TypeError: '{}' must return '{}' on every path",
                    function.name,
                    function.return_type.name()
                ));
            }
            Ok(FunctionDef {
                name: function.name.clone(),
                parameters,
                return_type: function.return_type.clone(),
                body,
            })
        })();
        self.scopes = saved_scopes;
        self.return_type = saved_return;
        result
    }

    fn require_global_definition(&self) -> Result<(), String> {
        if self.scopes.len() != 1 || self.return_type.is_some() {
            return Err(
                "TypeError: function and class definitions must be at module scope".to_string(),
            );
        }
        Ok(())
    }

    pub(super) fn check_function_definition(
        &mut self,
        function: &FunctionDef,
        interner: &mut Interner,
    ) -> Result<Stmt, String> {
        self.require_global_definition()?;
        let signature = self.signature(function, None)?;
        self.declare(
            interner.intern(&function.name),
            Type::Function(signature.clone()),
            interner,
        )?;
        Ok(Stmt::Function(
            self.check_function_body(function, &signature, interner)?,
        ))
    }

    pub(super) fn check_class(
        &mut self,
        name: &str,
        body: &[Stmt],
        interner: &mut Interner,
    ) -> Result<Stmt, String> {
        self.require_global_definition()?;
        if matches!(name, "int" | "bool") {
            return Err("TypeError: cannot redefine a builtin type".to_string());
        }
        self.declare(
            interner.intern(name),
            Type::Constructor(name.to_string()),
            interner,
        )?;
        let mut info = ClassInfo {
            fields: BTreeMap::new(),
            methods: BTreeMap::new(),
        };
        let mut names = BTreeSet::new();
        let mut resolved = Vec::new();
        // Fields may reference earlier classes only: the instance ownership graph
        // remains acyclic. All instance fields have a checked default value.
        for stmt in body {
            match stmt {
                Stmt::VariableDecl {
                    name,
                    typ,
                    initializer,
                } => {
                    if !names.insert(name.clone()) {
                        return Err(format!("TypeError: duplicate member '{}'", name));
                    }
                    self.validate_type(typ)?;
                    if let Some(value) = initializer {
                        let actual = self.check_expr(value, interner)?;
                        if &actual != typ {
                            return Err(format!(
                                "TypeError: field '{}' has incompatible default",
                                name
                            ));
                        }
                    } else if matches!(typ, Type::Named(_)) {
                        return Err(format!("TypeError: field '{}' needs an initializer", name));
                    }
                    info.fields.insert(name.clone(), typ.clone());
                    resolved.push(stmt.clone());
                }
                Stmt::Function(function) => {
                    if !names.insert(function.name.clone()) {
                        return Err(format!("TypeError: duplicate member '{}'", function.name));
                    }
                }
                Stmt::Pass => resolved.push(Stmt::Pass),
                _ => {
                    return Err(
                        "TypeError: class bodies contain annotated fields and methods only"
                            .to_string(),
                    );
                }
            }
        }
        self.classes.insert(name.to_string(), info.clone());
        // Register every method signature before checking any method body.
        for stmt in body {
            if let Stmt::Function(function) = stmt {
                let signature = self.signature(function, Some(name))?;
                if function.name == "__init__" && function.return_type != Type::None {
                    return Err("TypeError: __init__ must return None".to_string());
                }
                info.methods.insert(function.name.clone(), signature);
            }
        }
        self.classes.insert(name.to_string(), info.clone());
        for stmt in body {
            if let Stmt::Function(function) = stmt {
                let signature = info
                    .methods
                    .get(&function.name)
                    .ok_or("TypeError: missing method signature")?;
                resolved.push(Stmt::Function(
                    self.check_function_body(function, signature, interner)?,
                ));
            }
        }
        Ok(Stmt::Class {
            name: name.to_string(),
            body: resolved,
        })
    }

    fn class_info(&self, typ: &Type) -> Result<&ClassInfo, String> {
        if let Type::Named(name) = typ {
            return self
                .classes
                .get(name)
                .ok_or_else(|| format!("TypeError: unknown class '{}'", name));
        }
        Err(format!(
            "TypeError: '{}' has no instance members",
            typ.name()
        ))
    }

    pub(super) fn field_type(&self, typ: &Type, name: &str) -> Result<Type, String> {
        self.class_info(typ)?
            .fields
            .get(name)
            .cloned()
            .ok_or_else(|| format!("TypeError: unknown or read-only field '{}'", name))
    }

    pub(super) fn attribute_type(&self, typ: &Type, name: &str) -> Result<Type, String> {
        let info = self.class_info(typ)?;
        if let Some(typ) = info.fields.get(name) {
            return Ok(typ.clone());
        }
        let mut signature = info
            .methods
            .get(name)
            .cloned()
            .ok_or_else(|| format!("TypeError: unknown member '{}'", name))?;
        signature.parameters.remove(0);
        Ok(Type::Function(signature))
    }

    pub(super) fn call_signature(&self, typ: &Type) -> Result<Signature, String> {
        match typ {
            Type::Function(signature) => Ok(signature.clone()),
            Type::Constructor(name) => {
                let info = self
                    .classes
                    .get(name)
                    .ok_or("TypeError: incomplete class")?;
                let parameters = info
                    .methods
                    .get("__init__")
                    .map(|sig| sig.parameters[1..].to_vec())
                    .unwrap_or_default();
                Ok(Signature {
                    parameters,
                    result: Box::new(Type::Named(name.clone())),
                })
            }
            _ => Err(format!("TypeError: '{}' is not callable", typ.name())),
        }
    }
}

fn always_returns(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| match stmt {
        Stmt::Return(_) => true,
        Stmt::If {
            then_branch,
            elif_branches,
            else_branch: Some(other),
            ..
        } => {
            always_returns(then_branch)
                && elif_branches.iter().all(|(_, body)| always_returns(body))
                && always_returns(other)
        }
        _ => false,
    })
}
