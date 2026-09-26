# CLI and REPL

## File execution

```sh
cargo run -- path/to/main.tp
cargo run -- --debug path/to/main.tp
```

The built binary accepts the same arguments: `typy main.tp` and
`typy --debug main.tp`. `-d` is equivalent to `--debug`. Unknown flags, multiple
files, arbitrary script filenames and attempts to execute `lib.tp` are errors.
No filename starts the REPL. The runner does not automatically discover `main.tp`
when invoked without arguments.

All dependencies are loaded and checked before the VM starts. Only builtin
`print()` writes program stdout; assignments, declarations, expression statements
and the entry's return value are silent. `--debug` sends instruction traces to
stderr. Errors also go to stderr, with exit status 1. Success returns status 0.

## Interactive use

```sh
cargo run
```

The REPL displays a banner and `>>> ` / `... ` prompts. A line ending in `:`
starts a block. A blank line submits it; `elif` and `else` continue their block.
A dedented next statement is retained for the next submission. EOF submits pending
input and then exits. Each submission is one transaction.

Bare expression results are displayed unless they are `None`. This echo belongs
to the interactive UI; it is not file-execution behavior. `print` also works in
the REPL. Globals, functions, classes and objects persist between submissions.
Errors are reported without ending the session. Interactive `--debug` additionally
shows source, tokens, AST, bytecode and VM state alongside the session output.

Imports currently belong to file compilation, not standalone REPL submissions.
Use `main.tp` to explore multiple modules. `Interpreter::eval` offers the same
standalone evaluation model to Rust hosts, without terminal prompts or auto-echo.

## Migrating old scripts

Rename an executable to `main.tp`, retain imports/functions/classes at module
scope, and move executable statements into `def main() -> None`. Replace bare
expressions intended as output with `print(expression)`. Declare reusable library
exports in `lib.tp`. Initializer calls and repeated global assignments must move
into functions; see [module rules](modules.md).
