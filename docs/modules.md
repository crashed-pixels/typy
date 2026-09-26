# Modules, imports and entry points

## Executables and libraries

Only `main.tp` is executable. It must locally declare exactly the entry signature
`def main() -> None`: no parameters, no return value. Importing a `main` function
or assigning a function to that name does not declare an entry. `main.tp` itself
cannot be imported. Imported modules' functions named `main` are ordinary functions
and are never invoked automatically.

A library directory exposes its declarations and explicit reexports through
`lib.tp`. It has no automatic entry call and cannot be executed as a script.
A sibling `lib.tp` can also be imported directly with `import lib`.

Example project:

```typy main.tp
from arithmetic import twice as double
import toolkit as tools

def main() -> None:
    counter: tools.Counter = tools.Counter()
    print(double(counter.next()), counter.next())
```

```typy arithmetic.tp
def twice(value: int) -> int:
    return value + value
```

```typy toolkit/lib.tp
from counters import Counter
```

```typy toolkit/counters.tp
class Counter:
    value: int = 20

    def next(self) -> int:
        self.value = self.value + 1
        return self.value
```

```output
42 22
```

## Import syntax

| Syntax | Visible names |
| --- | --- |
| `import arithmetic` | Namespace `arithmetic`; use `arithmetic.twice` |
| `import arithmetic as math` | Namespace `math` |
| `from arithmetic import twice` | Only `twice` |
| `from values import first, second as other` | `first`, `other` |
| `import nested.tools as tools` | Namespace for the dotted path |

`from module import *` is always rejected. An `import` statement imports one
module; repeat the statement for another. A dotted import without `as` binds its
last component: `import nested.tools` binds `tools`. Relative-dot syntax, dynamic
imports and runtime module objects are unsupported. Imports are module-scope only.

Each module exports its own declarations and explicitly imported names. Importing
one function does not leak its globals or dependencies into the importing scope.
A namespace is compile-time only: it cannot be passed to `print`, assigned as a
value or modified. Named imports are read-only aliases. Local annotated bindings
may shadow them; instance fields reachable through an imported value remain mutable.

## Resolution and compilation

The CLI first searches beside the importing file, then beside the entry `main.tp`.
A logical `tools` resolves to either `tools.tp` or `tools/lib.tp`. If both exist
at the selected search location, resolution fails as ambiguous. Dotted paths map
to directories. Filenames use ASCII snake_case. There is no global search path or
package download. Canonical file identities deduplicate imports and symlinks;
resolved files must stay inside the entry's project directory.

Imports expand during compilation at their source position, before type checking
and bytecode generation. The first import inserts the module's declarations and
its dependencies; later imports only introduce aliases to those same definitions.
The entire imported module is checked, including unused definitions. No runtime
file access or import instruction remains.

Globals and nominal types are qualified by canonical module identity. Two modules
can both define `Point` or `value` without collisions. Diamond imports share one
module instance. Names must be visible at their use site: an import after a
function does not retroactively make names visible inside that function. Cycles,
missing names, duplicate local import names and more than 64 active modules fail
compilation. Cyclic imports report the dependency path.

## Top-level rules

A file contains imports, functions, classes, scalar/global declarations and `pass`.
A first assignment such as `limit = 10` declares a global; later reassignment must
happen inside a function. Global and field initializers may use literals, earlier
names and non-call expressions, but cannot call functions or constructors.
Top-level conditionals, expression statements, field mutations and returns are
rejected. This prevents imported modules from running user code before `main`.
Declaration initialization may still report arithmetic errors before entry.

The REPL and `Interpreter::eval` evaluate standalone submissions. They retain
interactive statement behavior and require no `main`; imports need the separate
`Program::compile` API and a host [module loader](embedding.md).
