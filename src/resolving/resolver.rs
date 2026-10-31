use std::collections::HashMap;
use std::mem;

use crate::error::{ResolutionError, ResolutionErrorKind};
use crate::parsing::expr::{Expr, ExprId, Identifier, PositionedExpr, Variable};
use crate::parsing::stmt::{ClassDecl, FunctionDecl, Stmt};
use crate::position::Span;

pub type Locals = HashMap<ExprId, usize>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionKind {
    None,
    Function,
    Initializer,
    Method,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClassKind {
    None,
    Class,
    Subclass,
}

pub fn resolve(statements: &[Stmt]) -> Result<Locals, Vec<ResolutionError>> {
    let mut resolver = Resolver {
        scopes: Vec::new(),
        locals: Locals::new(),
        function: FunctionKind::None,
        class: ClassKind::None,
        errors: Vec::new(),
    };
    resolver.resolve_stmts(statements);
    if resolver.errors.is_empty() {
        Ok(resolver.locals)
    } else {
        Err(resolver.errors)
    }
}

struct Resolver {
    scopes: Vec<HashMap<String, bool>>,
    locals: Locals,
    function: FunctionKind,
    class: ClassKind,
    errors: Vec<ResolutionError>,
}

impl Resolver {
    fn report(&mut self, kind: ResolutionErrorKind, span: Span) {
        self.errors.push(ResolutionError { kind, span });
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, identifier: &Identifier) {
        let Some(scope) = self.scopes.last_mut() else {
            return;
        };
        if scope.insert(identifier.name.clone(), false).is_some() {
            self.report(ResolutionErrorKind::AlreadyDeclared, identifier.span);
        }
    }

    fn define(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), true);
        }
    }

    fn resolve_local(&mut self, variable: &Variable) {
        let name = &variable.identifier.name;
        if let Some(depth) = self
            .scopes
            .iter()
            .rev()
            .position(|scope| scope.contains_key(name))
        {
            self.locals.insert(variable.id, depth);
        }
    }

    fn resolve_stmts(&mut self, statements: &[Stmt]) {
        for statement in statements {
            self.resolve_stmt(statement);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expression(expr) | Stmt::Print(expr) => self.resolve_expr(expr),
            Stmt::Var { name, initializer } => {
                self.declare(name);
                if let Some(initializer) = initializer {
                    self.resolve_expr(initializer);
                }
                self.define(&name.name);
            }
            Stmt::Block(statements) => {
                self.begin_scope();
                self.resolve_stmts(statements);
                self.end_scope();
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expr(condition);
                self.resolve_stmt(then_branch);
                if let Some(else_branch) = else_branch {
                    self.resolve_stmt(else_branch);
                }
            }
            Stmt::While { condition, body } => {
                self.resolve_expr(condition);
                self.resolve_stmt(body);
            }
            Stmt::Function(declaration) => {
                self.declare(&declaration.name);
                self.define(&declaration.name.name);
                self.resolve_function(declaration, FunctionKind::Function);
            }
            Stmt::Return { keyword, value } => self.resolve_return(*keyword, value.as_ref()),
            Stmt::Class(declaration) => self.resolve_class(declaration),
        }
    }

    fn resolve_return(&mut self, keyword: Span, value: Option<&PositionedExpr>) {
        if self.function == FunctionKind::None {
            self.report(ResolutionErrorKind::TopLevelReturn, keyword);
        }
        if let Some(value) = value {
            if self.function == FunctionKind::Initializer {
                self.report(ResolutionErrorKind::ValueReturnedFromInitializer, keyword);
            }
            self.resolve_expr(value);
        }
    }

    fn resolve_function(&mut self, declaration: &FunctionDecl, kind: FunctionKind) {
        let enclosing = mem::replace(&mut self.function, kind);
        self.begin_scope();
        for param in &declaration.params {
            self.declare(param);
            self.define(&param.name);
        }
        self.resolve_stmts(&declaration.body);
        self.end_scope();
        self.function = enclosing;
    }

    fn resolve_class(&mut self, declaration: &ClassDecl) {
        let enclosing = mem::replace(&mut self.class, ClassKind::Class);
        self.declare(&declaration.name);
        self.define(&declaration.name.name);

        if let Some(superclass) = &declaration.superclass {
            if superclass.identifier.name == declaration.name.name {
                self.report(
                    ResolutionErrorKind::SelfInheritance,
                    superclass.identifier.span,
                );
            }
            self.class = ClassKind::Subclass;
            self.resolve_local(superclass);
            self.begin_scope();
            self.define("super");
        }

        self.begin_scope();
        self.define("this");
        for method in &declaration.methods {
            let kind = match method.name.name.as_str() {
                "init" => FunctionKind::Initializer,
                _ => FunctionKind::Method,
            };
            self.resolve_function(method, kind);
        }
        self.end_scope();

        if declaration.superclass.is_some() {
            self.end_scope();
        }
        self.class = enclosing;
    }

    fn resolve_expr(&mut self, pexpr: &PositionedExpr) {
        match &pexpr.expr {
            Expr::Num(_) | Expr::Str(_) | Expr::T | Expr::F | Expr::Nil => {}
            Expr::Grouping(inner) | Expr::Unary { e1: inner, .. } => self.resolve_expr(inner),
            Expr::Binary { e1, e2, .. } | Expr::Logical { e1, e2, .. } => {
                self.resolve_expr(e1);
                self.resolve_expr(e2);
            }
            Expr::Variable(variable) => self.resolve_variable(variable),
            Expr::Assign { target, value } => {
                self.resolve_expr(value);
                self.resolve_local(target);
            }
            Expr::Call { callee, args } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::Get { object, .. } => self.resolve_expr(object),
            Expr::Set { object, value, .. } => {
                self.resolve_expr(value);
                self.resolve_expr(object);
            }
            Expr::This(keyword) => match self.class {
                ClassKind::None => self.report(
                    ResolutionErrorKind::ThisOutsideClass,
                    keyword.identifier.span,
                ),
                ClassKind::Class | ClassKind::Subclass => self.resolve_local(keyword),
            },
            Expr::Super { keyword, .. } => match self.class {
                ClassKind::None => self.report(
                    ResolutionErrorKind::SuperOutsideClass,
                    keyword.identifier.span,
                ),
                ClassKind::Class => self.report(
                    ResolutionErrorKind::SuperWithoutSuperclass,
                    keyword.identifier.span,
                ),
                ClassKind::Subclass => self.resolve_local(keyword),
            },
        }
    }

    fn resolve_variable(&mut self, variable: &Variable) {
        let declared_but_undefined = self
            .scopes
            .last()
            .and_then(|scope| scope.get(&variable.identifier.name))
            == Some(&false);
        if declared_but_undefined {
            self.report(
                ResolutionErrorKind::ReadInOwnInitializer,
                variable.identifier.span,
            );
        }
        self.resolve_local(variable);
    }
}
