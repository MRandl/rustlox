use std::{collections::HashMap, mem, rc::Rc};

use crate::{
    error::{RuntimeError, RuntimeErrorKind},
    interpreting::{
        callable::{LoxFunction, NativeFunction},
        class::{LoxClass, LoxInstance},
        environment::Environment,
        operators,
        value::LoxValue,
    },
    parsing::{
        expr::{Expr, Identifier, LogicalOp, PositionedExpr, Variable},
        stmt::{ClassDecl, Stmt},
    },
    resolving::resolver::Locals,
};

pub enum Unwind {
    Error(RuntimeError),
    Return(LoxValue),
}

impl From<RuntimeError> for Unwind {
    fn from(error: RuntimeError) -> Unwind {
        Unwind::Error(error)
    }
}

type EvalResult = Result<LoxValue, RuntimeError>;
type ExecResult = Result<(), Unwind>;

pub struct Interpreter {
    globals: Environment,
    environment: Environment,
    locals: Locals,
}

impl Default for Interpreter {
    fn default() -> Interpreter {
        let globals = Environment::default();
        for native in NativeFunction::all() {
            globals.define(native.name, LoxValue::Native(Rc::new(native)));
        }
        Interpreter {
            environment: globals.clone(),
            globals,
            locals: HashMap::new(),
        }
    }
}

impl Interpreter {
    pub fn interpret(&mut self, statements: &[Stmt], locals: Locals) -> Result<(), RuntimeError> {
        self.locals.extend(locals);
        for statement in statements {
            match self.execute(statement) {
                Ok(()) => {}
                Err(Unwind::Error(error)) => return Err(error),
                Err(Unwind::Return(_)) => unreachable!("the resolver rejects top-level returns"),
            }
        }
        Ok(())
    }

    pub fn execute_block(&mut self, statements: &[Stmt], environment: Environment) -> ExecResult {
        let enclosing = mem::replace(&mut self.environment, environment);
        let result = statements
            .iter()
            .try_for_each(|statement| self.execute(statement));
        self.environment = enclosing;
        result
    }

    fn execute(&mut self, stmt: &Stmt) -> ExecResult {
        match stmt {
            Stmt::Expression(expr) => {
                self.evaluate(expr)?;
            }
            Stmt::Print(expr) => println!("{}", self.evaluate(expr)?),
            Stmt::Var { name, initializer } => {
                let value = match initializer {
                    Some(initializer) => self.evaluate(initializer)?,
                    None => LoxValue::Nil,
                };
                self.environment.define(&name.name, value);
            }
            Stmt::Block(statements) => {
                self.execute_block(statements, self.environment.child())?;
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.evaluate(condition)?.is_truthy() {
                    self.execute(then_branch)?;
                } else if let Some(else_branch) = else_branch {
                    self.execute(else_branch)?;
                }
            }
            Stmt::While { condition, body } => {
                while self.evaluate(condition)?.is_truthy() {
                    self.execute(body)?;
                }
            }
            Stmt::Function(declaration) => {
                let function =
                    LoxFunction::new(declaration.clone(), self.environment.clone(), false);
                self.environment.define(
                    &declaration.name.name,
                    LoxValue::Function(Rc::new(function)),
                );
            }
            Stmt::Return { value, .. } => {
                let value = match value {
                    Some(value) => self.evaluate(value)?,
                    None => LoxValue::Nil,
                };
                return Err(Unwind::Return(value));
            }
            Stmt::Class(declaration) => self.declare_class(declaration)?,
        }
        Ok(())
    }

    fn declare_class(&mut self, declaration: &ClassDecl) -> Result<(), RuntimeError> {
        let superclass = match &declaration.superclass {
            Some(variable) => match self.look_up(variable)? {
                LoxValue::Class(class) => Some(class),
                _ => {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::SuperclassMustBeClass,
                        variable.identifier.span,
                    ))
                }
            },
            None => None,
        };

        let name = &declaration.name.name;
        self.environment.define(name, LoxValue::Nil);

        let method_closure = match &superclass {
            Some(superclass) => {
                let environment = self.environment.child();
                environment.define("super", LoxValue::Class(superclass.clone()));
                environment
            }
            None => self.environment.clone(),
        };
        let methods = declaration
            .methods
            .iter()
            .map(|method| {
                let method_name = method.name.name.clone();
                let is_initializer = LoxClass::is_initializer(&method_name);
                let function =
                    LoxFunction::new(method.clone(), method_closure.clone(), is_initializer);
                (method_name, Rc::new(function))
            })
            .collect();

        let class = LoxClass::new(name.clone(), superclass, methods);
        self.environment
            .assign_at(0, name, LoxValue::Class(Rc::new(class)));
        Ok(())
    }

    fn evaluate(&mut self, pexpr: &PositionedExpr) -> EvalResult {
        let fail = |kind| RuntimeError::new(kind, pexpr.span);
        match &pexpr.expr {
            Expr::Num(n) => Ok(LoxValue::Number(*n)),
            Expr::Str(s) => Ok(LoxValue::Str(s.as_str().into())),
            Expr::T => Ok(LoxValue::Bool(true)),
            Expr::F => Ok(LoxValue::Bool(false)),
            Expr::Nil => Ok(LoxValue::Nil),
            Expr::Grouping(inner) => self.evaluate(inner),
            Expr::Unary { op, e1 } => {
                let operand = self.evaluate(e1)?;
                operators::unary(*op, operand).map_err(fail)
            }
            Expr::Binary { e1, op, e2 } => {
                let left = self.evaluate(e1)?;
                let right = self.evaluate(e2)?;
                operators::binary(*op, left, right).map_err(fail)
            }
            Expr::Logical { e1, op, e2 } => {
                let left = self.evaluate(e1)?;
                let short_circuits = match op {
                    LogicalOp::Or => left.is_truthy(),
                    LogicalOp::And => !left.is_truthy(),
                };
                if short_circuits {
                    Ok(left)
                } else {
                    self.evaluate(e2)
                }
            }
            Expr::Variable(variable) | Expr::This(variable) => self.look_up(variable),
            Expr::Assign { target, value } => {
                let value = self.evaluate(value)?;
                self.assign(target, value.clone())?;
                Ok(value)
            }
            Expr::Call { callee, args } => {
                let callee = self.evaluate(callee)?;
                let args = args
                    .iter()
                    .map(|arg| self.evaluate(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let callable = callee
                    .as_callable()
                    .ok_or_else(|| fail(RuntimeErrorKind::NotCallable))?;
                if args.len() != callable.arity() {
                    return Err(fail(RuntimeErrorKind::ArityMismatch {
                        expected: callable.arity(),
                        got: args.len(),
                    }));
                }
                callable.call(self, args)
            }
            Expr::Get { object, name } => match self.evaluate(object)? {
                LoxValue::Instance(instance) => Self::property(&instance, name),
                _ => Err(fail(RuntimeErrorKind::OnlyInstancesHaveProperties)),
            },
            Expr::Set {
                object,
                name,
                value,
            } => {
                let LoxValue::Instance(instance) = self.evaluate(object)? else {
                    return Err(fail(RuntimeErrorKind::OnlyInstancesHaveFields));
                };
                let value = self.evaluate(value)?;
                instance.set(&name.name, value.clone());
                Ok(value)
            }
            Expr::Super { keyword, method } => self.super_method(keyword, method),
        }
    }

    fn property(instance: &Rc<LoxInstance>, name: &Identifier) -> EvalResult {
        instance.get(&name.name).ok_or_else(|| {
            RuntimeError::new(
                RuntimeErrorKind::UndefinedProperty(name.name.clone()),
                name.span,
            )
        })
    }

    fn super_method(&self, keyword: &Variable, method: &Identifier) -> EvalResult {
        let depth = *self
            .locals
            .get(&keyword.id)
            .expect("the resolver binds every 'super' expression");
        let Some(LoxValue::Class(superclass)) = self.environment.get_at(depth, "super") else {
            unreachable!("'super' is always bound to a class")
        };
        let Some(LoxValue::Instance(instance)) = self.environment.get_at(depth - 1, "this") else {
            unreachable!("'this' is always bound one scope inside 'super'")
        };
        let method = superclass.find_method(&method.name).ok_or_else(|| {
            RuntimeError::new(
                RuntimeErrorKind::UndefinedProperty(method.name.clone()),
                method.span,
            )
        })?;
        Ok(LoxValue::Function(Rc::new(method.bind(instance))))
    }

    fn scope_of(&self, variable: &Variable) -> (&Environment, usize) {
        match self.locals.get(&variable.id) {
            Some(&depth) => (&self.environment, depth),
            None => (&self.globals, 0),
        }
    }

    fn undefined(variable: &Variable) -> RuntimeError {
        RuntimeError::new(
            RuntimeErrorKind::UndefinedVariable(variable.identifier.name.clone()),
            variable.identifier.span,
        )
    }

    fn look_up(&self, variable: &Variable) -> EvalResult {
        let (environment, depth) = self.scope_of(variable);
        environment
            .get_at(depth, &variable.identifier.name)
            .ok_or_else(|| Self::undefined(variable))
    }

    fn assign(&self, variable: &Variable, value: LoxValue) -> Result<(), RuntimeError> {
        let (environment, depth) = self.scope_of(variable);
        if environment.assign_at(depth, &variable.identifier.name, value) {
            Ok(())
        } else {
            Err(Self::undefined(variable))
        }
    }
}
