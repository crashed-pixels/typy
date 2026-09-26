# Embedding TyPy

The library always uses `#![no_std]`, `alloc` and `#![forbid(unsafe_code)]`.
Disable the default `cli` feature in firmware:

```toml
[dependencies]
typy = { git = "https://github.com/skv0zsneg/typy", branch = "no-std-objects-inference", default-features = false }
```

The host provides an allocator, startup/linker integration, a panic handler and I/O.
`no_std` does not mean heapless. Handles use non-atomic `Rc` and are neither `Send`
nor `Sync`. Budget both heap and native parser/AST traversal stack space.

## Persistent submissions

Keep one `Interpreter` to retain globals across complete submissions:

```rust
let mut interpreter = typy::Interpreter::new();
interpreter.set_call_limit(128);
interpreter.eval("value = 21")?;
let mut output = String::new();
interpreter.eval_with_output("print(value + value)", |line| {
    output.push_str(line);
    Ok(())
})?;
assert_eq!(output, "42\n");
```

`eval` returns the last expression, with no automatic output. Calling `print`
without a callback returns an error. `eval_with_trace` supplies instruction traces;
`eval_with_io` combines output and tracing. Trace events borrow VM state only for
the duration of the callback. Transport framing and multiline buffering belong to
the host. These APIs evaluate standalone source and do not resolve imports.

## Compiled projects from memory

Use `Program` for imports and executable-entry checks. `ModuleLoader` receives the
importer identity and a dotted logical name, and returns source plus a canonical
identity. Resolve aliases consistently; library entries should identify `lib.tp`.
A firmware loader can read flash or an in-memory table instead of a filesystem.

```rust
use typy::modules::{ModuleLoader, Program, SourceModule};

struct Sources;
impl ModuleLoader for Sources {
    fn load(&mut self, _: &str, name: &str) -> Result<SourceModule, String> {
        match name {
            "math" => Ok(SourceModule::new(
                "math/lib.tp",
                "def twice(n: int) -> int:\n    return n + n\n",
            )),
            _ => Err(format!("ImportError: unknown module {name}")),
        }
    }
}
let entry = SourceModule::new(
    "main.tp",
    "from math import twice\ndef main() -> None:\n    print(twice(21))\n",
);
let program = Program::compile(entry, &mut Sources)?;
let mut output = String::new();
program.run(|line| { output.push_str(line); Ok(()) })?;
assert_eq!(output, "42\n");
```

In `no_std` host code, import `String` and `format!` from `alloc`. The loader owns
path/access policy; the core validates language rules and detects cycles. A complete
`Program` can be reused without its loader; every run uses a fresh VM. Use
`run_with_options(call_limit, output, trace)` to configure execution.

Output callbacks receive a fully formatted line including `\n` and return
`Result<(), String>`. Transport failure aborts evaluation and rolls back language
state. Previously delivered bytes cannot be retracted; callbacks should not panic.

## Objects and lower-level APIs

Use `Object::int`, `bool`, `none`, `as_int`, `as_bool`, `is_none`, `type_object` and
`is_identical`. Cloning a handle shares identity. A user descriptor is borrowed
from its owner, so `type_object()` need not return a `'static` reference.
See [internals](internals.md) for ownership and [errors](errors.md) for transactions.

`VM::run` and `run_with_trace` provide no output sink; `run_with_io` accepts one.
Custom pipelines must compile the AST returned by `TypeChecker::check_and_resolve`.
`check` is validation-only. Expand imports before passing an AST to the low-level
compiler. `Interpreter` and `Program` perform the applicable pipeline automatically.

## Target checks

```sh
rustup target add thumbv6m-none-eabi riscv32imc-unknown-none-elf wasm32-unknown-unknown
cargo build --locked --release --no-default-features --lib --example embedded_core --target thumbv6m-none-eabi
```

CI repeats the build for all three targets. `examples/embedded_core.rs` is a
compile-only integration library, not board firmware or a provided allocator.
