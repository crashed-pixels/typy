use std::{cell::RefCell, io::Write};
use typy::{
    compiler::Compiler, parser::Parser, symbol::Interner, tokenizer::try_tokenize_str,
    types::TypeChecker, vm::VM,
};

/// Desktop presentation of the same atomic pipeline used by Interpreter.
pub(super) struct Session {
    vm: VM,
    interner: Interner,
    checker: TypeChecker,
    debug: bool,
}
impl Session {
    pub(super) fn new(debug: bool) -> Self {
        Self {
            vm: VM::new(),
            interner: Interner::new(),
            checker: TypeChecker::new(),
            debug,
        }
    }
    pub(super) fn execute(&mut self, source: &str, output: &mut impl Write) -> Result<(), String> {
        self.evaluate(source, output)
            .map_err(|error| error.to_string())
    }
    fn evaluate(
        &mut self,
        source: &str,
        output: &mut impl Write,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tokens = try_tokenize_str(source)?;
        if self.debug {
            writeln!(output, "[0] Source:\n{source}\n[1] Tokens: {tokens:?}")?;
        }
        let ast = Parser::new(tokens).parse()?;
        if self.debug {
            writeln!(output, "\n[2] AST: {ast:#?}")?;
        }
        let mut candidate = self.checker.clone();
        let ast = candidate.check_and_resolve(&ast, &mut self.interner)?;
        let code = Compiler::new().compile(&ast, &mut self.interner);
        if self.debug {
            writeln!(output, "\n[3] Byte-code: {code:?}\n\n[4] Running:")?;
        }
        let mut trace_error = None;
        let shared_output = RefCell::new(&mut *output);
        let result = self.vm.run_with_io(
            &code,
            &self.interner,
            |line| {
                shared_output
                    .borrow_mut()
                    .write_all(line.as_bytes())
                    .map_err(|error| format!("IOError: {error}"))
            },
            |event| {
                if self.debug && trace_error.is_none() {
                    trace_error = writeln!(shared_output.borrow_mut(), "{event}").err();
                }
            },
        )?;
        // A failed output transport must not commit only one side of the state.
        self.checker = candidate;
        if let Some(error) = trace_error {
            return Err(error.into());
        }
        if !result.is_none() {
            writeln!(output, "{result}")?;
        }
        Ok(())
    }
}
