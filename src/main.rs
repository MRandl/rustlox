#![allow(dead_code)]

use crate::position::Position;

mod lexing;
mod position;

fn main() {
    let mut lex = lexing::lexer::Lexer {
        source : "// this is a comment\n(( )){} // grouping stuff\n!*+-/=<> <= == // operators".chars().peekable(),
        position: Position::default()
    };

    while !lex.is_done() {
        if let Some(x) = lex.scan_token()  {
            println!("{:?}", x)
        }
    }
    println!("Hello, world!");
}
