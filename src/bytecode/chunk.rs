use crate::{bytecode::value::Value, position::Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Constant(u8),
    Nil,
    True,
    False,
    Pop,
    GetLocal(u8),
    SetLocal(u8),
    GetGlobal(u8),
    DefineGlobal(u8),
    SetGlobal(u8),
    GetUpvalue(u8),
    SetUpvalue(u8),
    GetProperty(u8),
    SetProperty(u8),
    GetSuper(u8),
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,
    Print,
    Jump(u16),
    JumpIfFalse(u16),
    Loop(u16),
    Call(u8),
    Invoke(u8, u8),
    SuperInvoke(u8, u8),
    Closure(u8),
    CloseUpvalue,
    Return,
    Class(u8),
    Inherit,
    Method(u8),
}

impl Op {
    pub fn with_jump_offset(self, offset: u16) -> Op {
        match self {
            Op::Jump(_) => Op::Jump(offset),
            Op::JumpIfFalse(_) => Op::JumpIfFalse(offset),
            other => unreachable!("only forward jumps are patched, not {other:?}"),
        }
    }

    pub fn constant_operand(self) -> Option<u8> {
        match self {
            Op::Constant(index)
            | Op::GetGlobal(index)
            | Op::DefineGlobal(index)
            | Op::SetGlobal(index)
            | Op::GetProperty(index)
            | Op::SetProperty(index)
            | Op::GetSuper(index)
            | Op::Invoke(index, _)
            | Op::SuperInvoke(index, _)
            | Op::Closure(index)
            | Op::Class(index)
            | Op::Method(index) => Some(index),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Chunk {
    pub code: Vec<Op>,
    pub spans: Vec<Span>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn write(&mut self, op: Op, span: Span) -> usize {
        self.code.push(op);
        self.spans.push(span);
        self.code.len() - 1
    }

    pub fn add_constant(&mut self, value: Value) -> Option<u8> {
        let index = match self.constants.iter().position(|&known| known == value) {
            Some(index) => index,
            None => {
                self.constants.push(value);
                self.constants.len() - 1
            }
        };
        u8::try_from(index).ok()
    }
}
