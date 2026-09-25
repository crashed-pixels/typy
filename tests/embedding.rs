use typy::Interpreter;
use typy::object::Object;

#[test]
fn portable_interpreter_preserves_state_and_recovers_after_errors() {
    let mut interpreter = Interpreter::new();
    interpreter.eval("a = 123").unwrap();
    assert!(interpreter.eval("a = True").is_err());
    assert_eq!(interpreter.eval("a").unwrap(), Object::int(123));
    assert!(interpreter.eval("b = 1 / 0").is_err());
    assert_eq!(interpreter.eval("b = True\nb").unwrap(), Object::bool(true));
    assert!(interpreter.eval("@").is_err());
    assert_eq!(interpreter.eval("42").unwrap(), Object::int(42));
}

#[test]
fn host_can_capture_traces_without_stdout() {
    let mut interpreter = Interpreter::new();
    let mut trace = Vec::new();
    let result = interpreter
        .eval_with_trace("if True:\n    42\n", |event| {
            trace.push(event.to_string());
        })
        .unwrap();
    assert_eq!(result, Object::int(42));
    assert!(trace.iter().any(|line| line.contains("Frames (2)")));
    assert!(trace.last().unwrap().contains("Frames (1)"));
    assert!(interpreter.eval("if True:\n    1 / 0\n").is_err());
    interpreter
        .eval_with_trace("43", |event| assert_eq!(event.frames.len(), 1))
        .unwrap();
}
