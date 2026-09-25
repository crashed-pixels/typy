use crate::compiler::Compiler;
use crate::object::Object;
use crate::parser::Parser;
use crate::symbol::Interner;
use crate::tokenizer::try_tokenize_str;
use crate::types::TypeChecker;
use crate::vm::{TraceEvent, VM};
use alloc::string::String;

/// A persistent interpreter with no platform I/O.
///
/// Each evaluation is atomic for bindings and values. The host owns the
/// allocator, input collection, output, and any policy for reporting errors.
#[derive(Default)]
pub struct Interpreter {
    checker: TypeChecker,
    interner: Interner,
    vm: VM,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates one complete submission and returns its last expression.
    ///
    /// ```
    /// let mut interpreter = typy::Interpreter::new();
    /// interpreter.eval("a = 123")?;
    /// assert_eq!(interpreter.eval("a + 1")?.as_int(), Some(124));
    /// assert!(interpreter.eval("a = True").is_err());
    /// # Ok::<(), String>(())
    /// ```
    pub fn eval(&mut self, source: &str) -> Result<Object, String> {
        self.eval_with_trace(source, |_| {})
    }

    /// Evaluates with borrowed instruction traces sent to a host callback.
    pub fn eval_with_trace(
        &mut self,
        source: &str,
        trace: impl FnMut(&TraceEvent<'_>),
    ) -> Result<Object, String> {
        let ast = Parser::new(try_tokenize_str(source)?).parse()?;
        let mut candidate = self.checker.clone();
        let ast = candidate.check_and_resolve(&ast, &mut self.interner)?;
        let code = Compiler::new().compile(&ast, &mut self.interner);
        let result = self.vm.run_with_trace(&code, &self.interner, trace)?;
        self.checker = candidate;
        Ok(result)
    }
}
