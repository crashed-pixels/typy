use std::io::Write;
use std::process::{Command, Stdio};
use typy::compiler::{Compiler, Instruction};
use typy::object::Object;
use typy::parser::{Parser, Stmt};
use typy::symbol::Interner;
use typy::tokenizer::tokenize_str;
use typy::types::TypeChecker;
use typy::vm::VM;

fn parse(source: &str) -> Vec<Stmt> {
    Parser::new(tokenize_str(source)).parse().unwrap()
}

fn run(source: &str) -> Result<Object, String> {
    let ast = parse(source);
    let mut interner = Interner::new();
    TypeChecker::new().check(&ast, &mut interner)?;
    let code = Compiler::new().compile(&ast, &mut interner);
    VM::new().run(&code, &interner, false)
}

fn repl(source: &str, debug: bool) -> (String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_typy"));
    if debug { command.arg("--debug"); }
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    (String::from_utf8(output.stdout).unwrap(), String::from_utf8(output.stderr).unwrap())
}

fn values(stdout: &str) -> Vec<&str> {
    stdout.lines().map(|line| line.trim_start_matches(">>> ").trim_start_matches("... "))
        .map(|line| line.trim_start_matches(">>> ").trim_start_matches("... "))
        .filter(|line| line.parse::<i64>().is_ok() || *line == "True" || *line == "False").collect()
}

#[test]
fn enclosing_local_reads_and_writes_preserve_global() {
    let source = "x: int = 100\nif True:\n    x: int = 10\n    if True:\n        x = 20\nx\n";
    assert_eq!(run(source).unwrap(), Object::Int(100));
    assert_eq!(run("if True:\n    x: int = 10\n    if True:\n        if True:\n            x = x + 1\n    x\n").unwrap(), Object::Int(11));
    assert_eq!(run("if True:\n    x: int = 10\n    if True:\n        x: int = x + 1\n        if True:\n            x = x + 1\n        x\n").unwrap(), Object::Int(12));
}

#[test]
fn failed_check_rolls_back_scopes_and_declarations() {
    let mut checker = TypeChecker::new();
    let mut interner = Interner::new();
    checker.check(&parse("x: int = 1\n"), &mut interner).unwrap();
    for _ in 0..3 {
        assert!(checker.check(&parse("y: int = 2\nif True:\n    x: bool = True\n    1 + True\n"), &mut interner).is_err());
        assert!(checker.check(&parse("x = False\n"), &mut interner).is_err());
    }
    checker.check(&parse("y: int = 3\nx = 2\n"), &mut interner).unwrap();
}

#[test]
fn repl_failed_initializer_can_be_retried() {
    let (out, err) = repl("x: int = 1 / 0\nx: int = 42\nx\n", false);
    assert_eq!(err.lines().count(), 1, "{err}");
    assert!(err.contains("ZeroDivisionError"));
    assert_eq!(values(&out).last(), Some(&"42"), "{out}");
}

#[test]
fn repl_failed_block_rolls_back_executed_assignments() {
    let (out, err) = repl("x: int = 1\nif True:\n    x = 99\n    1 / 0\n\nx\n", false);
    assert!(err.contains("ZeroDivisionError"));
    assert_eq!(values(&out).last(), Some(&"1"), "{out}");
}

#[test]
fn vm_cleans_operands_after_success_and_errors() {
    let mut vm = VM::new();
    let interner = Interner::new();
    assert_eq!(vm.run(&[Instruction::LoadConst(Object::Int(11)), Instruction::LoadConst(Object::Int(22))], &interner, false).unwrap(), Object::Int(22));
    assert_eq!(vm.run(&[], &interner, false).unwrap(), Object::None);
    for _ in 0..3 {
        assert!(vm.run(&[Instruction::EnterBlock(0), Instruction::LoadConst(Object::Int(11)), Instruction::LoadConst(Object::Int(1)), Instruction::LoadConst(Object::Int(0)), Instruction::Divide], &interner, false).is_err());
        assert_eq!(vm.run(&[], &interner, false).unwrap(), Object::None);
        assert!(vm.run(&[Instruction::ExitBlock], &interner, false).is_err());
    }
}

#[test]
fn overflow_is_a_language_error() {
    for source in ["9223372036854775807 + 1", "(0 - 9223372036854775807 - 1) - 1", "9223372036854775807 * 2", "(0 - 9223372036854775807 - 1) / (0 - 1)"] {
        assert!(run(source).unwrap_err().contains("OverflowError"), "{source}");
    }
    assert_eq!(run("9223372036854775807 + 0").unwrap(), Object::Int(i64::MAX));
    assert_eq!(run("(0 - 9223372036854775807 - 1) / 1").unwrap(), Object::Int(i64::MIN));
}

#[test]
fn repl_recovers_from_lexical_errors() {
    for source in ["@\n42\n", "!\n42\n", "9223372036854775808\n42\n", "if True:\n    1\n  2\n\n42\n"] {
        let (out, err) = repl(source, false);
        assert!(err.contains("Error:"), "{err}");
        assert_eq!(values(&out).last(), Some(&"42"), "{out}");
    }
}

#[test]
fn boolean_ordering_is_rejected_statically() {
    for op in ["<", ">", "<=", ">="] {
        let mut checker = TypeChecker::new();
        assert!(checker.check(&parse(&format!("True {op} False")), &mut Interner::new()).is_err());
    }
    assert_eq!(run("True == False").unwrap(), Object::Bool(false));
    assert_eq!(run("True != False").unwrap(), Object::Bool(true));
}

#[test]
fn adjacent_statements_require_newlines() {
    for source in ["1 2", "x: int = 1 y: int = 2", "if True:\n    1 2\n", "if True:\n    x: int = 1 x = 2\n"] {
        assert!(Parser::new(tokenize_str(source)).parse().is_err(), "{source}");
    }
}

#[test]
fn blank_lines_preserve_block_structure() {
    for newline in ["\n", "\r\n"] {
        for blank in ["", "    ", "\t"] {
            let source = format!("if True:\n{blank}\n    1\n{blank}\n    42\n").replace('\n', newline);
            assert_eq!(run(&source).unwrap(), Object::Int(42));
        }
    }
}

#[test]
fn repl_executes_pending_block_at_eof() {
    let (out, err) = repl("if True:\n    42\n", false);
    assert!(err.is_empty(), "{err}");
    assert_eq!(values(&out), ["42"], "{out}");
}

#[test]
fn repl_preserves_consecutive_block_headers() {
    let (out, err) = repl("if True:\n    1\nif True:\n    2\n\n", false);
    assert!(err.is_empty(), "{err}");
    assert_eq!(values(&out), ["1", "2"], "{out}");
}

#[test]
fn repl_scope_errors_do_not_change_global_type() {
    let (out, err) = repl("x: int = 1\nif True:\n    x: bool = True\n    1 + True\n\nx = False\nx\n", false);
    assert_eq!(err.lines().count(), 2, "{err}");
    assert_eq!(values(&out).last(), Some(&"1"), "{out}");
}

#[test]
fn repl_empty_submission_has_no_stale_value() {
    let (out, err) = repl("if True:\n    11\n    22\n\n\n", false);
    assert!(err.is_empty(), "{err}");
    assert_eq!(values(&out), ["22"], "{out}");
}

#[test]
fn repl_unwinds_failed_frames() {
    let (out, err) = repl("if True:\n    1 / 0\n\n42\n", true);
    assert!(err.contains("ZeroDivisionError"));
    let state = out.lines().find(|line| line.contains("LoadConst(Int(42)) | Stack")).unwrap();
    assert!(state.contains("Frames (1)"), "{state}");
}

#[test]
fn repl_keeps_elif_else_and_dedented_expressions() {
    let (out, err) = repl("if False:\n    1\nelif False:\n    2\nelse:\n    3\n42\n", false);
    assert!(err.is_empty(), "{err}");
    assert_eq!(values(&out), ["3", "42"], "{out}");
}

#[test]
fn file_lexical_error_has_normal_exit_status() {
    let path = std::env::temp_dir().join(format!("typy_issue_five_{}.tp", std::process::id()));
    std::fs::write(&path, "@\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_typy")).arg(&path).output().unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr).unwrap().contains("SyntaxError"));
}

#[test]
fn vm_rolls_back_all_globals_on_failed_run() {
    let mut vm = VM::new();
    let mut interner = Interner::new();
    let initial = Compiler::new().compile(&parse("x: int = 1"), &mut interner);
    vm.run(&initial, &interner, false).unwrap();
    let failed = Compiler::new().compile(&parse("y: int = 2\nx = 99\n1 / 0"), &mut interner);
    assert!(vm.run(&failed, &interner, false).is_err());
    let read_x = Compiler::new().compile(&parse("x"), &mut interner);
    assert_eq!(vm.run(&read_x, &interner, false).unwrap(), Object::Int(1));
    let read_y = Compiler::new().compile(&parse("y"), &mut interner);
    assert!(vm.run(&read_y, &interner, false).is_err());
}

#[test]
fn assignments_are_silent_and_do_not_leave_operands() {
    assert_eq!(run("x: int = 1\nx = 2").unwrap(), Object::None);
    assert_eq!(run("11\nx: int = 1\nx = 2").unwrap(), Object::Int(11));
    assert_eq!(run("11\nif False:\n    22\n").unwrap(), Object::Int(11));
}
