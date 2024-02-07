use std::slice::Iter;

use peekmore::PeekMoreIterator;

use crate::lexing::token::{Token, TokenType, TokenType::*};
use crate::parsing::expr::{BinaryOp, Expr};

use super::expr::UnaryOp;

pub struct Parser <'a> {
    pub lex : PeekMoreIterator<Iter<'a, Token>>
}

impl Parser<'_> {

    fn next(&mut self) -> Option<&Token> {
        self.lex.next()
    }

    fn peek(&mut self) -> Option<&Token> {
        self.lex.peek().copied()
    }

    fn peek_match(&mut self, types : Vec<TokenType>) -> bool {
        match self.peek() {
            None => false,
            Some(tok) => types.contains(&tok.typ)
        }
    }

    pub fn parse(&mut self) -> Option<Expr> {
        Some(self.expression())
    }

    fn expression(&mut self) -> Expr {
        self.equality()
    }

    fn equality(&mut self) -> Expr {
        let mut expr = self.comparison();
        while self.peek_match(vec![BANGEQUAL, EQUALEQUAL]) {
            let op = match self.next().unwrap().typ {
                BANGEQUAL => BinaryOp::NotEqual,
                EQUALEQUAL => BinaryOp::Equalequal,
                _ => unreachable!()
            };
            let right = self.comparison();
            expr = Expr::Binary { e1: Box::new(expr), op: op, e2: Box::new(right) }
        }
        expr
    }
    
    fn comparison(&mut self) -> Expr {
        let mut expr = self.term();
        while self.peek_match(vec![GREATER, GREATEREQUAL, LESS, LESSEQUAL]) {
            let op = match self.next().unwrap().typ {
                GREATER => BinaryOp::Ge,
                GREATEREQUAL => BinaryOp::Geq,
                LESS => BinaryOp::Le,
                LESSEQUAL => BinaryOp::Leq,
                _ => unreachable!()
            };
            let right = self.term();
            expr = Expr::Binary { e1: Box::new(expr), op, e2 : Box::new(right) }
        }
        expr
    }

    fn term(&mut self) -> Expr {
        let mut expr = self.factor();
        while self.peek_match(vec![MINUS, PLUS]) {
            let op = match self.next().unwrap().typ {
                MINUS => BinaryOp::BinMinus,
                PLUS => BinaryOp::Plus,
                _ => unreachable!()
            };
            let right = self.factor();
            expr = Expr::Binary { e1: Box::new(expr), op, e2 : Box::new(right) }
        }
        expr
    }

    fn factor(&mut self) -> Expr {
        let mut expr = self.unary();
        while self.peek_match(vec![SLASH, STAR]) {
            let op = match self.next().unwrap().typ {
                SLASH => BinaryOp::BinMinus,
                STAR => BinaryOp::Times,
                _ => unreachable!()
            };
            let right = self.unary();
            expr = Expr::Binary { e1: Box::new(expr), op, e2 : Box::new(right) }
        }
        expr
    }

    fn unary(&mut self) -> Expr {
        if self.peek_match(vec![BANG, MINUS]) {
            let op = match self.next().unwrap().typ {
                BANG => UnaryOp::Not,
                MINUS => UnaryOp::UnaMinus,
                _ => unreachable!()
            };
            let right = self.unary();
            Expr::Unary { op, e1: Box::new(right) }
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Expr {
        let tok = self.next().unwrap();
        if vec![FALSE, TRUE, NIL, NUMBER, STRING].contains(&tok.typ) {
            match tok.typ {
                FALSE => Expr::F,
                TRUE => Expr::T,
                NIL => Expr::Nil,
                NUMBER => Expr::Num(str::parse(&tok.lexeme).unwrap()),
                STRING => Expr::Str(tok.lexeme.clone()),
                _ => unreachable!()
            }
        } else if tok.typ == LEFTPAREN { 
            self.handle_pars()
        } else {
            panic!("found illegal token '{}' at position {}. Expected primary expression.", tok.lexeme, tok.from_pos)
        }
    }

    fn handle_pars(&mut self) -> Expr {
        let expr = self.expression();
        let par = self.next();
        if par.is_some_and(|x| x.typ == RIGHTPAREN) {
            expr
        } else {
            panic!("found illegal token '{}' at position {}. Expected closing parenthesis.", par.map(|x| x.lexeme.clone()).unwrap_or("EOF".to_string()), par.map(|x| x.to_pos.pretty_print()).unwrap_or("end of file".to_string()))
        }
    }
}