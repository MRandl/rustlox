#![allow(dead_code)]

use peekmore::PeekMore;

use crate::position::Position;

mod lexing;
mod position;

fn main() {
    let mut lex = lexing::lexer::Lexer {
        source: "\"t'is this time of the year\" - 123321".chars().peekmore(),
        position: Position::default(),
    };

    while !lex.is_done() {
        if let Some(x) = lex.scan_token() {
            println!("{:?}", x)
        }
    }
    println!("Hello, world!");
}
