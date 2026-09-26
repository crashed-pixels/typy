# Builtin functions

## print

`print()` is the current builtin function. It accepts zero or more positional
values of any supported runtime type, separates them with one space, appends
one newline, and returns `None`. An empty call prints a blank line. There are no
`sep`, `end`, `file` or `flush` keyword options, and string literals are not yet
supported.

```typy main.tp
def main() -> None:
    write = print
    write(42, True, False, None)
    write()
    result = print(7)
    print(result)
```

```output
42 True False None

7
None
```

Integers use decimal notation; booleans use `True`/`False`; absence uses `None`.
Functions, classes, instances and bound methods use descriptive angle-bracket
representations. Their names may be module-qualified; memory addresses are not
printed. There is no user-defined `__str__` or `__repr__` dispatch.

The checker knows the builtin's variadic signature and `None` result. Every
argument expression is still checked; `print(unknown)` and `n: int = print(1)`
fail before output. `print` is an object, not a keyword: it can be aliased and can
be shadowed by a local binding. Calls then follow the shadowing binding's type.

Under the hood, the VM resolves `print` from a native builtin table and caches its
object identity. `CALL` dispatches native and user functions through the same
callable interface. Native `print` formats values and invokes the host's output
callback; the portable core does not call Rust's `println!` or require `std`.

Output is immediate. A later runtime error rolls back language state, but cannot
retract printed text. An output callback returning an error aborts evaluation and
rolls back state. Without an output callback, calling `print` produces `IOError`.
The CLI supplies stdout; embedded hosts can provide a buffer, UART or another sink.
