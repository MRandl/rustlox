use std::rc::Rc;

use crate::{
    parsing::expr::{Identifier, PositionedExpr, Variable},
    position::Span,
};

#[derive(Debug)]
pub enum Stmt {
    Expression(PositionedExpr),
    Print(PositionedExpr),
    Var {
        name: Identifier,
        initializer: Option<PositionedExpr>,
    },
    Block(Vec<Stmt>),
    If {
        condition: PositionedExpr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        condition: PositionedExpr,
        body: Box<Stmt>,
    },
    Function(Rc<FunctionDecl>),
    Return {
        keyword: Span,
        value: Option<PositionedExpr>,
    },
    Class(ClassDecl),
}

#[derive(Debug)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub params: Vec<Identifier>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct ClassDecl {
    pub name: Identifier,
    pub superclass: Option<Variable>,
    pub methods: Vec<Rc<FunctionDecl>>,
}
