use std::rc::Rc;

use crate::{
    clock,
    error::RuntimeError,
    interpreting::{
        class::LoxInstance,
        environment::Environment,
        interpret::{Interpreter, Unwind},
        value::LoxValue,
    },
    parsing::stmt::FunctionDecl,
};

pub trait Callable {
    fn arity(&self) -> usize;

    fn call(
        self: Rc<Self>,
        interpreter: &mut Interpreter,
        args: Vec<LoxValue>,
    ) -> Result<LoxValue, RuntimeError>;
}

pub struct LoxFunction {
    declaration: Rc<FunctionDecl>,
    closure: Environment,
    is_initializer: bool,
}

impl LoxFunction {
    pub fn new(
        declaration: Rc<FunctionDecl>,
        closure: Environment,
        is_initializer: bool,
    ) -> LoxFunction {
        LoxFunction {
            declaration,
            closure,
            is_initializer,
        }
    }

    pub fn name(&self) -> &str {
        &self.declaration.name.name
    }

    pub fn bind(&self, instance: Rc<LoxInstance>) -> LoxFunction {
        let environment = self.closure.child();
        environment.define("this", LoxValue::Instance(instance));
        LoxFunction::new(self.declaration.clone(), environment, self.is_initializer)
    }

    fn bound_instance(&self) -> LoxValue {
        self.closure
            .get_at(0, "this")
            .expect("initializers are only called once bound to an instance")
    }
}

impl Callable for LoxFunction {
    fn arity(&self) -> usize {
        self.declaration.params.len()
    }

    fn call(
        self: Rc<Self>,
        interpreter: &mut Interpreter,
        args: Vec<LoxValue>,
    ) -> Result<LoxValue, RuntimeError> {
        let environment = self.closure.child();
        for (param, arg) in self.declaration.params.iter().zip(args) {
            environment.define(&param.name, arg);
        }
        let returned = match interpreter.execute_block(&self.declaration.body, environment) {
            Ok(()) => LoxValue::Nil,
            Err(Unwind::Return(value)) => value,
            Err(Unwind::Error(error)) => return Err(error),
        };
        if self.is_initializer {
            Ok(self.bound_instance())
        } else {
            Ok(returned)
        }
    }
}

pub struct NativeFunction {
    pub name: &'static str,
    arity: usize,
    function: fn(&[LoxValue]) -> LoxValue,
}

impl NativeFunction {
    pub fn all() -> Vec<NativeFunction> {
        vec![NativeFunction {
            name: "clock",
            arity: 0,
            function: clock,
        }]
    }
}

impl Callable for NativeFunction {
    fn arity(&self) -> usize {
        self.arity
    }

    fn call(
        self: Rc<Self>,
        _interpreter: &mut Interpreter,
        args: Vec<LoxValue>,
    ) -> Result<LoxValue, RuntimeError> {
        Ok((self.function)(&args))
    }
}

fn clock(_args: &[LoxValue]) -> LoxValue {
    LoxValue::Number(clock::seconds_since_epoch())
}
