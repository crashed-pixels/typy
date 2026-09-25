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
