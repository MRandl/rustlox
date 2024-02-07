use peekmore::PeekMoreIterator;

use crate::error::ParsingError;
use crate::lexing::token::{Token, TokenType, TokenType::*};
use crate::parsing::expr::{BinaryOp, PositionedExpr};
use crate::position::Position;

use super::expr::{Expr, UnaryOp};

type ParsingResult = Result<PositionedExpr, Vec<ParsingError>>;

pub struct Parser<I: Iterator> {
    pub lex: PeekMoreIterator<I>,
}

impl<I: Iterator<Item = Token>> Parser<I> {
    fn next(&mut self) -> Option<Token> {
        self.lex.next()
    }

    fn peek(&mut self) -> Option<&Token> {
        self.lex.peek()
    }

    fn peek_match(&mut self, types: Vec<TokenType>) -> bool {
        match self.peek() {
            None => false,
            Some(tok) => types.contains(&tok.typ),
        }
    }

    pub fn parse(&mut self) -> ParsingResult {
        self.expression(&Position{line : 0, col : 0})
    }

    fn expression(&mut self, last_position : &Position) -> ParsingResult {
        self.equality(last_position)
    }

    fn equality(&mut self, last_position : &Position) -> ParsingResult {
        let mut expr = self.comparison(last_position)?;
        while self.peek_match(vec![BANGEQUAL, EQUALEQUAL]) {
            let tok = self.next().unwrap();
            let op = match tok.typ {
                BANGEQUAL => BinaryOp::NotEqual,
                EQUALEQUAL => BinaryOp::Equalequal,
                _ => unreachable!(),
            };
            let right = self.comparison(&tok.to_pos)?;
            expr = PositionedExpr{
                start : expr.start,
                end : right.end,
                expr : Expr::Binary {
                    e1: Box::new(expr),
                    op: op,
                    e2: Box::new(right),
                },
            }
        }
        Ok(expr)
    }

    fn comparison(&mut self, last_position : &Position) -> ParsingResult {
        let mut expr = self.term(last_position)?;
        while self.peek_match(vec![GREATER, GREATEREQUAL, LESS, LESSEQUAL]) {
            let tok = self.next().unwrap();
            let op = match tok.typ {
                GREATER => BinaryOp::Ge,
                GREATEREQUAL => BinaryOp::Geq,
                LESS => BinaryOp::Le,
                LESSEQUAL => BinaryOp::Leq,
                _ => unreachable!(),
            };
            let right = self.term(&tok.to_pos)?;
            expr = PositionedExpr{
                start : expr.start,
                end : right.end,
                expr:Expr::Binary {
                    e1: Box::new(expr),
                    op,
                    e2: Box::new(right),
                }
            }
        }
        Ok(expr)
    }

    fn term(&mut self, last_position : &Position) -> ParsingResult {
        let mut expr = self.factor(last_position)?;
        while self.peek_match(vec![MINUS, PLUS]) {
            let tok = self.next().unwrap();
            let op = match tok.typ {
                MINUS => BinaryOp::BinMinus,
                PLUS => BinaryOp::Plus,
                _ => unreachable!(),
            };
            let right = self.factor(&tok.to_pos)?;
            expr = PositionedExpr {
                start : expr.start,
                end : right.end,
                expr : Expr::Binary {
                    e1: Box::new(expr),
                    op,
                    e2: Box::new(right),
                },
            }
        }
        Ok(expr)
    }

    fn factor(&mut self, last_position : &Position) -> ParsingResult {
        let mut expr = self.unary(last_position)?;
        
        while self.peek_match(vec![SLASH, STAR]) {
            let tok = self.next().unwrap();
            let op = match tok.typ {
                SLASH => BinaryOp::BinMinus,
                STAR => BinaryOp::Times,
                _ => unreachable!(),
            };
            let right = self.unary(&tok.to_pos)?;
            expr = PositionedExpr{
                start : expr.start,
                end : right.end,
                expr : Expr::Binary {
                    e1: Box::new(expr),
                    op,
                    e2: Box::new(right),
                },
            };   
        }
        Ok(expr)
    }

    fn unary(&mut self, last_position : &Position) -> ParsingResult {
        if self.peek_match(vec![BANG, MINUS]) {
            let tok = self.next().unwrap();
            let op = match tok.typ {
                BANG => UnaryOp::Not,
                MINUS => UnaryOp::UnaMinus,
                _ => unreachable!(),
            };
            let right = self.unary(&tok.to_pos);
            match right {
                Ok(pexpr) => Ok(PositionedExpr{
                    start : tok.from_pos,
                    end : pexpr.end,
                    expr: Expr::Unary { op, e1: Box::new(pexpr) },
                }),
                Err(v) => {
                    Err(v)
                },
            }
        } else {
            self.primary(last_position)
        }
    }

    fn primary(&mut self, last_position : &Position) -> ParsingResult {
        let tok = self.next();
        match tok {
            Some(tok) => {
                if vec![FALSE, TRUE, NIL, NUMBER, STRING].contains(&tok.typ) {
                    Ok(PositionedExpr {
                        expr : match tok.typ {
                            FALSE => Expr::F,
                            TRUE => Expr::T,
                            NIL => Expr::Nil,
                            NUMBER => Expr::Num(str::parse(&tok.lexeme).unwrap()),
                            STRING => Expr::Str(tok.lexeme.clone()),
                            _ => unreachable!(),
                        },
                        start : tok.from_pos,
                        end : tok.to_pos
                    })
                } else if tok.typ == LEFTPAREN {
                    self.handle_parens(&tok.to_pos)
                } else {
                    Err(vec![ParsingError::IllegalToken { start: tok.from_pos, end: tok.to_pos, expected: "Primary token".to_string() }])
                }
            },
            None => {
                Err(vec![
                    ParsingError::EarlyEof { pos: *last_position }
                ])
            },
        }
        
    }

    fn handle_parens(&mut self, last_position : &Position) -> ParsingResult {
        let expr = self.expression(last_position);
        let par = self.next();
        if par.as_ref().is_some_and(|x| x.typ == RIGHTPAREN) {
            expr
        } else {
            match (expr, par) {
                (Ok(pexpr), None) => Err(vec![ParsingError::EarlyEof { pos: pexpr.end }, ParsingError::UnclosedParenthesis { open_paren_start: *last_position }]),
                (Ok(pexpr), Some(_)) => Ok(pexpr),
                (Err(mut vec), None) => {
                    vec.push(ParsingError::EarlyEof { pos: *last_position });
                    Err(vec)
                }
                (Err(vec), Some(_)) => Err(vec),
            }
        }
    }
}
