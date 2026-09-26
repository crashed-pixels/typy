use typy::Interpreter;

const COUNTER: &str = "class Counter:\n    value: int = 0\n    def __init__(self, start: int) -> None:\n        self.value = start\n    def add(self, n: int) -> int:\n        self.value = self.value + n\n        return self.value\n";
fn value(source: &str) -> String { Interpreter::new().eval(source).unwrap().to_string() }

#[test]
fn typed_functions_and_nested_calls() {
    assert_eq!(value("def add(a: int, b: int) -> int:\n    return a + b\ndef twice(x: int) -> int:\n    return add(x, x)\ntwice(add(2, 3))"), "10");
}
#[test]
fn recursion_and_early_returns() {
    assert_eq!(value("def fact(n: int) -> int:\n    if n == 0:\n        return 1\n    return n * fact(n - 1)\nfact(6)"), "720");
}
#[test]
fn function_locals_do_not_escape() {
    let mut vm = Interpreter::new();
    vm.eval("a = 100\ndef f(a: int) -> int:\n    b = a + 1\n    if True:\n        b = b + 1\n    return b").unwrap();
    assert_eq!(vm.eval("f(1)").unwrap().as_int(), Some(3));
    assert_eq!(vm.eval("a").unwrap().as_int(), Some(100));
    assert!(vm.eval("b").is_err());
}
#[test]
fn none_return_and_pass() {
    assert_eq!(value("def f() -> None:\n    pass\nf()"), "None");
    assert_eq!(value("def f() -> None:\n    return\nf()"), "None");
}
#[test]
fn rejects_invalid_function_programs() {
    for source in ["def f(x) -> int:\n    return 1", "def f(x: int):\n    return x", "def f(x: int) -> bool:\n    return x", "def f(x: int) -> int:\n    if x > 0:\n        return x", "def f(x: int) -> int:\n    return x\nf(True)", "def f(x: int) -> int:\n    return x\nf()", "def f(x: int) -> int:\n    return x\nf(1, 2)", "return 1", "def f(x: int, x: int) -> int:\n    return x", "def f() -> int:\n    return", "def f() -> int:\n    return missing"] {
        assert!(Interpreter::new().eval(source).is_err(), "{source}");
    }
}
#[test]
fn constructors_fields_and_bound_methods() {
    let mut vm = Interpreter::new(); vm.eval(COUNTER).unwrap();
    assert_eq!(vm.eval("c = Counter(10)\nc.add(5)").unwrap().as_int(), Some(15));
    assert_eq!(vm.eval("m = c.add\nm(2)").unwrap().as_int(), Some(17));
    assert_eq!(vm.eval("c.value").unwrap().as_int(), Some(17));
}
#[test]
fn instances_alias_but_constructors_are_independent() {
    let mut vm = Interpreter::new(); vm.eval(COUNTER).unwrap();
    vm.eval("a = Counter(1)\nb = a\nc = Counter(2)\nb.value = 9").unwrap();
    assert_eq!(vm.eval("a.value").unwrap().as_int(), Some(9));
    assert_eq!(vm.eval("c.value").unwrap().as_int(), Some(2));
    assert_eq!(vm.eval("a == b").unwrap().as_bool(), Some(true));
    assert_eq!(vm.eval("a == c").unwrap().as_bool(), Some(false));
}
#[test]
fn nominal_parameter_and_return_types() {
    let mut vm = Interpreter::new(); vm.eval(COUNTER).unwrap();
    vm.eval("def make(n: int) -> Counter:\n    return Counter(n)\ndef read(c: Counter) -> int:\n    return c.value").unwrap();
    assert_eq!(vm.eval("read(make(42))").unwrap().as_int(), Some(42));
    assert!(vm.eval("read(42)").is_err());
    assert!(vm.eval("c: Counter = True").is_err());
}
#[test]
fn default_constructor_and_empty_class() {
    assert_eq!(value("class Point:\n    x: int\np = Point()\np.x"), "0");
    assert_eq!(value("class Empty:\n    pass\ne = Empty()\ne == e"), "True");
}
#[test]
fn rejects_invalid_classes() {
    for source in ["class A:\n    x: int = True", "class A:\n    x: Unknown", "class A:\n    x: int = 0\na = A()\na.x = True", "class A:\n    pass\na = A()\na.missing = 1", "class A:\n    pass\na = A()\na.missing", "class A:\n    def f(self) -> int:\n        return True", "class A:\n    def __init__(self) -> int:\n        return 1", "class A:\n    def f(x: int) -> int:\n        return x", "class A:\n    x: int\n    x: bool", "class A:\n    def f(self) -> None:\n        pass\na = A()\na.f = 1", "class A:\n    pass\nclass B:\n    pass\na = A()\na = B()"] {
        assert!(Interpreter::new().eval(source).is_err(), "{source}");
    }
}
#[test]
fn errors_roll_back_aliased_mutations_and_unwind_calls() {
    let mut vm = Interpreter::new(); vm.eval(COUNTER).unwrap();
    vm.eval("a = Counter(1)\nb = a\ndef fail() -> int:\n    a.add(2)\n    return 1 / 0").unwrap();
    assert!(vm.eval("a.add(2)\nb.value = 7\nfail()").is_err());
    assert_eq!(vm.eval("a.value").unwrap().as_int(), Some(1));
    assert_eq!(vm.eval("b.value").unwrap().as_int(), Some(1));
    assert_eq!(vm.eval("a.add(1)").unwrap().as_int(), Some(2));
}
#[test]
fn failed_definitions_do_not_poison_the_session() {
    let mut vm = Interpreter::new();
    assert!(vm.eval("def f() -> int:\n    return True").is_err());
    vm.eval("def f() -> int:\n    return 42").unwrap();
    assert_eq!(vm.eval("f()").unwrap().as_int(), Some(42));
}
#[test]
fn recursion_limit_is_recoverable() {
    let mut vm = Interpreter::new();
    vm.eval("def f() -> int:\n    return f()").unwrap();
    assert!(vm.eval("f()").unwrap_err().contains("RecursionError"));
    assert_eq!(vm.eval("42").unwrap().as_int(), Some(42));
}
