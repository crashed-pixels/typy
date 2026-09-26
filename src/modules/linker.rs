use super::names::{Binding, Names, Namespace};
use super::{ModuleLoader, SourceModule};
use crate::{
    ast::{Expr, Stmt},
    parser::Parser,
    tokenizer::try_tokenize_str,
    types::Type,
};
use alloc::{boxed::Box, collections::BTreeMap, format, rc::Rc, string::String, vec::Vec};

pub(super) struct Linker<'a, L> {
    loader: &'a mut L,
    loaded: BTreeMap<String, Rc<Namespace>>,
    active: Vec<String>,
    statements: Vec<Stmt>,
}
impl<'a, L: ModuleLoader> Linker<'a, L> {
    pub(super) fn new(loader: &'a mut L) -> Self {
        Self {
            loader,
            loaded: BTreeMap::new(),
            active: Vec::new(),
            statements: Vec::new(),
        }
    }
    pub(super) fn program(mut self, entry: SourceModule) -> Result<Vec<Stmt>, String> {
        if filename(&entry.name) != "main.tp" {
            return Err("EntryError: programs must start in main.tp".into());
        }
        let parsed = parse(&entry)?;
        let main = parsed
            .iter()
            .find_map(|stmt| match stmt {
                Stmt::Function(function) if function.name == "main" => Some(function),
                _ => None,
            })
            .ok_or("EntryError: main.tp must declare def main() -> None")?;
        if !main.parameters.is_empty() || main.return_type != Type::None {
            return Err("EntryError: expected def main() -> None".into());
        }
        let exports = self.expand(entry.name, parsed)?;
        let entry = exports
            .get("main")
            .ok_or("EntryError: missing main")?
            .value()?;
        self.statements.push(Stmt::Expr(Expr::Call {
            callee: Box::new(Expr::Name(entry)),
            arguments: Vec::new(),
        }));
        Ok(self.statements)
    }

    fn import(&mut self, importer: &str, module: &str) -> Result<Rc<Namespace>, String> {
        let source = self
            .loader
            .load(importer, module)
            .map_err(|error| format!("{importer}: import {module}: {error}"))?;
        if filename(&source.name) == "main.tp" {
            return Err(
                "ImportError: main.tp is an executable entry, not an importable library".into(),
            );
        }
        if let Some(exports) = self.loaded.get(&source.name) {
            return Ok(exports.clone());
        }
        if self.active.contains(&source.name) {
            return Err(format!(
                "ImportError: cycle: {} -> {}",
                self.active.join(" -> "),
                source.name
            ));
        }
        if self.active.len() >= 64 {
            return Err("ImportError: module nesting exceeds 64".into());
        }
        let ast = parse(&source)?;
        self.expand(source.name, ast)
    }

    fn expand(&mut self, name: String, ast: Vec<Stmt>) -> Result<Rc<Namespace>, String> {
        self.active.push(name.clone());
        let mut names = Names::new(&name);
        for stmt in ast {
            match stmt {
                Stmt::Import { module, alias } => {
                    let exports = self.import(&name, &module)?;
                    names.import(alias, Binding::Module(exports))?;
                }
                Stmt::FromImport {
                    module,
                    names: imports,
                } => {
                    let exports = self.import(&name, &module)?;
                    for import in imports {
                        let binding = exports.get(&import.name).ok_or_else(|| {
                            format!("ImportError: '{module}' does not export '{}'", import.name)
                        })?;
                        names.import(import.alias, binding.clone())?;
                    }
                }
                _ => {
                    validate_declaration(&stmt).map_err(|error| format!("{name}: {error}"))?;
                    let stmt = names
                        .statement(&stmt)
                        .map_err(|error| format!("{name}: {error}"))?;
                    self.statements.push(stmt);
                }
            }
        }
        self.active.pop();
        let exports = Rc::new(names.exports());
        self.loaded.insert(name, exports.clone());
        Ok(exports)
    }
}
fn filename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}
fn parse(source: &SourceModule) -> Result<Vec<Stmt>, String> {
    let tokens =
        try_tokenize_str(&source.source).map_err(|error| format!("{}: {error}", source.name))?;
    Parser::new(tokens)
        .parse()
        .map_err(|error| format!("{}: {error}", source.name))
}

/// Module initialization may construct declarations, but never call user code.
fn validate_declaration(stmt: &Stmt) -> Result<(), String> {
    match stmt {
        Stmt::Function(_) | Stmt::Pass => Ok(()),
        Stmt::Class { body, .. } => {
            for member in body {
                match member {
                    Stmt::Function(_) | Stmt::Pass | Stmt::VariableDecl { .. } => {
                        validate_declaration(member)?
                    }
                    _ => {
                        return Err(
                            "TypeError: class bodies allow declared fields and methods".into()
                        );
                    }
                }
            }
            Ok(())
        }
        Stmt::VariableDecl { initializer, .. } => {
            if let Some(value) = initializer {
                validate_initializer(value)?;
            }
            Ok(())
        }
        Stmt::Assign { value, .. } => validate_initializer(value),
        _ => Err(
            "EntryError: executable statements belong inside functions, starting with main".into(),
        ),
    }
}
fn validate_initializer(expr: &Expr) -> Result<(), String> {
    match expr {
        Expr::Call { .. } => Err(
            "EntryError: module initializers cannot call functions; move execution into main"
                .into(),
        ),
        Expr::BinaryOp { left, right, .. } => {
            validate_initializer(left)?;
            validate_initializer(right)
        }
        Expr::Attribute { object, .. } => validate_initializer(object),
        _ => Ok(()),
    }
}
