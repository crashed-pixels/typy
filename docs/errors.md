# Errors and recovery

The high-level APIs return `Result<_, String>`. Errors are not TyPy exception
objects; there is no `try`/`except` syntax.

| Phase | Examples | Effect |
| --- | --- | --- |
| Tokenization/parsing | Invalid token, overflowing literal, indentation mismatch, missing annotation syntax | No bytecode runs |
| Linking | Wildcard, cycle, missing export, duplicate alias, invalid entry, top-level execution | No bytecode runs |
| Type checking | Unknown name/type, wrong argument, incompatible field write, incomplete return paths | No bytecode runs |
| VM | `ZeroDivisionError`, `OverflowError`, `RecursionError` | Restore evaluation state |
| Host output | `IOError` or a callback's error | Restore state; prior output remains delivered |

A malformed module anywhere in the import graph prevents `main` from running,
even if none of its functions would be called. Parser/linker errors include
module identities; diagnostics do not yet provide full source spans or Python-like
tracebacks. Unsupported constructs fail rather than falling back to dynamic typing.

`Interpreter` checks against a cloned type environment. The VM snapshots globals
and journals every instance-field write. On failure, it restores globals and
replays field changes backward, then clears operands and temporary frames. It
commits the checker only after successful execution. Earlier successful submissions
remain available. Symbols interned during failed checks may remain allocated.

Already delivered output and trace callbacks cannot be rolled back. Callbacks
should return errors instead of panicking. Allocation failure, host panic and host
stack exhaustion are outside language-level recovery. This interpreter is not an
untrusted-code sandbox: parsing and AST traversal use the native stack, and heap
allocation is host-controlled.

Function calls use an explicit VM stack with a configurable default limit of 128.
Import expansion permits at most 64 active module levels and rejects cycles.
These limits do not impose a complete CPU or memory budget.
