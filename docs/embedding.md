# Embedding TyPy

The library always uses `#![no_std]`, `alloc`, and `#![forbid(unsafe_code)]`.
Only the default `cli` binary uses the OS. Disable default features in firmware:

```toml
[dependencies]
typy = { git = "https://github.com/skv0zsneg/typy", branch = "no-std-objects-inference", default-features = false }
```

Keep one `Interpreter` for persistent state. `eval` accepts a complete source
submission; `eval_with_trace` supplies borrowed instruction traces to the host.
Transport framing, UART/USB, and multiline collection belong to the host.

Provide a global allocator, startup/linker integration, and a panic handler.
Budget heap and parsing-stack space; OOM and host stack exhaustion are not TyPy
language exceptions. Function calls use an explicit heap-backed activation stack,
with a default limit of 128; `set_call_limit` adjusts it. Exceeding the limit returns
`RecursionError` and restores the session.

```sh
rustup target add thumbv6m-none-eabi
cargo build --locked --release --no-default-features --lib --example embedded_core --target thumbv6m-none-eabi
```

The same compile check runs for `riscv32imc-unknown-none-elf` and
`wasm32-unknown-unknown`. The example is a library integration check, not firmware
with a board-specific allocator or startup implementation.

## Objects and transactions

`Object` owns a reference-counted value with an immutable type header. Builtin
objects use static descriptors; instances share their class's owned descriptor.
`IntObject`, `BoolObject`, `NoneObject`, functions, classes, instances, and bound
methods are runtime objects. `Rc` avoids atomics; handles are `!Send` and `!Sync`.
Type descriptors expose `type` as their metatype. This follows CPython's common
header/type-pointer idea, without claiming C ABI compatibility.

Values compare by value for scalars and by identity for instances/callables.
Cloning a handle preserves identity. Instance fields are mutable; the VM journals
writes and restores them in reverse order on failure, including through aliases
and nested calls. Successful writes persist. Constructor defaults are deep-copied
per instance, preserving aliases within each copied default graph.

Field types may refer only to earlier classes, so instances cannot form ownership
cycles. Recursive field schemas, inheritance, dynamic attributes, and closures are
not supported. Methods and function parameters/results may use their class's own
type. Method signatures are registered before bodies; module functions and class
annotations otherwise resolve names declared earlier. Functions may call themselves.

Definitions are checked transactionally. Failed declarations and runtime errors
leave the checker and VM consistent. Interned symbols remain allocated. Delivered
trace events and host output cannot be retracted. A host callback should not panic.

## Rust API

Use `Object::int`, `Object::bool`, `Object::none`, `as_int`, `as_bool`, `is_none`,
`type_object`, and `is_identical`. Type descriptors are borrowed from their owner;
the return value of `type_object()` is no longer necessarily `'static`.

`VM::run(code, interner)` executes quietly; `run_with_trace` accepts a callback.
Custom pipelines must compile the AST returned by `TypeChecker::check_and_resolve`;
`check` is validation-only. `Interpreter` performs the entire pipeline automatically.
Class names are nominal; method arguments and return types are never dynamically
coerced. `bool` remains separate from `int`, and integers remain checked `i64`.

## References

- [Embedded Rust Book: no_std](https://doc.rust-lang.org/stable/embedded-book/intro/no-std.html)
- [CPython object structures](https://docs.python.org/3/c-api/structures.html)
- [CPython type objects](https://docs.python.org/3/c-api/typeobj.html)
- CPython counterparts: `Include/object.h`, `Include/cpython/object.h`,
  `Objects/typeobject.c`, `Objects/funcobject.c`, `Objects/classobject.c`.
