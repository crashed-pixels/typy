#![no_std]
#![forbid(unsafe_code)]

//! Portable TyPy interpreter core. Requires `alloc` and a host-provided allocator.
//! Files, terminal I/O, and process handling live exclusively in the optional CLI.

extern crate alloc;

pub mod compiler;
pub mod object;
pub mod parser;
pub mod symbol;
pub mod tokenizer;
pub mod types;
pub mod vm;

pub mod interpreter;
pub use interpreter::Interpreter;

pub mod ast;

pub mod bytecode;

pub mod builtins;
pub mod modules;
