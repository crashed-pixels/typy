use alloc::{borrow::Cow, rc::Rc, string::String};
use core::fmt;

/// An immutable header that owns a class descriptor or borrows a builtin.
pub struct ObjectHeader {
    typ: TypeReference,
}
enum TypeReference {
    Builtin(&'static TypeObject),
    User(Rc<TypeObject>),
}
impl ObjectHeader {
    pub(super) fn builtin(typ: &'static TypeObject) -> Self {
        Self {
            typ: TypeReference::Builtin(typ),
        }
    }
    pub(super) fn user(typ: Rc<TypeObject>) -> Self {
        Self {
            typ: TypeReference::User(typ),
        }
    }
    pub fn type_object(&self) -> &TypeObject {
        match &self.typ {
            TypeReference::Builtin(typ) => typ,
            TypeReference::User(typ) => typ,
        }
    }
}

/// Builtin descriptors are static; user descriptors are reference-counted.
/// Every descriptor has the same metatype, so its header is a computed view.
pub struct TypeObject {
    name: Cow<'static, str>,
    base: Option<&'static TypeObject>,
}
impl TypeObject {
    pub fn header(&self) -> ObjectHeader {
        ObjectHeader::builtin(&TYPE_TYPE)
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn base(&self) -> Option<&'static TypeObject> {
        self.base
    }
    pub(super) fn user(name: String) -> Self {
        Self {
            name: Cow::Owned(name),
            base: Some(&OBJECT_TYPE),
        }
    }
}
impl fmt::Debug for TypeObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeObject")
            .field("name", &self.name)
            .finish()
    }
}
pub static OBJECT_TYPE: TypeObject = TypeObject {
    name: Cow::Borrowed("object"),
    base: None,
};
pub static TYPE_TYPE: TypeObject = TypeObject {
    name: Cow::Borrowed("type"),
    base: Some(&OBJECT_TYPE),
};
pub static INT_TYPE: TypeObject = TypeObject {
    name: Cow::Borrowed("int"),
    base: Some(&OBJECT_TYPE),
};
pub static BOOL_TYPE: TypeObject = TypeObject {
    name: Cow::Borrowed("bool"),
    base: Some(&OBJECT_TYPE),
};
pub static NONE_TYPE: TypeObject = TypeObject {
    name: Cow::Borrowed("NoneType"),
    base: Some(&OBJECT_TYPE),
};
pub static FUNCTION_TYPE: TypeObject = TypeObject {
    name: Cow::Borrowed("function"),
    base: Some(&OBJECT_TYPE),
};
