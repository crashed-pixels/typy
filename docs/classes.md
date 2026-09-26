# Classes and methods

Classes are nominal types: two classes with identical fields are still different
types. Declare classes at module scope. Their bodies contain annotated fields,
methods and `pass`. There is no inheritance or dynamic creation of attributes.

Fields without initializers use scalar defaults. `__init__` is optional, must
return `None`, and supplies the constructor's parameters after `self`. A class
without it accepts no arguments. Every method's first parameter must be `self`;
its annotation may be omitted or name the owning class.

```typy main.tp
class Counter:
    value: int = 0

    def __init__(self, start: int) -> None:
        self.value = start

    def add(self, amount: int) -> int:
        self.value = self.value + amount
        return self.get()

    def get(self) -> int:
        return self.value

def read(counter: Counter) -> int:
    return counter.get()

def main() -> None:
    first = Counter(10)
    alias = first
    other = Counter(0)
    increment = alias.add
    print(increment(5), read(first), other.get())
    print(first == alias, first == other)
```

```output
15 15 0
True False
```

`obj.field` reads a declared field; assignment checks its exact type. `obj.method`
creates a bound method holding the receiver. Saving it and calling it later
preserves that receiver. All method signatures are registered before checking
method bodies, so `add` may call the later-declared `get`. Fields and methods
cannot share a name. Class-level field access and static/class methods are absent.

Assignment shares instance identity. Construction creates a distinct instance;
mutable field defaults are deep-copied for each instance, preserving aliases
within that copied graph. Scalar defaults are evaluated when the class is defined.

Object-valued fields need explicit defaults of an earlier class. This prevents
ownership cycles without a garbage collector. Self-referential fields and mutual
recursive field schemas are rejected; method parameters and results may still
use their own class. In interactive/embedding evaluations, a default can be a
constructor expression. File modules prohibit calls in initializers, so such
constructor defaults are currently unavailable in compiled file projects.

A failed evaluation restores instance-field writes through every alias, including
writes made by methods and failing constructors. Printed text is already delivered
and cannot be restored. See [errors](errors.md) and [object internals](internals.md).
