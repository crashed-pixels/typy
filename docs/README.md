# TyPy documentation

TyPy is statically checked before bytecode execution. Python-like syntax does not
imply Python compatibility; the pages below describe the implemented language.

| Guide | Contents |
| --- | --- |
| [Getting started](getting-started.md) | Installation, first executable, project layout |
| [Language](language.md) | Syntax, values, inference, scopes, operators, conditionals |
| [Functions](functions.md) | Signatures, calls, recursion, returns, callable values |
| [Classes](classes.md) | Fields, construction, methods, identity, ownership |
| [Modules](modules.md) | Explicit imports, libraries, linking and entry rules |
| [Builtins](builtins.md) | `print`, formatting and output errors |
| [CLI and REPL](cli.md) | File execution, interactive input, debugging, migration |
| [Errors](errors.md) | Static/runtime failures, rollback and limits |
| [Internals](internals.md) | Lexer, AST, module linker, checker, compiler, objects, VM |
| [Embedding](embedding.md) | Rust APIs, `no_std`, source loaders, output callbacks |

Complete examples use fences labelled `typy main.tp` (or another filename).
Save each block under that name. The `output` block is the expected stdout.
`tests/docs_examples.rs` compiles these projects and verifies their output.
