#![allow(dead_code)]

use peekmore::PeekMore;

use crate::position::Position;

mod lexing;
mod position;
mod parsing;
mod interpreting;
mod error;

fn main() {
    let mut lex = lexing::lexer::Lexer {
        source: "false == (2 == false)".chars().peekmore(),
        position: Position::default(),
    };

    let mut tokens = Vec::new();
    while !lex.is_done() {
        if let Some(x) = lex.scan_token() {
           tokens.push(x)
        }
    }

    let mut parser = parsing::parser::Parser{lex : tokens.iter().peekmore()};
    let full_expr = parser.parse();
    match full_expr {
        Some(ex) => println!("{:?}", interpreting::interpret::interpret(&ex)),
        None => println!("could not parse"),
    }
    println!("Hello, world!");
}
