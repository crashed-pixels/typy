//! Compile-time module expansion. The host supplies sources; the core never opens files.
mod linker;
mod names;
use crate::{
    compiler::{Compiler, Instruction},
    object::Object,
    symbol::Interner,
    types::TypeChecker,
    vm::{TraceEvent, VM},
};
use alloc::{string::String, vec::Vec};

/// A source and its canonical identity. Aliases must return the same name, so
/// diamond imports share definitions and cyclic imports can be detected.
pub struct SourceModule {
    pub name: String,
    pub source: String,
}
impl SourceModule {
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
        }
    }
}

/// Resolves a logical dotted import relative to the importing module.
/// Filesystem hosts expose libraries through `<library>/lib.tp`.
pub trait ModuleLoader {
    fn load(&mut self, importer: &str, module: &str) -> Result<SourceModule, String>;
}

/// Fully linked and checked bytecode ending with one call to `main() -> None`.
/// Every run uses a fresh VM; compilation and imports perform no program I/O.
pub struct Program {
    instructions: Vec<Instruction>,
    interner: Interner,
}
impl Program {
    pub fn compile(entry: SourceModule, loader: &mut impl ModuleLoader) -> Result<Self, String> {
        let ast = linker::Linker::new(loader).program(entry)?;
        let mut interner = Interner::new();
        let ast = TypeChecker::new().check_and_resolve(&ast, &mut interner)?;
        let instructions = Compiler::new().compile(&ast, &mut interner);
        Ok(Self {
            instructions,
            interner,
        })
    }

    pub fn run(&self, output: impl FnMut(&str) -> Result<(), String>) -> Result<Object, String> {
        self.run_with_options(128, output, |_| {})
    }

    /// Select a call limit and separate host callbacks for output and diagnostics.
    pub fn run_with_options(
        &self,
        call_limit: usize,
        output: impl FnMut(&str) -> Result<(), String>,
        trace: impl FnMut(&TraceEvent<'_>),
    ) -> Result<Object, String> {
        let mut vm = VM::new();
        vm.set_call_limit(call_limit);
        vm.run_with_io(&self.instructions, &self.interner, output, trace)
    }
}
