# Getting started

Build with stable Rust (edition 2024):

```sh
cargo build --release --locked
```

Create `main.tp`:

```typy main.tp
def main() -> None:
    value = 21
    print(value + value)
```

Run it:

```sh
cargo run -- main.tp
```

```output
42
```

A program must declare `def main() -> None` in a file named `main.tp`. The runner
checks the whole dependency graph, creates declarations, then calls this function
once. Merely writing `value + value` does not print it.

Keep reusable declarations in sibling modules such as `arithmetic.tp`. A library
directory exposes its API through `lib.tp`; see [modules](modules.md). For example,
`from arithmetic import twice` imports one named definition. There is no package
manager, registry search, runtime import or wildcard import.

Use four spaces for each indentation level and one statement per line. Current
syntax has no comments or strings: examples deliberately contain neither. Omit
the filename to enter the [REPL](cli.md), where expressions are displayed as an
interactive convenience.
