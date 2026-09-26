# TyPy

A small, statically typed Python-like language written in Rust.
The interpreter core uses `no_std + alloc`; the optional CLI runs files and a REPL.

Save as `main.tp`:

```typy main.tp
def twice(n: int) -> int:
    return n + n

def main() -> None:
    print(twice(21))
```

```output
42
```

```sh
cargo run -- main.tp
cargo run -- --debug main.tp   # diagnostics go to stderr
cargo run                     # interactive REPL
```

- Fixed-type inference: `a = 123`; assigning `True` to `a` is a static error.
- Checked integers, booleans, `None`, arithmetic and conditionals.
- Typed functions, recursion, classes, constructors, fields and bound methods.
- Explicit imports expanded at compile time; wildcard imports are forbidden.
- Programs enter `main.tp::main() -> None`; libraries expose `lib.tp`.
- Program output uses builtin `print()`; bare expressions are silent in files.

[Documentation](docs/README.md) covers the language, modules, CLI, internals and
embedding. Current limits include no strings, containers, loops, inheritance or
closures. Embedded hosts provide an allocator, platform runtime and I/O.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --release --locked --all-features
cargo test --locked --no-default-features
```

CI checks Linux, Windows, macOS and ARM/RISC-V/Wasm cross-compilation. Complete
Markdown examples are compiled and their printed output is tested.
