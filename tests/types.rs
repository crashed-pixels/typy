use typy::parser::Parser;
use typy::symbol::Interner;
use typy::tokenizer::tokenize_str;
use typy::types::TypeChecker;

fn check(input: &str) -> Result<(), String> {
    let tokens = tokenize_str(input);
    let stmts = Parser::new(tokens).parse().unwrap();
    let mut interner = Interner::new();
    let mut checker = TypeChecker::new();
    checker.check(&stmts, &mut interner)
}

#[test]
fn accepts_valid_variable_declaration() {
    assert!(check("x: int = 42\n").is_ok());
}

#[test]
fn rejects_type_mismatch_in_declaration() {
    let result = check("x: int = True\n");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("TypeError"));
}

#[test]
fn rejects_undeclared_variable() {
    let result = check("x\n");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not defined"));
}

#[test]
fn rejects_type_mismatch_in_assignment() {
    let result = check("x: int = 5\nx = True\n");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("TypeError"));
}

#[test]
fn accepts_valid_arithmetic() {
    assert!(check("x: int = 1 + 2 * 3\n").is_ok());
}

#[test]
fn rejects_arithmetic_with_bool() {
    let result = check("x: int = True + 1\n");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unsupported operand types"));
}

#[test]
fn accepts_valid_comparison() {
    assert!(check("x: bool = 1 < 2\n").is_ok());
}

#[test]
fn rejects_comparison_of_different_types() {
    let result = check("x: bool = 1 == True\n");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot compare"));
}

#[test]
fn accepts_valid_if_statement() {
    assert!(check("if True:\n    x: int = 1\n").is_ok());
}

#[test]
fn rejects_non_bool_condition() {
    let result = check("if 1:\n    x: int = 1\n");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be 'bool'"));
}

#[test]
fn rejects_duplicate_declaration() {
    let result = check("x: int = 1\nx: int = 2\n");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already declared"));
}
