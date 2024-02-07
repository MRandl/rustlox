use crate::position::Position;

#[derive(Debug)]
pub enum UnaryOp {
    UnaMinus,
    Not,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum Expr {
    Num(i64),
    Str(String),
    T,
    F,
    Nil,

    Unary {
        op: UnaryOp,
        e1: Box<PositionedExpr>,
    },
    Binary {
        e1: Box<PositionedExpr>,
        op: BinaryOp,
        e2: Box<PositionedExpr>,
    },
}

#[derive(Debug)]
pub struct PositionedExpr {
    pub expr : Expr,
    pub start : Position,
    pub end : Position
}