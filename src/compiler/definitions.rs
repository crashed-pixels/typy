use super::*;
use crate::ast::FunctionDef;
use alloc::{collections::BTreeMap, string::ToString};

impl Compiler {
    pub(super) fn compile_function(
        function: &FunctionDef,
        interner: &mut Interner,
    ) -> Rc<FunctionCode> {
        let mut compiler = Self::new();
        compiler.enter_scope();
        for parameter in &function.parameters {
            compiler.declare_local(interner.intern(&parameter.name));
        }
        let locals =
            function.parameters.len() + compiler.count_block_locals(&function.body, interner);
        for stmt in &function.body {
            compiler.compile_stmt(stmt, interner);
        }
        compiler.emit(Instruction::LoadConst(Object::none()));
        compiler.emit(Instruction::Return);
        Rc::new(FunctionCode {
            name: function.name.clone(),
            arity: function.parameters.len(),
            locals,
            instructions: compiler.code,
        })
    }

    pub(super) fn compile_class(&mut self, name: &str, body: &[Stmt], interner: &mut Interner) {
        let mut fields = Vec::new();
        let mut methods = BTreeMap::new();
        for stmt in body {
            match stmt {
                Stmt::VariableDecl {
                    name,
                    typ,
                    initializer,
                } => {
                    fields.push(name.clone());
                    if let Some(value) = initializer {
                        self.compile_expr(value, interner);
                    } else {
                        let value = match typ {
                            Type::Int => Object::int(0),
                            Type::Bool => Object::bool(false),
                            _ => Object::none(),
                        };
                        self.emit(Instruction::LoadConst(value));
                    }
                }
                Stmt::Function(function) => {
                    methods.insert(
                        function.name.clone(),
                        Self::compile_function(function, interner),
                    );
                }
                _ => {}
            }
        }
        self.emit(Instruction::CreateClass(Rc::new(ClassCode {
            name: name.to_string(),
            fields,
            methods,
        })));
        self.emit(Instruction::StoreName(interner.intern(name)));
    }
}
