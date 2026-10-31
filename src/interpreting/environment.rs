use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::interpreting::value::LoxValue;

#[derive(Clone, Default)]
pub struct Environment(Rc<RefCell<Frame>>);

#[derive(Default)]
struct Frame {
    values: HashMap<String, LoxValue>,
    enclosing: Option<Environment>,
}

impl Environment {
    pub fn child(&self) -> Environment {
        Environment(Rc::new(RefCell::new(Frame {
            values: HashMap::new(),
            enclosing: Some(self.clone()),
        })))
    }

    pub fn define(&self, name: &str, value: LoxValue) {
        self.0.borrow_mut().values.insert(name.to_string(), value);
    }

    pub fn get_at(&self, depth: usize, name: &str) -> Option<LoxValue> {
        self.ancestor(depth).0.borrow().values.get(name).cloned()
    }

    pub fn assign_at(&self, depth: usize, name: &str, value: LoxValue) -> bool {
        let ancestor = self.ancestor(depth);
        let mut frame = ancestor.0.borrow_mut();
        match frame.values.get_mut(name) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }

    fn ancestor(&self, depth: usize) -> Environment {
        (0..depth).fold(self.clone(), |environment, _| {
            environment
                .0
                .borrow()
                .enclosing
                .clone()
                .expect("resolved depths never exceed the scope chain")
        })
    }
}
