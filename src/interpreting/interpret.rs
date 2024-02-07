use crate::{error::LoxError, parsing::expr::*};
use LoxValue::*;

#[derive(PartialEq, Eq, Debug)]
pub enum LoxValue {
    Int(i64),
    Bool(bool),
    Strng(String),
    Nil,
}

pub fn interpret(pexpr: &PositionedExpr) -> Result<LoxValue, LoxError> {
    match &pexpr.expr {
        Expr::Nil => Ok(Nil),
        Expr::Num(x) => Ok(Int(*x)),
        Expr::Str(s) => Ok(Strng(s.to_string())),
        Expr::T => Ok(Bool(true)),
        Expr::F => Ok(Bool(false)),
        Expr::Unary { op, e1 } => match (op, interpret(e1.as_ref())) {
            (_, Err(err)) => Err(err),
            (UnaryOp::UnaMinus, Ok(Int(x))) => Ok(Int(-x)),
            (UnaryOp::UnaMinus, Ok(Bool(_))) => Err(LoxError {
                msg: "opposite of bool is undefined".to_string(),
            }),
            (UnaryOp::UnaMinus, Ok(Strng(_))) => Err(LoxError {
                msg: "opposite of string is undefined".to_string(),
            }),
            (UnaryOp::UnaMinus, Ok(Nil)) => Err(LoxError {
                msg: "opposite of nil is undefined".to_string(),
            }),
            (UnaryOp::Not, Ok(Int(_))) => Err(LoxError {
                msg: "negation of int is undefined".to_string(),
            }),
            (UnaryOp::Not, Ok(Bool(b))) => Ok(Bool(!b)),
            (UnaryOp::Not, Ok(Strng(_))) => Err(LoxError {
                msg: "negation of string is undefined".to_string(),
            }),
            (UnaryOp::Not, Ok(Nil)) => Err(LoxError {
                msg: "negation of nil is undefined".to_string(),
            }),
        },

        Expr::Binary { e1, op, e2 } => match (interpret(e1.as_ref()), op, interpret(e2.as_ref())) {
            (Err(e1), _, _) => Err(e1),
            (_, _, Err(e2)) => Err(e2),
            (Ok(i1), BinaryOp::Equalequal, Ok(i2)) => Ok(Bool(i1 == i2)),
            (Ok(i1), BinaryOp::NotEqual, Ok(i2)) => Ok(Bool(i1 != i2)),
            (Ok(i1), BinaryOp::Le, Ok(i2)) => {
                if let (Int(n1), Int(n2)) = (i1, i2) {
                    Ok(Bool(n1 < n2))
                } else {
                    Err(LoxError {
                        msg: "Comparison should be between numbers only".to_string(),
                    })
                }
            }
            (Ok(i1), BinaryOp::Leq, Ok(i2)) => {
                if let (Int(n1), Int(n2)) = (i1, i2) {
                    Ok(Bool(n1 <= n2))
                } else {
                    Err(LoxError {
                        msg: "Comparison should be between numbers only".to_string(),
                    })
                }
            }
            (Ok(i1), BinaryOp::Ge, Ok(i2)) => {
                if let (Int(n1), Int(n2)) = (i1, i2) {
                    Ok(Bool(n1 > n2))
                } else {
                    Err(LoxError {
                        msg: "Comparison should be between numbers only".to_string(),
                    })
                }
            }
            (Ok(i1), BinaryOp::Geq, Ok(i2)) => {
                if let (Int(n1), Int(n2)) = (i1, i2) {
                    Ok(Bool(n1 > n2))
                } else {
                    Err(LoxError {
                        msg: "Comparison should be between numbers only".to_string(),
                    })
                }
            }
            (Ok(i1), BinaryOp::Plus, Ok(i2)) => match (i1, i2) {
                (Int(n1), Int(n2)) => Ok(Int(n1 + n2)),
                (Strng(n1), Strng(n2)) => Ok(Strng(format!("{}{}", &n1, &n2))),
                _ => Err(LoxError {
                    msg: "Addition should be between numbers or strings only".to_string(),
                }),
            },
            (Ok(i1), BinaryOp::BinMinus, Ok(i2)) => {
                if let (Int(n1), Int(n2)) = (i1, i2) {
                    Ok(Int(n1 - n2))
                } else {
                    Err(LoxError {
                        msg: "Subtraction should be between numbers only".to_string(),
                    })
                }
            }
            (Ok(i1), BinaryOp::Times, Ok(i2)) => {
                if let (Int(n1), Int(n2)) = (i1, i2) {
                    Ok(Int(n1 * n2))
                } else {
                    Err(LoxError {
                        msg: "Multiplication should be between numbers only".to_string(),
                    })
                }
            }
            (Ok(i1), BinaryOp::Div, Ok(i2)) => {
                if let (Int(n1), Int(n2)) = (i1, i2) {
                    Ok(Int(n1 / n2))
                } else {
                    Err(LoxError {
                        msg: "Multiplication should be between numbers only".to_string(),
                    })
                }
            }
        },
    }
}
