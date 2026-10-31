use std::{
    fmt::{self, Display, Formatter},
    rc::Rc,
};

use crate::interpreting::{
    callable::{Callable, LoxFunction, NativeFunction},
    class::{LoxClass, LoxInstance},
};

#[derive(Clone)]
pub enum LoxValue {
    Nil,
    Bool(bool),
    Number(f64),
    Str(Rc<str>),
    Function(Rc<LoxFunction>),
    Native(Rc<NativeFunction>),
    Class(Rc<LoxClass>),
    Instance(Rc<LoxInstance>),
}

impl LoxValue {
    pub fn is_truthy(&self) -> bool {
        !matches!(self, LoxValue::Nil | LoxValue::Bool(false))
    }

    pub fn as_callable(&self) -> Option<Rc<dyn Callable>> {
        match self {
            LoxValue::Function(function) => Some(function.clone()),
            LoxValue::Native(native) => Some(native.clone()),
            LoxValue::Class(class) => Some(class.clone()),
            _ => None,
        }
    }
}

impl PartialEq for LoxValue {
    fn eq(&self, other: &LoxValue) -> bool {
        match (self, other) {
            (LoxValue::Nil, LoxValue::Nil) => true,
            (LoxValue::Bool(a), LoxValue::Bool(b)) => a == b,
            (LoxValue::Number(a), LoxValue::Number(b)) => a == b,
            (LoxValue::Str(a), LoxValue::Str(b)) => a == b,
            (LoxValue::Function(a), LoxValue::Function(b)) => Rc::ptr_eq(a, b),
            (LoxValue::Native(a), LoxValue::Native(b)) => Rc::ptr_eq(a, b),
            (LoxValue::Class(a), LoxValue::Class(b)) => Rc::ptr_eq(a, b),
            (LoxValue::Instance(a), LoxValue::Instance(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Display for LoxValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LoxValue::Nil => write!(f, "nil"),
            LoxValue::Bool(b) => write!(f, "{b}"),
            LoxValue::Number(n) => write!(f, "{n}"),
            LoxValue::Str(s) => write!(f, "{s}"),
            LoxValue::Function(function) => write!(f, "<fn {}>", function.name()),
            LoxValue::Native(_) => write!(f, "<native fn>"),
            LoxValue::Class(class) => write!(f, "{}", class.name()),
            LoxValue::Instance(instance) => write!(f, "{} instance", instance.class_name()),
        }
    }
}
