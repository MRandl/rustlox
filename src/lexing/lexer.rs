
use std::{iter::Peekable, str::Chars};

use crate::{lexing::token::{*, TokenType::*}, position::Position};

pub struct Lexer<'a> {
    pub source : Peekable<Chars<'a>>,
    pub position : Position
}

impl <'a> Lexer<'a> {

    fn next(&mut self) -> Option<char> {
        let char = self.source.next()?;
        if char == '\n' {
            self.position = self.position.brk()
        } else {
            self.position = self.position.bump(1)
        }
        Some(char)
    }

    fn peek(&mut self) -> Option<char> {
        self.source.peek().copied()
    }

    fn match_peek(&mut self, expected : char) -> bool {
        let matching = self.peek() == Some(expected);
        if matching {
            let _ = self.next(); //force consumption if match
        }
        matching
    }

    pub fn is_done(&mut self) -> bool {
        self.source.peek() == None
    }

    pub fn scan_token(&mut self) -> Option<Token> {
        let init_pos = self.position;

        let char = self.next()?; 

        let token_type = match char {
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
            '!' => 
                if self.match_peek('=') {
                    Some(BANGEQUAL)
                } else {
                    Some(BANG)
                }
            '=' => 
                if self.match_peek('=') {
                    Some(EQUALEQUAL)
                } else {
                    Some(EQUAL)
                }
            '<' => 
                if self.match_peek('=') {
                    Some(LESSEQUAL)
                } else {
                    Some(LESS)
                }
            '>' => 
                if self.match_peek('=') {
                    Some(GREATEREQUAL)
                } else {
                    Some(GREATER)
                }
            '/' =>
                if self.match_peek('/') {
                  // A comment goes until the end of the line.
                  while self.peek() != Some('\n') && self.peek().is_some() {
                    let _ = self.next();
                  }
                  None
                } else {
                  Some(SLASH)
                }

            x if x.is_whitespace() => {
                let mut next = self.peek();
                while next.is_some() && next.unwrap().is_whitespace() {
                    let _ = self.next();
                    next = self.peek();
                }
                None
            }
                
            _ => {
                print!("Encountered unknown character at position {}.", self.position.pretty_print());
                None
            }
            
        };

        match token_type {
            Some(typ) => {
                Some(Token {
                    typ,
                    lexeme : " ",
                    from_pos : init_pos,
                    to_pos : self.position
                })
            }
            | None => None

        }
        
    }

}