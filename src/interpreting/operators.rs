use crate::{
    error::RuntimeErrorKind,
    interpreting::value::LoxValue::{self, *},
    parsing::expr::{BinaryOp, UnaryOp},
};

type OperationResult = Result<LoxValue, RuntimeErrorKind>;

pub fn unary(op: UnaryOp, operand: LoxValue) -> OperationResult {
    match (op, operand) {
        (UnaryOp::Not, operand) => Ok(Bool(!operand.is_truthy())),
        (UnaryOp::UnaMinus, Number(n)) => Ok(Number(-n)),
        (UnaryOp::UnaMinus, _) => Err(RuntimeErrorKind::OperandMustBeNumber),
    }
}

pub fn binary(op: BinaryOp, left: LoxValue, right: LoxValue) -> OperationResult {
    match op {
        BinaryOp::Equalequal => Ok(Bool(left == right)),
        BinaryOp::NotEqual => Ok(Bool(left != right)),
        BinaryOp::Plus => add(left, right),
        BinaryOp::BinMinus => numbers(left, right).map(|(a, b)| Number(a - b)),
        BinaryOp::Times => numbers(left, right).map(|(a, b)| Number(a * b)),
        BinaryOp::Div => numbers(left, right).map(|(a, b)| Number(a / b)),
        BinaryOp::Le => numbers(left, right).map(|(a, b)| Bool(a < b)),
        BinaryOp::Leq => numbers(left, right).map(|(a, b)| Bool(a <= b)),
        BinaryOp::Ge => numbers(left, right).map(|(a, b)| Bool(a > b)),
        BinaryOp::Geq => numbers(left, right).map(|(a, b)| Bool(a >= b)),
    }
}

fn add(left: LoxValue, right: LoxValue) -> OperationResult {
    match (left, right) {
        (Number(a), Number(b)) => Ok(Number(a + b)),
        (Str(a), Str(b)) => Ok(Str(format!("{a}{b}").into())),
        _ => Err(RuntimeErrorKind::OperandsMustBeNumbersOrStrings),
    }
}

fn numbers(left: LoxValue, right: LoxValue) -> Result<(f64, f64), RuntimeErrorKind> {
    match (left, right) {
        (Number(a), Number(b)) => Ok((a, b)),
        _ => Err(RuntimeErrorKind::OperandsMustBeNumbers),
    }
}
