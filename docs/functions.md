# Functions

Declare functions at module scope using `def name(parameters) -> ReturnType:`.
Every parameter and the return type need annotations; the only exception is a
method's `self` receiver. Supported annotations are `int`, `bool`, `None` and
class names, including imported aliases and qualified names such as `models.Point`.

Calls accept positional arguments, evaluated left to right. A trailing comma is
allowed. Argument count and types are checked before execution. There are no
default, keyword, variadic user parameters or decorators.

`return value` exits immediately, even inside nested blocks. A non-`None` function
must return on every path; exhaustive `if/elif/else` branches count. A `None`
function may fall through or use bare `return`. Expression statements never act
as implicit return values.

```typy main.tp
def factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

def announce(value: int) -> None:
    print(value)
    return

def main() -> None:
    calculate = factorial
    announce(calculate(5,))
    print(announce(6))
```

```output
120
6
None
```

Function bindings are first-class objects with statically known signatures;
`calculate` has the same signature as `factorial`. Assigning an incompatible
callable to it is a static error. Functions can refer to earlier globals and
imported definitions, update their own module's globals, and call themselves.
Use an explicit local declaration to shadow a global. Definitions and imports
appearing later are unavailable: declaration order matters.

Mutual forward recursion, nested function definitions and closures are not
supported. The VM uses heap-backed call frames rather than Rust recursion.
The default active-call limit is 128; embedded hosts can configure it. Exceeding
it raises `RecursionError` and unwinds the entire evaluation.
