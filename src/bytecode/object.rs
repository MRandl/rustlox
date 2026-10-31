use std::{collections::HashMap, mem, rc::Rc};

use crate::bytecode::{
    chunk::{Chunk, Op},
    heap::ObjRef,
    value::Value,
};

pub enum Object {
    String(Rc<str>),
    Function(Function),
    Native(Native),
    Closure(Closure),
    Upvalue(Upvalue),
    Class(Class),
    Instance(Instance),
    BoundMethod(BoundMethod),
}

pub struct Function {
    pub name: Option<ObjRef>,
    pub arity: u8,
    pub chunk: Rc<Chunk>,
    pub upvalues: Vec<UpvalueDescriptor>,
}

#[derive(Clone, Copy, Debug)]
pub struct UpvalueDescriptor {
    pub is_local: bool,
    pub index: u8,
}

#[derive(Clone, Copy)]
pub struct Native {
    pub arity: u8,
    pub function: fn(&[Value]) -> Value,
}

pub struct Closure {
    pub function: ObjRef,
    pub upvalues: Vec<ObjRef>,
}

pub enum Upvalue {
    Open(usize),
    Closed(Value),
}

pub struct Class {
    pub name: ObjRef,
    pub methods: HashMap<ObjRef, ObjRef>,
}

pub struct Instance {
    pub class: ObjRef,
    pub fields: HashMap<ObjRef, Value>,
}

#[derive(Clone, Copy)]
pub struct BoundMethod {
    pub receiver: Value,
    pub method: ObjRef,
}

impl Object {
    pub fn trace(&self, references: &mut Vec<ObjRef>) {
        match self {
            Object::String(_) | Object::Native(_) => {}
            Object::Function(function) => {
                references.extend(function.name);
                references.extend(function.chunk.constants.iter().filter_map(|v| v.as_obj()));
            }
            Object::Closure(closure) => {
                references.push(closure.function);
                references.extend(&closure.upvalues);
            }
            Object::Upvalue(Upvalue::Open(_)) => {}
            Object::Upvalue(Upvalue::Closed(value)) => references.extend(value.as_obj()),
            Object::Class(class) => {
                references.push(class.name);
                references.extend(class.methods.iter().flat_map(|(&k, &v)| [k, v]));
            }
            Object::Instance(instance) => {
                references.push(instance.class);
                references.extend(instance.fields.keys());
                references.extend(instance.fields.values().filter_map(|v| v.as_obj()));
            }
            Object::BoundMethod(bound) => {
                references.push(bound.method);
                references.extend(bound.receiver.as_obj());
            }
        }
    }

    pub fn size(&self) -> usize {
        let entry = mem::size_of::<(ObjRef, Value)>();
        mem::size_of::<Object>()
            + match self {
                Object::String(string) => string.len(),
                Object::Function(function) => {
                    function.chunk.code.len() * mem::size_of::<Op>()
                        + function.chunk.constants.len() * mem::size_of::<Value>()
                }
                Object::Closure(closure) => closure.upvalues.len() * mem::size_of::<ObjRef>(),
                Object::Class(class) => class.methods.len() * entry,
                Object::Instance(instance) => instance.fields.len() * entry,
                Object::Native(_) | Object::Upvalue(_) | Object::BoundMethod(_) => 0,
            }
    }
}
