use typy::compiler::Compiler;
use typy::parser::Parser;
use typy::symbol::Interner;
use typy::tokenizer::tokenize;
use typy::types::TypeChecker;
use typy::vm::VM;

fn execute_program(source: &str) -> String {
    let tokens = tokenize(source.to_string());
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().expect("Parsing error");

    let mut interner = Interner::new();
    let mut checker = TypeChecker::new();
    if let Err(e) = checker.check(&stmts, &mut interner) {
        return e;
    }

    let compiler = Compiler::new();
    let bytecode = compiler.compile(&stmts, &mut interner);

    let mut vm = VM::new();
    match vm.run(&bytecode, &interner) {
        Ok(result) => format!("{}", result),
        Err(e) => e.to_string(),
    }
}

#[test]
fn test_int_sum() {
    assert_eq!(execute_program("2 + 2"), "4".to_string());
    assert_eq!(execute_program("10 + 10"), "20".to_string());
    assert_eq!(execute_program("0 + 0"), "0".to_string());
    assert_eq!(execute_program("123 + 0"), "123".to_string());
}

#[test]
fn test_int_sub() {
    let result = execute_program("5 - 2");
    assert_eq!(result, "3".to_string());
}

#[test]
fn test_int_mul() {
    let result = execute_program("2 * 5");
    assert_eq!(result, "10".to_string());
}

#[test]
fn test_int_div() {
    let result = execute_program("10 / 5");
    assert_eq!(result, "2".to_string());
}

#[test]
fn test_int_zero_div() {
    let result = execute_program("10 / 0");
    assert_eq!(result, "ZeroDivisionError: division by zero".to_string());
}

#[test]
fn test_int_vars() {
    let result = execute_program("x: int = 10\nx + 5");
    assert_eq!(result, "15".to_string());
}

#[test]
fn test_static_name_error() {
    let result = execute_program("a + 1");
    assert_eq!(result, "NameError: name 'a' is not defined");
}

#[test]
fn test_multiple_vars_persistence() {
    let result = execute_program("x: int = 10\ny: int = 20\nx + y");
    assert_eq!(result, "30".to_string());
}

#[test]
fn test_bool_with_type_annotation() {
    let result = execute_program("x: bool = True\ny: bool = False\nx == y");
    assert_eq!(result, "False".to_string());
}

#[test]
fn test_bool_literal() {
    let result = execute_program("True");
    assert_eq!(result, "True".to_string());
    let result = execute_program("False");
    assert_eq!(result, "False".to_string());
}

#[test]
fn test_comparison() {
    let result = execute_program("10 > 5");
    assert_eq!(result, "True".to_string());
    let result = execute_program("2 == 2");
    assert_eq!(result, "True".to_string());
    let result = execute_program("3 < 1");
    assert_eq!(result, "False".to_string());
}

#[test]
fn test_precedence() {
    let result = execute_program("2 + 3 < 10");
    assert_eq!(result, "True".to_string());
    let result = execute_program("2 < 3 + 4");
    assert_eq!(result, "True".to_string());
}

#[test]
fn test_static_type_error() {
    let result = execute_program("10 + True");
    assert_eq!(
        result,
        "TypeError: unsupported operand types for arithmetic: 'int' and 'bool'".to_string()
    );
    let result = execute_program("10 < True");
    assert_eq!(
        result,
        "TypeError: cannot compare 'int' and 'bool'".to_string()
    );
}

#[test]
fn test_if_true_branch() {
    let source = "if True:\n    10\n";
    assert_eq!(execute_program(source), "10");
}

#[test]
fn test_if_false_branch() {
    let source = "if False:\n    10\nelse:\n    20\n";
    assert_eq!(execute_program(source), "20");
}

#[test]
fn test_if_assign_in_block() {
    let source = "x: int = 5\nif x > 3:\n    x = 100\nx\n";
    assert_eq!(execute_program(source), "100");
}

#[test]
fn test_nested_if() {
    let source =
        "x: int = 10\nif True:\n    if False:\n        x = 1\n    else:\n        x = 2\nx\n";
    assert_eq!(execute_program(source), "2");
}

#[test]
fn test_static_if_condition_error() {
    let source = "if 1:\n    10\n";
    assert_eq!(
        execute_program(source),
        "TypeError: if condition must be 'bool', got 'int'"
    );
}

#[test]
fn test_precedence_still_works() {
    let source = "2 + 3 < 10\n";
    assert_eq!(execute_program(source), "True");
}

#[test]
fn test_elif_selects_matching_branch() {
    let source = "x: int = 2\nif x == 1:\n    10\nelif x == 2:\n    20\nelse:\n    30\n";
    assert_eq!(execute_program(source), "20");
}

#[test]
fn test_elif_skips_later_branches() {
    let source = "x: int = 10\nif True:\n    10\nelif True:\n    20\nelse:\n    30\n";
    assert_eq!(execute_program(source), "10");
}

#[test]
fn test_elif_falls_through_without_else() {
    let source = "x: int = 10\nif False:\n    10\nelif True:\n    20\n";
    assert_eq!(execute_program(source), "20");
}

#[test]
fn test_scope_bad() {
    let source = "if True:\n    x: int = 10\nx + 10\n";
    assert_eq!(
        execute_program(source),
        "NameError: name 'x' is not defined"
    );
}

#[test]
fn test_scope_good() {
    let source = "x: int = 10\nif True:\n    x: bool = False\nx + 15\n";
    assert_eq!(execute_program(source), "25");
}
