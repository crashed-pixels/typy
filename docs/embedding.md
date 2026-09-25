# Portable core, objects, and bindings

## Building and embedding

The library always declares `#![no_std]` and uses only `core` and `alloc`.
It forbids unsafe Rust in TyPy itself and has no third-party dependencies.
The default `cli` feature enables the desktop binary; it does not enable `std`
in the library. Only `src/main.rs` owns files, stdin/stdout, command-line
arguments, and process exit handling. File execution, REPL, and `--debug` remain
available through `cargo run` / `cargo run -- program.tp`.

A firmware dependency should disable the CLI:

```toml
[dependencies]
typy = { git = "https://github.com/skv0zsneg/typy", branch = "no-std-objects-inference", default-features = false }
```

```rust
let mut interpreter = typy::Interpreter::new();
interpreter.eval("a = 123")?;
let result = interpreter.eval("a + 1")?;
assert_eq!(result.as_int(), Some(124));
```

Keep the same interpreter to retain state across submissions. Send complete
source strings from any host transport; the library does not assume a terminal
or collect multiline input. `eval_with_trace` sends borrowed VM state to a
callback, so firmware can format it into a buffer or send it over UART.
Tracing is observational: already delivered events cannot be rolled back.

The embedding executable must supply:

- A global allocator with enough heap for source, AST, bytecode, objects, and
  state snapshots. This is **no_std + alloc**, not a heapless implementation.
- Its own startup code, panic handler, linker script, and target-specific I/O.
- Appropriate memory and stack budgets. Allocation failure and stack exhaustion
  are host/runtime failures; they are not recoverable TyPy language errors.

The core uses `BTreeMap`/`BTreeSet`, avoiding OS random-number sources, and `Rc`
for immutable objects, avoiding hardware atomics. An interpreter and its object
handles are single-threaded (`!Send`, `!Sync`). Current payloads cannot form
cycles; adding mutable objects with back-references will require a cycle policy.

CI cross-compiles the library and the `#![no_std]` embedding example for:

| Target | Environment |
| --- | --- |
| `thumbv6m-none-eabi` | ARM Cortex-M0/M0+, including targets without native atomics |
| `riscv32imc-unknown-none-elf` | Bare-metal RISC-V RV32IMC |
| `wasm32-unknown-unknown` | WebAssembly without a system interface |

```sh
rustup target add thumbv6m-none-eabi
cargo build --locked --release --no-default-features --lib --example embedded_core --target thumbv6m-none-eabi
cargo test --locked --no-default-features
```

These are compile checks, not execution tests on physical boards. `no_std`
removes OS dependencies; it does not guarantee sufficient resources on every
microcontroller. Desktop CI runs on Linux, Windows, and macOS.

## Object model

Every runtime value is an owning `Object` handle to an immutable `ObjectData`.
Its common `ObjectHeader` points at a `TypeObject`. Payloads are represented by
TyPy's `IntObject`, `BoolObject`, and `NoneObject`, not exposed enum variants of
Rust primitives. Native `i64` and `bool` are implementation details accessible
through checked accessors.

- `Object::int`, `Object::bool`, and `Object::none` construct typed objects.
- `clone()` shares identity through `Rc`; the last handle frees the allocation.
- `is_identical` tests identity. Value equality remains separate from identity.
- Type descriptors are immutable static objects. Their headers point at
  `TYPE_TYPE`; the metatype's own header points at itself.
- `INT_TYPE`, `BOOL_TYPE`, and `NONE_TYPE` have `OBJECT_TYPE` as their base.
  The static checker references the same built-in descriptors.

This follows the common-header/type-pointer idea from CPython, using safe Rust
reference counting instead of its C representation or ABI. It is groundwork
for classes, not class support: heap-allocated user types, attribute lookup,
method slots, constructors, MRO, and cycle collection remain future work.
`None` and booleans are not globally interned singletons. `None` remains an
internal no-result value, not a newly introduced source-language literal.

TyPy retains its existing semantics: `int` is checked `i64`, and `bool` is
separate from `int` rather than inheriting CPython's numeric behavior.

## Inference and shadowing

A first assignment to an unresolved name declares it in the current lexical
scope and infers its type from the initializer. Initializers may be literals
or already supported expressions:

```python
a = 123
b = a + 1
ok = b > a
```

The inferred type is fixed. `a = True` after this program is a static error;
`a = 124` updates the existing binding. A reference on the right-hand side must
already be defined: a first `a = a + 1` is an error.

Assignment to any visible existing name updates the nearest binding, including
an enclosing local or global. Explicit annotated block declarations continue
to support shadowing:

```python
a = 123
if True:
    a: bool = True
    a = False
a
```

The final result is `123`. A newly inferred name inside a block is local to
that block; declarations in `if`, `elif`, or `else` do not escape it. Repeating
an annotated declaration in the same scope remains an error.

The checker lowers inferred first assignments to typed `VariableDecl` nodes.
This lets the compiler allocate local slots using the checked declarations.
Library clients using the individual pipeline stages must compile the AST
returned by `TypeChecker::check_and_resolve`; `check` is validation-only.
`Interpreter` performs these stages and state management automatically.

On any language error, an evaluation rolls back all declarations and global
writes. A failed inferred initializer can be retried, even with a different
type. Interned symbols remain allocated, as before.

## Rust API migration

Language behavior is preserved except for the intentional addition of inferred
declarations. Rust API changes are necessary for owned objects and host I/O:

| Previous API | New API |
| --- | --- |
| `Object::Int(42)` | `Object::int(42)` |
| `Object::Bool(true)` | `Object::bool(true)` |
| `Object::None` | `Object::none()` |
| Matching an object enum variant | `as_int()`, `as_bool()`, `is_none()` |
| `vm.run(code, interner, false)` | `vm.run(code, interner)` |
| `vm.run(code, interner, true)` | `vm.run_with_trace(code, interner, callback)` |
| Check then compile original AST | Compile the AST from `check_and_resolve`, or use `Interpreter` |

## Design references

- [The Embedded Rust Book: no_std](https://doc.rust-lang.org/stable/embedded-book/intro/no-std.html)
- [CPython common object structures](https://docs.python.org/3/c-api/structures.html)
- [CPython type object structures](https://docs.python.org/3/c-api/typeobj.html)
- CPython source counterparts: `Include/object.h`, `Include/cpython/object.h`,
  `Objects/typeobject.c`, `Objects/longobject.c`, and `Objects/boolobject.c`.
