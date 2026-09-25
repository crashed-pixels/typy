use typy::Interpreter as Session;
use typy::object::Object;

#[test]
fn infers_literals_and_expressions() {
    let mut session = Session::default();
    assert_eq!(session.eval("a = 123\na").unwrap(), Object::int(123));
    assert_eq!(session.eval("b = True\nb").unwrap(), Object::bool(true));
    assert_eq!(session.eval("c = a + 2\nc").unwrap(), Object::int(125));
    assert_eq!(session.eval("d = c > a\nd").unwrap(), Object::bool(true));
}

#[test]
fn inferred_type_is_fixed_across_submissions() {
    let mut session = Session::default();
    session.eval("a = 123").unwrap();
    assert!(session.eval("a = True").unwrap_err().contains("TypeError"));
    assert_eq!(session.eval("a = a + 1\na").unwrap(), Object::int(124));
}

#[test]
fn inferred_locals_resolve_through_nested_blocks() {
    let mut session = Session::default();
    assert_eq!(session.eval("if True:\n    a = 1\n    if True:\n        if True:\n            a = a + 1\n    a\n").unwrap(), Object::int(2));
    assert!(session.eval("a").unwrap_err().contains("NameError"));
}

#[test]
fn assignments_still_update_the_nearest_existing_binding() {
    let mut session = Session::default();
    assert_eq!(
        session.eval("a = 1\nif True:\n    a = 2\na").unwrap(),
        Object::int(2)
    );
    assert_eq!(
        session
            .eval("if True:\n    a: bool = True\n    a = False\na")
            .unwrap(),
        Object::int(2)
    );
}

#[test]
fn inference_works_in_elif_and_else() {
    for source in [
        "if False:\n    x = 1\nelif True:\n    x = 2\n    x\n",
        "if False:\n    x = 1\nelse:\n    x = 2\n    x\n",
    ] {
        assert_eq!(Session::default().eval(source).unwrap(), Object::int(2));
    }
}

#[test]
fn inferred_initializer_failure_can_be_retried_with_another_type() {
    let mut session = Session::default();
    assert!(
        session
            .eval("a = 1 / 0")
            .unwrap_err()
            .contains("ZeroDivisionError")
    );
    assert_eq!(session.eval("a = True\na").unwrap(), Object::bool(true));
}

#[test]
fn failed_multi_statement_submission_rolls_back_inference_and_values() {
    let mut session = Session::default();
    session.eval("a = 1").unwrap();
    assert!(session.eval("b = True\na = 99\n1 / 0").is_err());
    assert_eq!(session.eval("a").unwrap(), Object::int(1));
    assert_eq!(session.eval("b = 42\nb").unwrap(), Object::int(42));
    assert!(session.eval("c = 3\na = False").is_err());
    assert_eq!(session.eval("c = True\nc").unwrap(), Object::bool(true));
}

#[test]
fn self_reference_does_not_declare_an_uninitialized_name() {
    let mut session = Session::default();
    assert!(session.eval("a = a + 1").unwrap_err().contains("NameError"));
    assert_eq!(session.eval("a = 1\na").unwrap(), Object::int(1));
}
