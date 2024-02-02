
use std::str::Chars;

use peekmore::PeekMoreIterator;

use crate::{lexing::token::{*, TokenType::*}, position::Position};

pub struct Lexer<'a> {
    pub source : PeekMoreIterator<Chars<'a>>,
    pub position : Position
}

impl Lexer<'_> {

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
        self.source.peek().is_none()
    }

    pub fn scan_token(&mut self) -> Option<Token> {
        let init_pos = self.position;

        let char = self.next()?; 

        let (token_type, lexeme) = match char {
            '(' => (Some(LEFTPAREN), None),
            ')' => (Some(RIGHTPAREN), None),
            '{' => (Some(LEFTBRACE), None),
            '}' => (Some(RIGHTBRACE), None),
            ',' => (Some(COMMA), None),
            '.' => (Some(DOT), None),
            '+' => (Some(PLUS), None),
            '-' => (Some(MINUS), None),
            ';' => (Some(SEMICOLON), None),
            '*' => (Some(STAR), None),
            '!' => 
                if self.match_peek('=') {
                    (Some(BANGEQUAL), None)
                } else {
                    (Some(BANG), None)
                }
            '=' => 
                if self.match_peek('=') {
                    (Some(EQUALEQUAL), None)
                } else {
                    (Some(EQUAL), None)
                }
            '<' => 
                if self.match_peek('=') {
                    (Some(LESSEQUAL), None)
                } else {
                    (Some(LESS), None)
                }
            '>' => 
                if self.match_peek('=') {
                    (Some(GREATEREQUAL), None)
                } else {
                    (Some(GREATER), None)
                }
            '/' =>
                if self.match_peek('/') {
                  // A comment goes until the end of the line.
                  let mut next = self.peek();
                  while next != Some('\n') && next.is_some() {
                    let _ = self.next();
                    next = self.peek();
                  }
                  (None, None)
                } else {
                  (Some(SLASH), None)
                }

            x if x.is_whitespace() => {
                let mut next = self.peek();
                while next.is_some() && next.unwrap().is_whitespace() {
                    let _ = self.next();
                    next = self.peek();
                }
                (None, None)
            }

            x if x.is_ascii_digit() => {
                let mut next = self.peek();
                let mut buf = String::with_capacity(4);
                buf.push(x);
                while next.is_some() && next.unwrap().is_ascii_digit() {
                    buf.push(next.unwrap());
                    let _ = self.next();
                    next = self.peek();
                }
                if next == Some('.') && self.source.advance_cursor().peek().is_some_and(|c| c.is_ascii_digit()){
                    buf.push('.');
                    self.source.reset_cursor();
                    let _ = self.next();

                    next = self.peek();
                    while next.is_some() && next.unwrap().is_ascii_digit() {
                        buf.push(next.unwrap());
                        let _ = self.next();
                        next = self.peek();
                    }
                }
                (Some(NUMBER), Some(buf))
            }

            x if x.is_ascii_alphabetic() => {
                let mut next = self.peek();
                let mut buf = String::new();
                buf.push(x);
                while next.is_some() && next.unwrap().is_ascii_alphabetic() {
                    buf.push(next.unwrap());
                    let _ = self.next();
                    next = self.peek();
                }
                (Some(IDENTIFIER), Some(buf))
            }

            '"' => {
                let mut next = self.peek();
                let mut buf = String::with_capacity(10);
                while next != Some('"') && next.is_some() {
                    buf.push(next.unwrap());
                    let _ = self.next();
                    next = self.peek();
                }
                if next.is_some() {
                    (Some(STRING), Some(buf))
                } else {
                    print!("Encountered unterminated string starting at position {}.", init_pos.pretty_print());
                    (None, None)
                }
            }, 
                
            _ => {
                print!("Encountered unknown character at position {}.", self.position.pretty_print());
                (None, None)
            }
            
        };

        match token_type {
            Some(typ) => {
                Some(Token {
                    typ,
                    lexeme : lexeme.unwrap_or(String::new()),
                    from_pos : init_pos,
                    to_pos : self.position
                })
            }
            | None => None

        }
        
    }

}