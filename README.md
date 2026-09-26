# TyPy

A small, statically typed Python-like language written in Rust.
Source is parsed, type-checked, compiled to bytecode, and executed by a stack VM.
The core uses `no_std + alloc`; the optional desktop CLI provides files and REPL.

```python
def twice(n: int) -> int:
    return n + n

class Counter:
    value: int = 0

    def __init__(self, start: int) -> None:
        self.value = start

    def add(self, amount: int) -> int:
        self.value = self.value + amount
        return self.value

counter = Counter(10)
counter.add(twice(3))
```

The result is `16`.

## Language

- Checked `i64` integers, booleans, `None`, arithmetic, comparisons, `if/elif/else`.
- `a = 123` infers a fixed type; `a = True` afterward is a static error.
- Annotated block declarations can shadow outer variables. Ordinary assignments
  update the nearest visible binding.
- Functions require parameter and return annotations. Methods infer the type of
  their first parameter, `self`. Calls, fields, returns, and all return paths are
  checked before execution. Functions support recursion and early `return`.
- Classes have declared fields, optional `__init__`, instance methods, and nominal
  instance types. Bound methods can be assigned to variables and called later.
- A failed submission rolls back declarations, globals, and instance-field writes.
  The last executed expression is displayed; declarations and assignments are silent.

Current limits: module-level definitions, positional arguments, no inheritance,
closures, decorators, or dynamic attributes. Declare fields in the class body.
Object-valued fields require defaults and can refer only to earlier classes,
keeping ownership acyclic. Defaults are copied for each new instance.

## Run

```sh
cargo run                       # REPL; a blank line submits a block
cargo run -- example.tp          # snake_case filename, .tp extension
cargo run -- --debug example.tp  # tokens, AST, bytecode, VM state
```

EOF submits pending REPL input. LF and CRLF source are supported.

## Embed

Disable default features in the embedding dependency. The host supplies an
allocator, startup code, panic handler, and I/O; `no_std` does not mean heapless.

```rust
let mut interpreter = typy::Interpreter::new();
interpreter.set_call_limit(128); // default; heap-backed VM call frames
interpreter.eval("a = 123")?;
assert_eq!(interpreter.eval("a + 1")?.as_int(), Some(124));
```

Objects use non-atomic reference counting and are single-threaded. Trace callbacks
are available through `eval_with_trace`. See [embedding details](docs/embedding.md).

## Layout

| Module | Responsibility |
| --- | --- |
| `ast`, `tokenizer`, `parser` | Syntax and declarations |
| `types` | Inference, signatures, nominal classes, return checking |
| `bytecode`, `compiler` | Instructions and function/class compilation |
| `object` | Type descriptors, values, instances, bound methods |
| `vm`, `interpreter` | Calls, execution, transactional state |
| `cli/repl`, `cli/session`, `cli/config` | Input buffering, presentation, arguments |

## Check

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --release --locked --all-features
cargo test --locked --no-default-features
```

CI covers Linux, Windows, macOS, and cross-compilation for Cortex-M0, RISC-V,
and WebAssembly. Cross-compilation does not replace testing on physical devices.
