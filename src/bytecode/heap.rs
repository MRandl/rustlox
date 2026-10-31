use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    rc::Rc,
};

use crate::bytecode::{
    object::{Class, Closure, Function, Instance, Object, Upvalue},
    value::Value,
};

const INITIAL_GC_THRESHOLD: usize = 1024 * 1024;
const GC_GROWTH_FACTOR: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjRef(usize);

struct Slot {
    object: Object,
    size: usize,
    marked: bool,
}

pub struct Heap {
    slots: Vec<Option<Slot>>,
    free: Vec<usize>,
    strings: HashMap<Rc<str>, ObjRef>,
    bytes_allocated: usize,
    next_gc: usize,
}

impl Default for Heap {
    fn default() -> Heap {
        Heap {
            slots: Vec::new(),
            free: Vec::new(),
            strings: HashMap::new(),
            bytes_allocated: 0,
            next_gc: INITIAL_GC_THRESHOLD,
        }
    }
}

macro_rules! typed_accessor {
    ($accessor:ident: &$variant:ident) => {
        pub fn $accessor(&self, obj: ObjRef) -> &$variant {
            match self.get(obj) {
                Object::$variant(inner) => inner,
                _ => unreachable!(concat!("object is not a ", stringify!($variant))),
            }
        }
    };
    ($accessor:ident: &mut $variant:ident) => {
        pub fn $accessor(&mut self, obj: ObjRef) -> &mut $variant {
            match self.get_mut(obj) {
                Object::$variant(inner) => inner,
                _ => unreachable!(concat!("object is not a ", stringify!($variant))),
            }
        }
    };
}

impl Heap {
    typed_accessor!(function: &Function);
    typed_accessor!(closure: &Closure);
    typed_accessor!(closure_mut: &mut Closure);
    typed_accessor!(upvalue: &Upvalue);
    typed_accessor!(upvalue_mut: &mut Upvalue);
    typed_accessor!(class: &Class);
    typed_accessor!(class_mut: &mut Class);
    typed_accessor!(instance: &Instance);
    typed_accessor!(instance_mut: &mut Instance);

    pub fn insert(&mut self, object: Object) -> ObjRef {
        let size = object.size();
        self.bytes_allocated += size;
        let slot = Some(Slot {
            object,
            size,
            marked: false,
        });
        match self.free.pop() {
            Some(index) => {
                self.slots[index] = slot;
                ObjRef(index)
            }
            None => {
                self.slots.push(slot);
                ObjRef(self.slots.len() - 1)
            }
        }
    }

    pub fn intern(&mut self, string: &str) -> ObjRef {
        if let Some(&interned) = self.strings.get(string) {
            return interned;
        }
        let shared: Rc<str> = Rc::from(string);
        let interned = self.insert(Object::String(shared.clone()));
        self.strings.insert(shared, interned);
        interned
    }

    pub fn get(&self, obj: ObjRef) -> &Object {
        &self.slots[obj.0]
            .as_ref()
            .expect("live references never point to freed slots")
            .object
    }

    pub fn get_mut(&mut self, obj: ObjRef) -> &mut Object {
        &mut self.slots[obj.0]
            .as_mut()
            .expect("live references never point to freed slots")
            .object
    }

    pub fn string(&self, obj: ObjRef) -> &str {
        match self.get(obj) {
            Object::String(string) => string,
            _ => unreachable!("object is not a String"),
        }
    }

    pub fn display(&self, value: Value) -> ValueDisplay<'_> {
        ValueDisplay { heap: self, value }
    }

    pub fn should_collect(&self) -> bool {
        cfg!(feature = "stress-gc") || self.bytes_allocated > self.next_gc
    }

    pub fn collect(&mut self, roots: impl IntoIterator<Item = ObjRef>) {
        let mut gray = Vec::new();
        for root in roots {
            self.mark(root, &mut gray);
        }
        let mut references = Vec::new();
        while let Some(obj) = gray.pop() {
            self.get(obj).trace(&mut references);
            for reference in references.drain(..) {
                self.mark(reference, &mut gray);
            }
        }
        self.sweep();
        self.next_gc = (self.bytes_allocated * GC_GROWTH_FACTOR).max(INITIAL_GC_THRESHOLD);
    }

    fn mark(&mut self, obj: ObjRef, gray: &mut Vec<ObjRef>) {
        let slot = self.slots[obj.0]
            .as_mut()
            .expect("live references never point to freed slots");
        if !slot.marked {
            slot.marked = true;
            gray.push(obj);
        }
    }

    fn sweep(&mut self) {
        for (index, entry) in self.slots.iter_mut().enumerate() {
            let Some(slot) = entry else {
                continue;
            };
            if slot.marked {
                slot.marked = false;
                continue;
            }
            if let Object::String(string) = &slot.object {
                self.strings.remove(string);
            }
            self.bytes_allocated -= slot.size;
            *entry = None;
            self.free.push(index);
        }
    }

    fn write_object(&self, f: &mut Formatter, obj: ObjRef) -> fmt::Result {
        match self.get(obj) {
            Object::String(string) => write!(f, "{string}"),
            Object::Function(function) => match function.name {
                Some(name) => write!(f, "<fn {}>", self.string(name)),
                None => write!(f, "<script>"),
            },
            Object::Native(_) => write!(f, "<native fn>"),
            Object::Closure(closure) => self.write_object(f, closure.function),
            Object::Upvalue(_) => write!(f, "upvalue"),
            Object::Class(class) => write!(f, "{}", self.string(class.name)),
            Object::Instance(instance) => {
                let class = self.class(instance.class);
                write!(f, "{} instance", self.string(class.name))
            }
            Object::BoundMethod(bound) => self.write_object(f, bound.method),
        }
    }
}

pub struct ValueDisplay<'h> {
    heap: &'h Heap,
    value: Value,
}

impl Display for ValueDisplay<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.value {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::Obj(obj) => self.heap.write_object(f, obj),
        }
    }
}
