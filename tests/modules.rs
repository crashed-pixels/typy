use std::collections::BTreeMap;
use typy::modules::{ModuleLoader, Program, SourceModule};
use typy::Interpreter;

#[derive(Default)]
struct Sources(BTreeMap<String, String>);
impl ModuleLoader for Sources {
    fn load(&mut self, _importer: &str, module: &str) -> Result<SourceModule, String> {
        self.0.get(module).map(|source| SourceModule::new(format!("{module}.tp"), source.clone()))
            .ok_or_else(|| format!("ImportError: missing {module}"))
    }
}
fn sources(items: &[(&str, &str)]) -> Sources {
    Sources(items.iter().map(|(name, source)| (name.to_string(), source.to_string())).collect())
}
fn run(source: &str, items: &[(&str, &str)]) -> Result<String, String> {
    let program = Program::compile(SourceModule::new("main.tp", source), &mut sources(items))?;
    let mut text = String::new();
    program.run(|line| { text.push_str(line); Ok(()) })?;
    Ok(text)
}

#[test]
fn main_is_called_once_and_expressions_are_silent() {
    assert_eq!(run("def helper() -> int:\n    print(7)\n    return 42\ndef main() -> None:\n    helper()\n    999\n", &[]).unwrap(), "7\n");
}
#[test]
fn print_is_a_variadic_builtin_object_returning_none() {
    let mut i = Interpreter::new();
    let mut text = String::new();
    let result = i.eval_with_output("p = print\np(42, True, False, None)\np()", |line| { text.push_str(line); Ok(()) }).unwrap();
    assert!(result.is_none());
    assert_eq!(text, "42 True False None\n\n");
    assert!(i.eval("print(unknown)").unwrap_err().contains("NameError"));
    assert!(i.eval("n: int = print(1)").unwrap_err().contains("TypeError"));
}
#[test]
fn output_failure_rolls_back_state_but_output_is_not_transactional() {
    let mut i = Interpreter::new();
    i.eval("x = 1").unwrap();
    assert!(i.eval_with_output("x = 2\nprint(x)", |_| Err("broken output".into())).is_err());
    assert_eq!(i.eval("x").unwrap().as_int(), Some(1));
    let mut text = String::new();
    assert!(i.eval_with_output("print(3)\n1 / 0", |line| { text.push_str(line); Ok(()) }).is_err());
    assert_eq!(text, "3\n");
}
#[test]
fn named_and_qualified_imports_keep_module_globals_isolated() {
    let items = [("left", "value = 10\ndef get() -> int:\n    return value\n"), ("right", "value = 20\ndef get() -> int:\n    return value\n")];
    assert_eq!(run("import left as l\nfrom right import get as r\nvalue = 99\ndef main() -> None:\n    print(l.get(), r(), value)\n", &items).unwrap(), "10 20 99\n");
}
#[test]
fn imported_classes_keep_nominal_identity_and_methods() {
    let items = [("model", "class Point:\n    x: int = 4\n    def get(self) -> int:\n        return self.x\n")];
    assert_eq!(run("import model\nfrom model import Point as P\ndef read(p: model.Point) -> int:\n    return p.get()\ndef main() -> None:\n    p: P = P()\n    p.x = 42\n    print(read(p))\n", &items).unwrap(), "42\n");
    let bad = "import model\nclass Point:\n    x: int = 0\ndef read(p: model.Point) -> int:\n    return p.x\ndef main() -> None:\n    read(Point())\n";
    assert!(run(bad, &items).unwrap_err().contains("TypeError"));
}
#[test]
fn imports_are_inserted_in_order_and_not_visible_early() {
    let items = [("math", "def twice(n: int) -> int:\n    return n + n\n")];
    assert_eq!(run("from math import twice\ndef main() -> None:\n    print(twice(21))\n", &items).unwrap(), "42\n");
    assert!(run("def main() -> None:\n    print(twice(21))\nfrom math import twice\n", &items).is_err());
}
#[test]
fn transitive_reexports_and_diamond_imports_share_one_module() {
    let items = [("base", "count = 0\ndef next() -> int:\n    count = count + 1\n    return count\n"), ("a", "from base import next\n"), ("b", "from base import next\n"), ("lib", "import a\nfrom b import next\n")];
    assert_eq!(run("import lib\ndef main() -> None:\n    print(lib.a.next(), lib.next())\n", &items).unwrap(), "1 2\n");
}
#[test]
fn explicit_exports_do_not_leak_dependencies() {
    let items = [("helper", "secret = 9\ndef get() -> int:\n    return secret\n"), ("lib", "from helper import get\n")];
    for source in ["from lib import secret\ndef main() -> None:\n    pass\n", "from lib import get\ndef main() -> None:\n    print(secret)\n"] {
        assert!(run(source, &items).is_err());
    }
}
#[test]
fn wildcard_cycles_missing_names_and_nested_imports_are_errors() {
    let items = [("a", "import b\n"), ("b", "import a\n"), ("ok", "x = 1\n")];
    for source in ["from ok import *\ndef main() -> None:\n    pass\n", "import a\ndef main() -> None:\n    pass\n", "import missing\ndef main() -> None:\n    pass\n", "from ok import missing\ndef main() -> None:\n    pass\n", "def main() -> None:\n    import ok\n"] {
        assert!(run(source, &items).is_err(), "{source}");
    }
    let error = run("import a\ndef main() -> None:\n    pass\n", &items).unwrap_err();
    assert!(error.contains("cycle") && error.contains("a.tp") && error.contains("b.tp"), "{error}");
}
#[test]
fn invalid_entries_and_top_level_execution_are_rejected_before_output() {
    for source in ["pass\n", "main = 1\n", "def main(n: int) -> None:\n    pass\n", "def main() -> int:\n    return 0\n", "print(1)\ndef main() -> None:\n    pass\n", "x = print(1)\ndef main() -> None:\n    pass\n", "if True:\n    print(1)\ndef main() -> None:\n    pass\n"] {
        assert!(run(source, &[]).is_err(), "{source}");
    }
    assert!(Program::compile(SourceModule::new("other.tp", "def main() -> None:\n    pass\n"), &mut Sources::default()).is_err());
    assert!(run("from lib import main\n", &[("lib", "def main() -> None:\n    pass\n")]).is_err());
}
#[test]
fn module_imports_cannot_be_reassigned_or_used_as_values() {
    let items = [("data", "x = 1\n")];
    for body in ["data.x = 2", "print(data)", "data = 2"] {
        assert!(run(&format!("import data\ndef main() -> None:\n    {body}\n"), &items).is_err());
    }
    assert!(run("from data import x\ndef main() -> None:\n    x = 2\n", &items).is_err());
}
#[test]
fn locals_and_explicit_shadowing_do_not_capture_module_names() {
    let items = [("data", "x = 10\ndef add(x: int) -> int:\n    if True:\n        x: int = 2\n        return x\n    return x\n")];
    assert_eq!(run("import data\ndef main() -> None:\n    data: int = 5\n    print(data)\n", &items).unwrap(), "5\n");
    assert_eq!(run("from data import add\ndef main() -> None:\n    print(add(99))\n", &items).unwrap(), "2\n");
}
