use typy::compiler::Compiler;
use typy::object::Object;
use typy::parser::Parser;
use typy::symbol::Interner;
use typy::tokenizer::tokenize_str;
use typy::vm::VM;

fn run(input: &str) -> Object {
    let tokens = tokenize_str(input);
    let stmts = Parser::new(tokens).parse().unwrap();
    let mut interner = Interner::new();
    let bytecode = Compiler::new().compile(&stmts, &mut interner);
    VM::new().run(&bytecode, &interner).unwrap()
}

#[test]
fn evaluates_number_literal() {
    assert_eq!(run("42\n"), Object::int(42));
}

#[test]
fn evaluates_bool_literal() {
    assert_eq!(run("True\n"), Object::bool(true));
}

#[test]
fn evaluates_arithmetic() {
    assert_eq!(run("1 + 2\n"), Object::int(3));
    assert_eq!(run("10 - 3\n"), Object::int(7));
    assert_eq!(run("4 * 5\n"), Object::int(20));
    assert_eq!(run("20 / 4\n"), Object::int(5));
}

#[test]
fn evaluates_comparison() {
    assert_eq!(run("1 < 2\n"), Object::bool(true));
    assert_eq!(run("5 > 3\n"), Object::bool(true));
    assert_eq!(run("2 == 2\n"), Object::bool(true));
    assert_eq!(run("3 != 4\n"), Object::bool(true));
}

#[test]
fn evaluates_variable_declaration() {
    assert_eq!(run("x: int = 10\nx\n"), Object::int(10));
}

#[test]
fn evaluates_variable_assignment() {
    assert_eq!(run("x: int = 5\nx = 20\nx\n"), Object::int(20));
}

#[test]
fn evaluates_if_statement() {
    assert_eq!(run("x: int = 0\nif True:\n    x = 1\nx\n"), Object::int(1));
    assert_eq!(run("x: int = 0\nif False:\n    x = 1\nx\n"), Object::int(0));
}

#[test]
fn evaluates_if_else() {
    let code = "x: int = 0\nif False:\n    x = 1\nelse:\n    x = 2\nx\n";
    assert_eq!(run(code), Object::int(2));
}

#[test]
fn rejects_undefined_variable() {
    let tokens = tokenize_str("x\n");
    let stmts = Parser::new(tokens).parse().unwrap();
    let mut interner = Interner::new();
    let bytecode = Compiler::new().compile(&stmts, &mut interner);
    let result = VM::new().run(&bytecode, &interner);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("NameError"));
}
