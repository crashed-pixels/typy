use typy::object::{BOOL_TYPE, INT_TYPE, NONE_TYPE, OBJECT_TYPE, TYPE_TYPE, Object};
use typy::types::Type;

#[test]
fn builtins_have_shared_type_objects_and_a_metatype() {
    for (object, typ) in [(Object::int(1), &INT_TYPE), (Object::bool(true), &BOOL_TYPE), (Object::none(), &NONE_TYPE)] {
        assert!(core::ptr::eq(object.type_object(), typ));
        assert!(core::ptr::eq(typ.header().type_object(), &TYPE_TYPE));
        assert!(core::ptr::eq(typ.base().unwrap(), &OBJECT_TYPE));
        assert_eq!(object.type_name(), typ.name());
    }
    assert!(core::ptr::eq(TYPE_TYPE.header().type_object(), &TYPE_TYPE));
    assert!(OBJECT_TYPE.base().is_none());
    assert!(core::ptr::eq(Type::Int.type_object(), &INT_TYPE));
    assert!(core::ptr::eq(Type::Bool.type_object(), &BOOL_TYPE));
}

#[test]
fn object_handles_share_identity_but_equality_compares_values() {
    let value = Object::int(42);
    assert_eq!(value.strong_count(), 1);
    let alias = value.clone();
    assert!(value.is_identical(&alias));
    assert_eq!(value.strong_count(), 2);
    let equal = Object::int(42);
    assert!(!value.is_identical(&equal));
    assert_eq!(value, equal);
    drop(alias);
    assert_eq!(value.strong_count(), 1);
}

#[test]
fn typed_accessors_do_not_coerce_bool_to_int() {
    assert_eq!(Object::int(1).as_int(), Some(1));
    assert_eq!(Object::int(1).as_bool(), None);
    assert_eq!(Object::bool(true).as_bool(), Some(true));
    assert_eq!(Object::bool(true).as_int(), None);
    assert!(Object::none().is_none());
    assert!(!Object::int(0).is_none());
    assert!(Object::bool(true).add(&Object::int(1)).is_err());
}

#[test]
fn arithmetic_returns_a_new_typed_object_without_mutating_aliases() {
    let first = Object::int(40);
    let alias = first.clone();
    let sum = first.add(&Object::int(2)).unwrap();
    assert_eq!(sum.as_int(), Some(42));
    assert_eq!(alias.as_int(), Some(40));
    assert!(core::ptr::eq(sum.type_object(), &INT_TYPE));
    assert_eq!(Object::none().to_string(), "None");
}
