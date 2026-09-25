use typy::object::Object;

#[test]
fn adds_integers() {
    let a = Object::int(5);
    let b = Object::int(3);
    assert_eq!(a.add(&b).unwrap(), Object::int(8));
}

#[test]
fn subtracts_integers() {
    let a = Object::int(10);
    let b = Object::int(4);
    assert_eq!(a.sub(&b).unwrap(), Object::int(6));
}

#[test]
fn multiplies_integers() {
    let a = Object::int(6);
    let b = Object::int(7);
    assert_eq!(a.mul(&b).unwrap(), Object::int(42));
}

#[test]
fn divides_integers() {
    let a = Object::int(20);
    let b = Object::int(4);
    assert_eq!(a.div(&b).unwrap(), Object::int(5));
}

#[test]
fn rejects_division_by_zero() {
    let a = Object::int(10);
    let b = Object::int(0);
    let result = a.div(&b);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ZeroDivisionError"));
}

#[test]
fn rejects_arithmetic_with_bool() {
    let a = Object::int(5);
    let b = Object::bool(true);
    assert!(a.add(&b).is_err());
    assert!(a.sub(&b).is_err());
    assert!(a.mul(&b).is_err());
    assert!(a.div(&b).is_err());
}

#[test]
fn compares_integers() {
    let a = Object::int(5);
    let b = Object::int(10);

    assert!(a.lt(&b).unwrap());
    assert!(!a.gt(&b).unwrap());
    assert!(a.le(&b).unwrap());
    assert!(!a.ge(&b).unwrap());
}

#[test]
fn checks_equality() {
    let a = Object::int(5);
    let b = Object::int(5);
    let c = Object::int(10);

    assert!(a.eq(&b).unwrap());
    assert!(!a.eq(&c).unwrap());

    let t1 = Object::bool(true);
    let t2 = Object::bool(true);
    let f = Object::bool(false);

    assert!(t1.eq(&t2).unwrap());
    assert!(!t1.eq(&f).unwrap());
}

#[test]
fn checks_inequality() {
    let a = Object::int(5);
    let b = Object::int(10);

    assert!(a.ne(&b).unwrap());
    assert!(!a.ne(&a).unwrap());
}

#[test]
fn rejects_comparison_of_different_types() {
    let a = Object::int(5);
    let b = Object::bool(true);

    assert!(a.eq(&b).is_err());
    assert!(a.ne(&b).is_err());
    assert!(a.lt(&b).is_err());
}

#[test]
fn displays_correctly() {
    assert_eq!(format!("{}", Object::int(42)), "42");
    assert_eq!(format!("{}", Object::bool(true)), "True");
    assert_eq!(format!("{}", Object::bool(false)), "False");
    assert_eq!(format!("{}", Object::none()), "None");
}

#[test]
fn type_names_are_correct() {
    assert_eq!(Object::int(0).type_name(), "int");
    assert_eq!(Object::bool(false).type_name(), "bool");
    assert_eq!(Object::none().type_name(), "NoneType");
}
