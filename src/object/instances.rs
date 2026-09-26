use super::*;
use crate::bytecode::ClassCode;
use alloc::collections::BTreeMap;
use core::cell::RefCell;

pub(crate) struct ClassObject {
    pub(crate) code: Rc<ClassCode>,
    descriptor: Rc<TypeObject>,
    defaults: BTreeMap<String, Object>,
}
pub(super) struct InstanceObject {
    pub(super) class: Rc<ClassObject>,
    fields: RefCell<BTreeMap<String, Object>>,
}
pub(super) struct BoundMethod {
    pub(super) function: Rc<FunctionCode>,
    receiver: Object,
}
pub(crate) enum Callable {
    Builtin(crate::builtins::Builtin),
    Function(Rc<FunctionCode>),
    Bound(Rc<FunctionCode>, Object),
    Class(Rc<ClassObject>),
}

impl Object {
    pub(crate) fn function(code: Rc<FunctionCode>) -> Self {
        Self::new(&FUNCTION_TYPE, Payload::Function(code))
    }
    pub(crate) fn class(code: Rc<ClassCode>, defaults: BTreeMap<String, Object>) -> Self {
        let descriptor = Rc::new(TypeObject::user(code.name.clone()));
        Self::new(
            &TYPE_TYPE,
            Payload::Class(Rc::new(ClassObject {
                code,
                descriptor,
                defaults,
            })),
        )
    }
    pub(crate) fn callable(&self) -> Result<Callable, String> {
        match &self.0.payload {
            Payload::Builtin(builtin) => Ok(Callable::Builtin(*builtin)),
            Payload::Function(function) => Ok(Callable::Function(function.clone())),
            Payload::BoundMethod(method) => Ok(Callable::Bound(
                method.function.clone(),
                method.receiver.clone(),
            )),
            Payload::Class(class) => Ok(Callable::Class(class.clone())),
            _ => Err(format!("TypeError: '{}' is not callable", self.type_name())),
        }
    }
    pub(crate) fn instantiate(class: Rc<ClassObject>) -> Self {
        let mut copies = BTreeMap::new();
        let fields = class
            .defaults
            .iter()
            .map(|(name, value)| (name.clone(), value.copy_default(&mut copies)))
            .collect();
        Self::instance(class, fields)
    }
    fn instance(class: Rc<ClassObject>, fields: BTreeMap<String, Object>) -> Self {
        Self(Rc::new(ObjectData {
            header: ObjectHeader::user(class.descriptor.clone()),
            payload: Payload::Instance(InstanceObject {
                class,
                fields: RefCell::new(fields),
            }),
        }))
    }
    // Field types can reference only earlier classes, so this graph is acyclic.
    // Preserve aliases inside a copied default graph without sharing mutable state
    // between separate constructor calls.
    fn copy_default(&self, copies: &mut BTreeMap<usize, Object>) -> Self {
        let id = Rc::as_ptr(&self.0) as usize;
        if let Some(copy) = copies.get(&id) {
            return copy.clone();
        }
        let copy = match &self.0.payload {
            Payload::Instance(instance) => {
                let fields = instance
                    .fields
                    .borrow()
                    .iter()
                    .map(|(name, value)| (name.clone(), value.copy_default(copies)))
                    .collect();
                Self::instance(instance.class.clone(), fields)
            }
            _ => self.clone(),
        };
        copies.insert(id, copy.clone());
        copy
    }
    pub(crate) fn get_attribute(&self, name: &str) -> Result<Self, String> {
        let Payload::Instance(instance) = &self.0.payload else {
            return Err(format!(
                "AttributeError: '{}' has no attribute '{}'",
                self.type_name(),
                name
            ));
        };
        if let Some(value) = instance.fields.borrow().get(name) {
            return Ok(value.clone());
        }
        if let Some(function) = instance.class.code.methods.get(name) {
            return Ok(Self::new(
                &FUNCTION_TYPE,
                Payload::BoundMethod(BoundMethod {
                    function: function.clone(),
                    receiver: self.clone(),
                }),
            ));
        }
        Err(format!("AttributeError: unknown member '{}'", name))
    }
    /// Replace a declared field, returning the old value for the VM undo journal.
    pub(crate) fn replace_field(&self, name: &str, value: Object) -> Result<Object, String> {
        let Payload::Instance(instance) = &self.0.payload else {
            return Err("AttributeError: expected instance".to_string());
        };
        let mut fields = instance.fields.borrow_mut();
        let field = fields
            .get_mut(name)
            .ok_or_else(|| format!("AttributeError: unknown or read-only field '{}'", name))?;
        if !core::ptr::eq(field.type_object(), value.type_object()) {
            return Err(format!(
                "TypeError: incompatible value for field '{}'",
                name
            ));
        }
        Ok(core::mem::replace(field, value))
    }
}
