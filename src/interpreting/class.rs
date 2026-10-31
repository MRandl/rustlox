use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    error::RuntimeError,
    interpreting::{
        callable::{Callable, LoxFunction},
        interpret::Interpreter,
        value::LoxValue,
    },
};

const INITIALIZER: &str = "init";

pub struct LoxClass {
    name: String,
    superclass: Option<Rc<LoxClass>>,
    methods: HashMap<String, Rc<LoxFunction>>,
}

impl LoxClass {
    pub fn new(
        name: String,
        superclass: Option<Rc<LoxClass>>,
        methods: HashMap<String, Rc<LoxFunction>>,
    ) -> LoxClass {
        LoxClass {
            name,
            superclass,
            methods,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn find_method(&self, name: &str) -> Option<Rc<LoxFunction>> {
        self.methods
            .get(name)
            .cloned()
            .or_else(|| self.superclass.as_ref()?.find_method(name))
    }

    pub fn is_initializer(method_name: &str) -> bool {
        method_name == INITIALIZER
    }
}

impl Callable for LoxClass {
    fn arity(&self) -> usize {
        self.find_method(INITIALIZER)
            .map_or(0, |initializer| initializer.arity())
    }

    fn call(
        self: Rc<Self>,
        interpreter: &mut Interpreter,
        args: Vec<LoxValue>,
    ) -> Result<LoxValue, RuntimeError> {
        let instance = Rc::new(LoxInstance::new(self.clone()));
        if let Some(initializer) = self.find_method(INITIALIZER) {
            Rc::new(initializer.bind(instance.clone())).call(interpreter, args)?;
        }
        Ok(LoxValue::Instance(instance))
    }
}

pub struct LoxInstance {
    class: Rc<LoxClass>,
    fields: RefCell<HashMap<String, LoxValue>>,
}

impl LoxInstance {
    fn new(class: Rc<LoxClass>) -> LoxInstance {
        LoxInstance {
            class,
            fields: RefCell::new(HashMap::new()),
        }
    }

    pub fn class_name(&self) -> &str {
        self.class.name()
    }

    pub fn get(self: &Rc<Self>, name: &str) -> Option<LoxValue> {
        let field = self.fields.borrow().get(name).cloned();
        field.or_else(|| {
            let method = self.class.find_method(name)?;
            Some(LoxValue::Function(Rc::new(method.bind(self.clone()))))
        })
    }

    pub fn set(&self, name: &str, value: LoxValue) {
        self.fields.borrow_mut().insert(name.to_string(), value);
    }
}
