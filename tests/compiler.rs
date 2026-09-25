use typy::compiler::{Compiler, Instruction};
use typy::object::Object;
use typy::parser::Parser;
use typy::symbol::Interner;
use typy::tokenizer::tokenize_str;

fn compile(input: &str) -> Vec<Instruction> {
    let tokens = tokenize_str(input);
    let stmts = Parser::new(tokens).parse().unwrap();
    let mut interner = Interner::new();
    Compiler::new().compile(&stmts, &mut interner)
}

#[test]
fn compiles_number_literal() {
    let code = compile("42\n");

    assert_eq!(code.len(), 2);
    assert_eq!(code[0], Instruction::LoadConst(Object::Int(42)));
    assert_eq!(code[1], Instruction::SetResult);
}

#[test]
fn compiles_bool_literal() {
    let code = compile("True\n");

    assert_eq!(code.len(), 2);
    assert_eq!(code[0], Instruction::LoadConst(Object::Bool(true)));
    assert_eq!(code[1], Instruction::SetResult);
}

#[test]
fn compiles_variable_declaration() {
    let code = compile("x: int = 10\n");

    // Should have LoadConst and StoreName
    assert!(
        code.iter()
            .any(|i| matches!(i, Instruction::LoadConst(Object::Int(10))))
    );
    assert!(code.iter().any(|i| matches!(i, Instruction::StoreName(_))));
}

#[test]
fn compiles_variable_declaration_without_initializer() {
    let code = compile("x: int\n");

    // Should emit default value (0) and StoreName
    assert!(
        code.iter()
            .any(|i| matches!(i, Instruction::LoadConst(Object::Int(0))))
    );
    assert!(code.iter().any(|i| matches!(i, Instruction::StoreName(_))));
}

#[test]
fn compiles_assignment() {
    let code = compile("x: int = 5\nx = 10\n");

    // Second assignment should use StoreName
    let store_count = code
        .iter()
        .filter(|i| matches!(i, Instruction::StoreName(_)))
        .count();
    assert_eq!(store_count, 2);
}

#[test]
fn compiles_binary_operation() {
    let code = compile("x: int = 1 + 2\n");

    assert!(
        code.iter()
            .any(|i| matches!(i, Instruction::LoadConst(Object::Int(1))))
    );
    assert!(
        code.iter()
            .any(|i| matches!(i, Instruction::LoadConst(Object::Int(2))))
    );
    assert!(code.iter().any(|i| matches!(i, Instruction::Add)));
}

#[test]
fn compiles_if_statement() {
    let code = compile("if true:\n    x: int = 1\n");

    // Should have JumpIfFalse and Jump instructions
    assert!(
        code.iter()
            .any(|i| matches!(i, Instruction::JumpIfFalse(_)))
    );
    assert!(code.iter().any(|i| matches!(i, Instruction::Jump(_))));
    assert!(code.iter().any(|i| matches!(i, Instruction::EnterBlock(_))));
    assert!(code.iter().any(|i| matches!(i, Instruction::ExitBlock)));
}

#[test]
fn compiles_if_elif_else() {
    let code =
        compile("if true:\n    x: int = 1\nelif false:\n    y: int = 2\nelse:\n    z: int = 3\n");

    // Should have multiple JumpIfFalse and Jump instructions
    let jump_if_false_count = code
        .iter()
        .filter(|i| matches!(i, Instruction::JumpIfFalse(_)))
        .count();
    let jump_count = code
        .iter()
        .filter(|i| matches!(i, Instruction::Jump(_)))
        .count();

    assert_eq!(jump_if_false_count, 2); // if and elif
    assert_eq!(jump_count, 2); // end of if and end of elif
}

#[test]
fn compiles_local_variables_in_block() {
    let code = compile("if true:\n    x: int = 1\n");

    // Should use StoreLocal instead of StoreName
    assert!(code.iter().any(|i| matches!(i, Instruction::StoreLocal(_))));
}

#[test]
fn compiles_comparison_operators() {
    let code = compile("x: bool = 1 < 2\n");

    assert!(code.iter().any(|i| matches!(i, Instruction::Less)));
}
