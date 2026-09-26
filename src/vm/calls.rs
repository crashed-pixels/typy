use super::*;
use crate::bytecode::FunctionCode;
use crate::object::Callable;
use alloc::rc::Rc;

pub(super) enum Code<'a> {
    Root(&'a [Instruction]),
    Function(Rc<FunctionCode>),
}
impl Code<'_> {
    pub(super) fn instructions(&self) -> &[Instruction] {
        match self {
            Self::Root(code) => code,
            Self::Function(function) => &function.instructions,
        }
    }
}
pub(super) struct Activation<'a> {
    pub(super) code: Code<'a>,
    pub(super) ip: usize,
    pub(super) frame_base: usize,
    pub(super) stack_base: usize,
    pub(super) saved_result: Option<Object>,
    pub(super) constructor: Option<Object>,
}
impl<'a> Activation<'a> {
    pub(super) fn root(code: &'a [Instruction]) -> Self {
        Self {
            code: Code::Root(code),
            ip: 0,
            frame_base: 1,
            stack_base: 0,
            saved_result: None,
            constructor: None,
        }
    }
}
pub(super) struct Invocation {
    pub(super) function: Rc<FunctionCode>,
    pub(super) arguments: Vec<Object>,
    pub(super) constructor: Option<Object>,
}

impl VM {
    pub(super) fn prepare_call(&mut self, arity: usize) -> Result<Option<Invocation>, String> {
        if self.stack.len() <= arity {
            return Err("SystemError: missing call operands".to_string());
        }
        let mut arguments = self.stack.split_off(self.stack.len() - arity);
        let callee = self.stack.pop().ok_or("SystemError: missing callee")?;
        let mut constructor = None;
        let function = match callee.callable()? {
            Callable::Function(function) => function,
            Callable::Bound(function, receiver) => {
                arguments.insert(0, receiver);
                function
            }
            Callable::Class(class) => {
                let object = Object::instantiate(class.clone());
                if let Some(function) = class.code.methods.get("__init__") {
                    arguments.insert(0, object.clone());
                    constructor = Some(object);
                    function.clone()
                } else {
                    if !arguments.is_empty() {
                        return Err("TypeError: constructor expects no arguments".to_string());
                    }
                    self.stack.push(object);
                    return Ok(None);
                }
            }
        };
        if function.arity != arguments.len() {
            return Err(format!(
                "TypeError: '{}' expects {} arguments, got {}",
                function.name,
                function.arity,
                arguments.len()
            ));
        }
        Ok(Some(Invocation {
            function,
            arguments,
            constructor,
        }))
    }
}
