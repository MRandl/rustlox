use crate::position::Position;

pub enum LexingError {}

pub enum ParsingError {
    EarlyEof { pos: Position },
    UnclosedParenthesis { open_paren_start: Position },
    IllegalToken { start : Position, end : Position, expected : String }
}

#[derive(Debug)]
pub struct LoxError {
    pub msg: String,
}
