use crate::bytecode::heap::ObjRef;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Obj(ObjRef),
}

impl Value {
    pub fn is_falsey(self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }

    pub fn as_obj(self) -> Option<ObjRef> {
        match self {
            Value::Obj(obj) => Some(obj),
            _ => None,
        }
    }
}
