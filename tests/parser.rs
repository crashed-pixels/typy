use typy::parser::{Expr, Operator, Parser, Stmt};
use typy::tokenizer::tokenize_str;
use typy::types::Type;

fn parse(input: &str) -> Vec<Stmt> {
    let tokens = tokenize_str(input);
    Parser::new(tokens).parse().unwrap()
}

#[test]
fn parses_variable_declaration_with_type() {
    let stmts = parse("x: int\n");

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::VariableDecl {
            name,
            typ,
            initializer,
        } => {
            assert_eq!(name, "x");
            assert_eq!(*typ, Type::Int);
            assert!(initializer.is_none());
        }
        _ => panic!("Expected VariableDecl"),
    }
}

#[test]
fn parses_variable_declaration_with_initializer() {
    let stmts = parse("x: int = 42\n");

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::VariableDecl {
            name,
            typ,
            initializer,
        } => {
            assert_eq!(name, "x");
            assert_eq!(*typ, Type::Int);
            assert!(matches!(initializer, Some(Expr::Number(42))));
        }
        _ => panic!("Expected VariableDecl"),
    }
}

#[test]
fn parses_assignment() {
    let stmts = parse("x = 10\n");

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::Assign { name, value } => {
            assert_eq!(name, "x");
            assert!(matches!(value, Expr::Number(10)));
        }
        _ => panic!("Expected Assign"),
    }
}

#[test]
fn parses_binary_expression() {
    let stmts = parse("x = 1 + 2 * 3\n");

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::Assign { name, value } => {
            assert_eq!(name, "x");
            // Should parse as: 1 + (2 * 3) due to precedence
            match value {
                Expr::BinaryOp { left, op, right } => {
                    assert_eq!(*op, Operator::Plus);
                    assert!(matches!(**left, Expr::Number(1)));
                    assert!(matches!(**right, Expr::BinaryOp { .. }));
                }
                _ => panic!("Expected BinaryOp"),
            }
        }
        _ => panic!("Expected Assign"),
    }
}

#[test]
fn parses_if_statement() {
    let stmts = parse("if x:\n    y\n");

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            assert!(matches!(condition, Expr::Name(_)));
            assert_eq!(then_branch.len(), 1);
            assert!(elif_branches.is_empty());
            assert!(else_branch.is_none());
        }
        _ => panic!("Expected If"),
    }
}

#[test]
fn parses_if_elif_else() {
    let stmts = parse("if x:\n    y\nelif z:\n    w\nelse:\n    v\n");

    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            assert!(matches!(condition, Expr::Name(_)));
            assert_eq!(then_branch.len(), 1);
            assert_eq!(elif_branches.len(), 1);
            assert!(else_branch.is_some());
        }
        _ => panic!("Expected If"),
    }
}

#[test]
fn parses_nominal_annotation_for_type_checker_resolution() {
    let statements = parse("x: Point\n");
    assert!(
        matches!(&statements[0], Stmt::VariableDecl { typ: Type::Named(name), .. } if name == "Point")
    );
}
