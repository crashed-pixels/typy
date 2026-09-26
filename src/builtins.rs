//! Statically known native callables. Hosts own all I/O.
use crate::{object::Object, types::Type};
use alloc::string::{String, ToString};
use core::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Print,
}

pub fn lookup(name: &str) -> Option<Builtin> {
    match name {
        "print" => Some(Builtin::Print),
        _ => None,
    }
}
impl Builtin {
    pub fn name(self) -> &'static str {
        match self {
            Self::Print => "print",
        }
    }
    pub fn result_type(self) -> Type {
        Type::None
    }

    pub(crate) fn call(
        self,
        arguments: &[Object],
        output: &mut impl FnMut(&str) -> Result<(), String>,
    ) -> Result<Object, String> {
        match self {
            Self::Print => {
                let mut line = String::new();
                for (index, value) in arguments.iter().enumerate() {
                    if index > 0 {
                        line.push(' ');
                    }
                    // String's formatter is infallible apart from allocation failure.
                    write!(line, "{value}")
                        .map_err(|_| "IOError: formatting failed".to_string())?;
                }
                line.push('\n');
                output(&line)?;
                Ok(Object::none())
            }
        }
    }
}

pub(crate) fn no_output(_: &str) -> Result<(), String> {
    Err("IOError: print requires a host output callback".into())
}
