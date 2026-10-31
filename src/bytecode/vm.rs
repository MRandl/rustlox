use std::{collections::HashMap, rc::Rc};

use crate::{
    bytecode::{
        chunk::{Chunk, Op},
        compiler, disassembler,
        heap::{Heap, ObjRef},
        object::{BoundMethod, Class, Closure, Instance, Native, Object, Upvalue},
        value::Value,
    },
    clock,
    error::{LoxError, RuntimeError, RuntimeErrorKind, TraceFrame},
    parsing::stmt::Stmt,
};

const FRAMES_MAX: usize = 1024;

type VmResult<T = ()> = Result<T, RuntimeErrorKind>;

struct CallFrame {
    closure: ObjRef,
    chunk: Rc<Chunk>,
    ip: usize,
    slots: usize,
}

enum Callee {
    Closure,
    Native(Native),
    Class(Option<ObjRef>),
    BoundMethod(BoundMethod),
}

pub struct Vm {
    heap: Heap,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    globals: HashMap<ObjRef, Value>,
    open_upvalues: Vec<ObjRef>,
    init_string: ObjRef,
    disassemble: bool,
}

impl Vm {
    pub fn new(disassemble: bool) -> Vm {
        let mut heap = Heap::default();
        let init_string = heap.intern("init");
        let mut vm = Vm {
            heap,
            stack: Vec::new(),
            frames: Vec::new(),
            globals: HashMap::new(),
            open_upvalues: Vec::new(),
            init_string,
            disassemble,
        };
        vm.define_native("clock", 0, clock_native);
        vm
    }

    pub fn interpret(&mut self, statements: &[Stmt]) -> Result<(), LoxError> {
        let function = compiler::compile(statements, &mut self.heap).map_err(LoxError::Compile)?;
        if self.disassemble {
            print!("{}", disassembler::disassemble(&self.heap, function));
        }
        self.push(Value::Obj(function));
        let closure = self.alloc(Object::Closure(Closure {
            function,
            upvalues: Vec::new(),
        }));
        self.pop();
        self.push(Value::Obj(closure));
        self.call(closure, 0)
            .and_then(|()| self.run())
            .map_err(|kind| {
                let error = self.runtime_error(kind);
                self.reset();
                LoxError::Runtime(error)
            })
    }

    fn define_native(&mut self, name: &str, arity: u8, function: fn(&[Value]) -> Value) {
        let name = self.heap.intern(name);
        self.push(Value::Obj(name));
        let native = self.alloc(Object::Native(Native { arity, function }));
        self.globals.insert(name, Value::Obj(native));
        self.pop();
    }

    fn reset(&mut self) {
        self.stack.clear();
        self.frames.clear();
        self.open_upvalues.clear();
    }

    fn runtime_error(&self, kind: RuntimeErrorKind) -> RuntimeError {
        let trace: Vec<TraceFrame> = self
            .frames
            .iter()
            .rev()
            .map(|frame| TraceFrame {
                function: self.function_name(frame.closure),
                position: frame.chunk.spans[frame.ip - 1].start,
            })
            .collect();
        let frame = self.frame();
        RuntimeError::new(kind, frame.chunk.spans[frame.ip - 1]).with_trace(trace)
    }

    fn function_name(&self, closure: ObjRef) -> Option<String> {
        let function = self.heap.function(self.heap.closure(closure).function);
        function.name.map(|name| self.heap.string(name).to_string())
    }

    fn alloc(&mut self, object: Object) -> ObjRef {
        if self.heap.should_collect() {
            self.collect_garbage();
        }
        self.heap.insert(object)
    }

    fn collect_garbage(&mut self) {
        let roots: Vec<ObjRef> = self
            .stack
            .iter()
            .filter_map(|value| value.as_obj())
            .chain(self.frames.iter().map(|frame| frame.closure))
            .chain(self.open_upvalues.iter().copied())
            .chain(self.globals.keys().copied())
            .chain(self.globals.values().filter_map(|value| value.as_obj()))
            .chain([self.init_string])
            .collect();
        self.heap.collect(roots);
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack
            .pop()
            .expect("compiled code never underflows the stack")
    }

    fn peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - 1 - distance]
    }

    fn frame(&self) -> &CallFrame {
        self.frames
            .last()
            .expect("code only runs inside a call frame")
    }

    fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames
            .last_mut()
            .expect("code only runs inside a call frame")
    }

    fn next_op(&mut self) -> Op {
        let frame = self.frame_mut();
        let op = frame.chunk.code[frame.ip];
        frame.ip += 1;
        op
    }

    fn constant(&self, index: u8) -> Value {
        self.frame().chunk.constants[index as usize]
    }

    fn name_constant(&self, index: u8) -> ObjRef {
        self.constant(index)
            .as_obj()
            .expect("names are interned string constants")
    }

    fn name_string(&self, name: ObjRef) -> String {
        self.heap.string(name).to_string()
    }

    fn instance_of(&self, value: Value) -> Option<ObjRef> {
        value
            .as_obj()
            .filter(|&obj| matches!(self.heap.get(obj), Object::Instance(_)))
    }

    fn class_of(&self, value: Value) -> Option<ObjRef> {
        value
            .as_obj()
            .filter(|&obj| matches!(self.heap.get(obj), Object::Class(_)))
    }

    fn run(&mut self) -> VmResult {
        loop {
            match self.next_op() {
                Op::Constant(index) => self.push(self.constant(index)),
                Op::Nil => self.push(Value::Nil),
                Op::True => self.push(Value::Bool(true)),
                Op::False => self.push(Value::Bool(false)),
                Op::Pop => {
                    self.pop();
                }
                Op::GetLocal(slot) => {
                    let base = self.frame().slots;
                    self.push(self.stack[base + slot as usize]);
                }
                Op::SetLocal(slot) => {
                    let base = self.frame().slots;
                    self.stack[base + slot as usize] = self.peek(0);
                }
                Op::GetGlobal(index) => self.get_global(index)?,
                Op::DefineGlobal(index) => {
                    let name = self.name_constant(index);
                    self.globals.insert(name, self.peek(0));
                    self.pop();
                }
                Op::SetGlobal(index) => self.set_global(index)?,
                Op::GetUpvalue(index) => {
                    let upvalue = self.frame_upvalue(index);
                    let value = match *self.heap.upvalue(upvalue) {
                        Upvalue::Open(slot) => self.stack[slot],
                        Upvalue::Closed(value) => value,
                    };
                    self.push(value);
                }
                Op::SetUpvalue(index) => {
                    let upvalue = self.frame_upvalue(index);
                    let value = self.peek(0);
                    match self.heap.upvalue_mut(upvalue) {
                        Upvalue::Open(slot) => self.stack[*slot] = value,
                        Upvalue::Closed(closed) => *closed = value,
                    }
                }
                Op::GetProperty(index) => self.get_property(index)?,
                Op::SetProperty(index) => self.set_property(index)?,
                Op::GetSuper(index) => {
                    let name = self.name_constant(index);
                    let superclass = self.pop().as_obj().expect("'super' holds a class");
                    self.bind_method(superclass, name)?;
                }
                Op::Equal => {
                    let right = self.pop();
                    let left = self.pop();
                    self.push(Value::Bool(left == right));
                }
                Op::Greater => self.numeric(|a, b| Value::Bool(a > b))?,
                Op::Less => self.numeric(|a, b| Value::Bool(a < b))?,
                Op::Add => self.add()?,
                Op::Subtract => self.numeric(|a, b| Value::Number(a - b))?,
                Op::Multiply => self.numeric(|a, b| Value::Number(a * b))?,
                Op::Divide => self.numeric(|a, b| Value::Number(a / b))?,
                Op::Not => {
                    let value = self.pop();
                    self.push(Value::Bool(value.is_falsey()));
                }
                Op::Negate => match self.peek(0) {
                    Value::Number(n) => {
                        self.pop();
                        self.push(Value::Number(-n));
                    }
                    _ => return Err(RuntimeErrorKind::OperandMustBeNumber),
                },
                Op::Print => {
                    let value = self.pop();
                    println!("{}", self.heap.display(value));
                }
                Op::Jump(offset) => self.frame_mut().ip += offset as usize,
                Op::JumpIfFalse(offset) => {
                    if self.peek(0).is_falsey() {
                        self.frame_mut().ip += offset as usize;
                    }
                }
                Op::Loop(offset) => self.frame_mut().ip -= offset as usize,
                Op::Call(argc) => self.call_value(self.peek(argc as usize), argc)?,
                Op::Invoke(index, argc) => {
                    let name = self.name_constant(index);
                    self.invoke(name, argc)?;
                }
                Op::SuperInvoke(index, argc) => {
                    let name = self.name_constant(index);
                    let superclass = self.pop().as_obj().expect("'super' holds a class");
                    self.invoke_from_class(superclass, name, argc)?;
                }
                Op::Closure(index) => self.make_closure(index),
                Op::CloseUpvalue => {
                    self.close_upvalues(self.stack.len() - 1);
                    self.pop();
                }
                Op::Return => {
                    let result = self.pop();
                    let frame = self.frames.pop().expect("a frame is returning");
                    self.close_upvalues(frame.slots);
                    self.stack.truncate(frame.slots);
                    if self.frames.is_empty() {
                        return Ok(());
                    }
                    self.push(result);
                }
                Op::Class(index) => {
                    let name = self.name_constant(index);
                    let class = self.alloc(Object::Class(Class {
                        name,
                        methods: HashMap::new(),
                    }));
                    self.push(Value::Obj(class));
                }
                Op::Inherit => self.inherit()?,
                Op::Method(index) => {
                    let name = self.name_constant(index);
                    let method = self.peek(0).as_obj().expect("methods are closures");
                    let class = self.peek(1).as_obj().expect("methods belong to a class");
                    self.heap.class_mut(class).methods.insert(name, method);
                    self.pop();
                }
            }
        }
    }

    fn get_global(&mut self, index: u8) -> VmResult {
        let name = self.name_constant(index);
        let value = *self
            .globals
            .get(&name)
            .ok_or_else(|| RuntimeErrorKind::UndefinedVariable(self.name_string(name)))?;
        self.push(value);
        Ok(())
    }

    fn set_global(&mut self, index: u8) -> VmResult {
        let name = self.name_constant(index);
        let value = self.peek(0);
        match self.globals.get_mut(&name) {
            Some(slot) => {
                *slot = value;
                Ok(())
            }
            None => Err(RuntimeErrorKind::UndefinedVariable(self.name_string(name))),
        }
    }

    fn frame_upvalue(&self, index: u8) -> ObjRef {
        self.heap.closure(self.frame().closure).upvalues[index as usize]
    }

    fn numeric(&mut self, operation: fn(f64, f64) -> Value) -> VmResult {
        match (self.peek(1), self.peek(0)) {
            (Value::Number(a), Value::Number(b)) => {
                self.pop();
                self.pop();
                self.push(operation(a, b));
                Ok(())
            }
            _ => Err(RuntimeErrorKind::OperandsMustBeNumbers),
        }
    }

    fn add(&mut self) -> VmResult {
        let concatenated = match (self.peek(1), self.peek(0)) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::Obj(a), Value::Obj(b)) => match (self.heap.get(a), self.heap.get(b)) {
                (Object::String(a), Object::String(b)) => {
                    let joined = format!("{a}{b}");
                    if self.heap.should_collect() {
                        self.collect_garbage();
                    }
                    Value::Obj(self.heap.intern(&joined))
                }
                _ => return Err(RuntimeErrorKind::OperandsMustBeNumbersOrStrings),
            },
            _ => return Err(RuntimeErrorKind::OperandsMustBeNumbersOrStrings),
        };
        self.pop();
        self.pop();
        self.push(concatenated);
        Ok(())
    }

    fn call_value(&mut self, callee: Value, argc: u8) -> VmResult {
        let obj = callee.as_obj().ok_or(RuntimeErrorKind::NotCallable)?;
        let kind = match self.heap.get(obj) {
            Object::Closure(_) => Callee::Closure,
            Object::Native(native) => Callee::Native(*native),
            Object::Class(class) => Callee::Class(class.methods.get(&self.init_string).copied()),
            Object::BoundMethod(bound) => Callee::BoundMethod(*bound),
            _ => return Err(RuntimeErrorKind::NotCallable),
        };
        let receiver_slot = self.stack.len() - argc as usize - 1;
        match kind {
            Callee::Closure => self.call(obj, argc),
            Callee::Native(native) => {
                check_arity(native.arity, argc)?;
                let result = (native.function)(&self.stack[receiver_slot + 1..]);
                self.stack.truncate(receiver_slot);
                self.push(result);
                Ok(())
            }
            Callee::Class(initializer) => {
                let instance = self.alloc(Object::Instance(Instance {
                    class: obj,
                    fields: HashMap::new(),
                }));
                self.stack[receiver_slot] = Value::Obj(instance);
                match initializer {
                    Some(initializer) => self.call(initializer, argc),
                    None => check_arity(0, argc),
                }
            }
            Callee::BoundMethod(bound) => {
                self.stack[receiver_slot] = bound.receiver;
                self.call(bound.method, argc)
            }
        }
    }

    fn call(&mut self, closure: ObjRef, argc: u8) -> VmResult {
        let function = self.heap.function(self.heap.closure(closure).function);
        check_arity(function.arity, argc)?;
        if self.frames.len() == FRAMES_MAX {
            return Err(RuntimeErrorKind::StackOverflow);
        }
        let chunk = function.chunk.clone();
        self.frames.push(CallFrame {
            closure,
            chunk,
            ip: 0,
            slots: self.stack.len() - argc as usize - 1,
        });
        Ok(())
    }

    fn invoke(&mut self, name: ObjRef, argc: u8) -> VmResult {
        let receiver = self.peek(argc as usize);
        let instance = self
            .instance_of(receiver)
            .ok_or(RuntimeErrorKind::OnlyInstancesHaveMethods)?;
        let instance = self.heap.instance(instance);
        if let Some(&field) = instance.fields.get(&name) {
            let receiver_slot = self.stack.len() - argc as usize - 1;
            self.stack[receiver_slot] = field;
            return self.call_value(field, argc);
        }
        self.invoke_from_class(instance.class, name, argc)
    }

    fn invoke_from_class(&mut self, class: ObjRef, name: ObjRef, argc: u8) -> VmResult {
        let method = self.find_method(class, name)?;
        self.call(method, argc)
    }

    fn find_method(&self, class: ObjRef, name: ObjRef) -> VmResult<ObjRef> {
        self.heap
            .class(class)
            .methods
            .get(&name)
            .copied()
            .ok_or_else(|| RuntimeErrorKind::UndefinedProperty(self.name_string(name)))
    }

    fn bind_method(&mut self, class: ObjRef, name: ObjRef) -> VmResult {
        let method = self.find_method(class, name)?;
        let bound = self.alloc(Object::BoundMethod(BoundMethod {
            receiver: self.peek(0),
            method,
        }));
        self.pop();
        self.push(Value::Obj(bound));
        Ok(())
    }

    fn get_property(&mut self, index: u8) -> VmResult {
        let name = self.name_constant(index);
        let instance = self
            .instance_of(self.peek(0))
            .ok_or(RuntimeErrorKind::OnlyInstancesHaveProperties)?;
        let instance = self.heap.instance(instance);
        if let Some(&value) = instance.fields.get(&name) {
            self.pop();
            self.push(value);
            return Ok(());
        }
        self.bind_method(instance.class, name)
    }

    fn set_property(&mut self, index: u8) -> VmResult {
        let name = self.name_constant(index);
        let instance = self
            .instance_of(self.peek(1))
            .ok_or(RuntimeErrorKind::OnlyInstancesHaveFields)?;
        let value = self.peek(0);
        self.heap.instance_mut(instance).fields.insert(name, value);
        self.pop();
        self.pop();
        self.push(value);
        Ok(())
    }

    fn inherit(&mut self) -> VmResult {
        let superclass = self
            .class_of(self.peek(1))
            .ok_or(RuntimeErrorKind::SuperclassMustBeClass)?;
        let subclass = self.peek(0).as_obj().expect("classes inherit into a class");
        let inherited = self.heap.class(superclass).methods.clone();
        self.heap.class_mut(subclass).methods.extend(inherited);
        self.pop();
        Ok(())
    }

    fn make_closure(&mut self, index: u8) {
        let function = self
            .constant(index)
            .as_obj()
            .expect("closures wrap function constants");
        let closure = self.alloc(Object::Closure(Closure {
            function,
            upvalues: Vec::new(),
        }));
        self.push(Value::Obj(closure));
        let descriptors = self.heap.function(function).upvalues.clone();
        for descriptor in descriptors {
            let upvalue = if descriptor.is_local {
                self.capture_upvalue(self.frame().slots + descriptor.index as usize)
            } else {
                self.frame_upvalue(descriptor.index)
            };
            self.heap.closure_mut(closure).upvalues.push(upvalue);
        }
    }

    fn capture_upvalue(&mut self, slot: usize) -> ObjRef {
        let existing = self.open_upvalues.iter().copied().find(
            |&upvalue| matches!(self.heap.upvalue(upvalue), Upvalue::Open(open) if *open == slot),
        );
        existing.unwrap_or_else(|| {
            let upvalue = self.alloc(Object::Upvalue(Upvalue::Open(slot)));
            self.open_upvalues.push(upvalue);
            upvalue
        })
    }

    fn close_upvalues(&mut self, from: usize) {
        let stack = &self.stack;
        let heap = &mut self.heap;
        self.open_upvalues.retain(|&upvalue| {
            let upvalue = heap.upvalue_mut(upvalue);
            match *upvalue {
                Upvalue::Open(slot) if slot >= from => {
                    *upvalue = Upvalue::Closed(stack[slot]);
                    false
                }
                _ => true,
            }
        });
    }
}

fn check_arity(expected: u8, got: u8) -> VmResult {
    if expected == got {
        Ok(())
    } else {
        Err(RuntimeErrorKind::ArityMismatch {
            expected: expected as usize,
            got: got as usize,
        })
    }
}

fn clock_native(_args: &[Value]) -> Value {
    Value::Number(clock::seconds_since_epoch())
}
