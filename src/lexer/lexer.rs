use std::str::Chars;

use crate::lexer::token::{*, TokenType::*};
use crate::position::*;

pub struct Lexer<'a> {
    source : Chars<'a>,
    position : Position
}

impl Lexer<'_> {

    pub fn scan_token(&mut self) -> Option<Token> {
        
        let char = self.source.next()?; 
        let token_type: TokenType = match char {
            '(' => Some(LEFTPAREN),
            ')' => Some(RIGHTPAREN),
            '{' => Some(LEFTBRACE),
            '}' => Some(RIGHTBRACE),
            ',' => Some(COMMA),
            '.' => Some(DOT),
            '+' => Some(PLUS),
            '-' => Some(MINUS),
            ';' => Some(SEMICOLON),
            '*' => Some(STAR),
            _ => {
                print!("Encountered unknown character at position {}.", self.position.pretty_print());
                None
            }
            
        }?;

        Some(Token {
            typ : token_type,
            lexeme : " ",
            pos : self.position
        })
    }

}