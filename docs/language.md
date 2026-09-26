# Language reference

## Source and names

Source is UTF-8. Identifiers start with a Unicode letter or `_` and continue with
letters, digits or `_`. CLI module filenames use ASCII snake_case and `.tp`.
Keywords are `def`, `class`, `return`, `pass`, `if`, `elif`, `else`, `import`,
`from`, `as`, `True`, `False`, and `None`. `print`, `int` and `bool` are names.

Newlines terminate statements. Indentation defines blocks; use four spaces.
A tab counts as four spaces, not Python's tab-stop expansion. Dedentation must
match an earlier level. Blank lines do not change indentation. LF, CRLF and CR
line endings are accepted; EOF closes pending blocks. A block cannot be empty:
use `pass`. There are no semicolons, comments or line continuations.

## Values and declarations

| Type | Values | Default for an annotated declaration |
| --- | --- | --- |
| `int` | Signed, checked 64-bit integers | `0` |
| `bool` | `True`, `False`; distinct from `int` | `False` |
| `None` | The single absence value; runtime type name `NoneType` | `None` |
| Class name | Instances of exactly that nominal class | Initializer required |

Decimal integer literals range from `0` through `9223372036854775807`. Negative
values are computed with subtraction, such as `0 - 5`; unary minus is not yet
implemented. Overflow, underflow and division by zero are recoverable errors.

`count = 10` declares a new binding with inferred type `int`. Later assignments
must keep that type. `count: int = 10` is explicit; `count: int` uses the default.
There are no implicit conversions or truthiness: conditions require `bool`.
Functions and bound methods can be assigned to inferred bindings; their signatures
remain fixed. Callable type annotations are not yet part of the syntax.

## Scopes and operators

An ordinary assignment updates the nearest visible binding. If none exists, it
creates one in the current scope. An explicit declaration creates a new binding
in that scope and can shadow an outer name. Repeating a declaration in the same
scope is an error. Function parameters and block locals do not escape their scope.
Imported names are read-only bindings; explicit local declarations can shadow them.

| Precedence, highest first | Operations |
| --- | --- |
| Atoms and postfix | Parentheses, calls `f(...)`, attributes `obj.field` |
| Multiplicative | `*`, `/` |
| Additive | `+`, `-` |
| Comparison | `==`, `!=`, `<`, `>`, `<=`, `>=` |

Binary operators associate left to right. Arithmetic and ordering require `int`.
Division returns `int` and truncates toward zero. Equality requires matching types;
scalars compare by value, instances and callables by identity. Comparisons return
`bool`. Chained comparisons are not Python chains: write nested conditionals
instead of `1 < x < 3`. `and`, `or`, `not`, `%`, `//` and `**` are unsupported.

## Conditionals

`if` tests one condition, followed by any number of `elif` branches and an optional
`else`. Only the first matching branch runs. Each branch introduces a block scope.
All branches are statically checked, including those that cannot run.

```typy main.tp
def main() -> None:
    count = 10
    empty: int
    flag: bool
    absent: None
    if count > 5:
        count = count + 1
        label: bool = True
        print(label)
    elif count == 5:
        pass
    else:
        print(False)
    if True:
        count: bool = False
        print(count)
    print(count, empty, flag, absent)
    print(2 + 3 * 4, (0 - 5) / 2, 4 != 5, 4 <= 5)
```

```output
True
False
11 0 False None
14 -2 True True
```

At file scope, assignments must introduce new bindings and initializers cannot
call functions. Perform computations with side effects and mutations in `main`
or functions it calls. See [module rules](modules.md).

Strings, floats, containers, loops, exceptions, pattern matching, generators,
async execution and operator overloading are not implemented.
