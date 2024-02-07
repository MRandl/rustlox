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
    BufReader::new(fs::File::open("loxsrc/main.lox").unwrap())
        .read_to_string(&mut content)
        .unwrap();

    let mut lex = lexing::lexer::Lexer {
        source: content.chars().peekmore(),
        position: Position::default(),
    };

    let tokens = std::iter::from_fn(|| lex.scan_token())
        .filter(|x| x.typ != lexing::token::TokenType::WHITESPACE);

    let mut parser = parsing::parser::Parser {
        lex: tokens.peekmore(),
    };
    let full_expr = parser.parse();
    match full_expr {
        Ok(pexpr) => {println!("{:?}", interpreting::interpret::interpret(&pexpr))},
        Err(v) => {
            for err in v {
                match err {
                    error::ParsingError::EarlyEof { pos } => todo!(),
                    error::ParsingError::UnclosedParenthesis { open_paren_start } => todo!(),
                    error::ParsingError::IllegalToken { start, end, expected } => todo!(),
                }
            }
        },
    }

}
