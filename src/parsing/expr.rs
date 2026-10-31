use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{lexing::token::Literal, position::Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExprId(usize);

impl ExprId {
    pub fn fresh() -> ExprId {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        ExprId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub id: ExprId,
    pub identifier: Identifier,
}

impl Variable {
    pub fn new(identifier: Identifier) -> Variable {
        Variable {
            id: ExprId::fresh(),
            identifier,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
    UnaMinus,
    Not,
}

#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
    Equalequal,
    NotEqual,
    Le,
    Leq,
    Ge,
    Geq,
    Plus,
    BinMinus,
    Times,
    Div,
}

#[derive(Clone, Copy, Debug)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug)]
pub enum Expr {
    Num(f64),
    Str(String),
    T,
    F,
    Nil,

    Grouping(Box<PositionedExpr>),
    Unary {
        op: UnaryOp,
        e1: Box<PositionedExpr>,
    },
    Binary {
        e1: Box<PositionedExpr>,
        op: BinaryOp,
        e2: Box<PositionedExpr>,
    },
    Logical {
        e1: Box<PositionedExpr>,
        op: LogicalOp,
        e2: Box<PositionedExpr>,
    },

    Variable(Variable),
    Assign {
        target: Variable,
        value: Box<PositionedExpr>,
    },

    Call {
        callee: Box<PositionedExpr>,
        args: Vec<PositionedExpr>,
    },
    Get {
        object: Box<PositionedExpr>,
        name: Identifier,
    },
    Set {
        object: Box<PositionedExpr>,
        name: Identifier,
        value: Box<PositionedExpr>,
    },
    This(Variable),
    Super {
        keyword: Variable,
        method: Identifier,
    },
}

impl From<Literal> for Expr {
    fn from(literal: Literal) -> Expr {
        match literal {
            Literal::Number(n) => Expr::Num(n),
            Literal::Str(s) => Expr::Str(s),
        }
    }
}

#[derive(Debug)]
pub struct PositionedExpr {
    pub expr: Expr,
    pub span: Span,
}

impl PositionedExpr {
    pub fn new(expr: Expr, span: Span) -> PositionedExpr {
        PositionedExpr { expr, span }
    }
}
