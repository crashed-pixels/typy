#![no_std]
#![forbid(unsafe_code)]

//! Compile-only integration example. Firmware supplies an allocator, panic
//! handler, startup code, and transport (UART/USB/etc.) in its executable.
//! No allocator or panic handler is installed globally by the TyPy library.
extern crate alloc;

use alloc::string::String;
use typy::{Interpreter, object::Object};

/// Call repeatedly with the same interpreter to preserve globals between inputs.
pub fn evaluate(interpreter: &mut Interpreter, source: &str) -> Result<Object, String> {
    interpreter.eval(source)
}

/// A host sink may forward complete print lines to UART, USB, or a buffer.
pub fn evaluate_with_output(
    interpreter: &mut Interpreter,
    source: &str,
    output: impl FnMut(&str) -> Result<(), String>,
) -> Result<Object, String> {
    interpreter.eval_with_output(source, output)
}

/// Compile and execute sources supplied by firmware, without filesystem access.
pub fn run_program(
    entry: typy::modules::SourceModule,
    loader: &mut impl typy::modules::ModuleLoader,
    output: impl FnMut(&str) -> Result<(), String>,
) -> Result<Object, String> {
    typy::modules::Program::compile(entry, loader)?.run(output)
}
