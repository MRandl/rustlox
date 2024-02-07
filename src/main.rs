#![allow(dead_code)]

use peekmore::PeekMore;
use std::{
    fs,
    io::{BufReader, Read},
};

use crate::position::Position;

mod error;
mod interpreting;
mod lexing;
mod parsing;
mod position;

fn main() {
    let mut content = String::with_capacity(1000);
    BufReader::new(fs::File::open("src/loxsrc/main.lox").unwrap())
        .read_to_string(&mut content)
        .unwrap();

    let mut lex = lexing::lexer::Lexer {
        source: "false == (2 == false)".chars().peekmore(),
        position: Position::default(),
    };

    let tokens = std::iter::from_fn(|| lex.scan_token());

    let mut parser = parsing::parser::Parser {
        lex: tokens.peekmore(),
    };
    let full_expr = parser.parse();
    match full_expr {
        Some(ex) => println!("{:?}", interpreting::interpret::interpret(&ex)),
        None => println!("could not parse"),
    }
    println!("Hello, world!");
}
