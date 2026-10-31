use std::rc::Rc;

use crate::error::ParsingError;
use crate::lexing::token::{Token, TokenType, TokenType::*};
use crate::parsing::expr::{
    BinaryOp, Expr, Identifier, LogicalOp, PositionedExpr, UnaryOp, Variable,
};
use crate::parsing::stmt::{ClassDecl, FunctionDecl, Stmt};

const MAX_ARITY: usize = 255;

type ParsingResult<T> = Result<T, ParsingError>;
type ExprResult = ParsingResult<PositionedExpr>;
type Operand<P> = fn(&mut P) -> ExprResult;
type Combinator<O> = fn(Box<PositionedExpr>, O, Box<PositionedExpr>) -> Expr;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    errors: Vec<ParsingError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            current: 0,
            errors: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Result<Vec<Stmt>, Vec<ParsingError>> {
        let mut statements = Vec::new();
        while !self.at_end() {
            statements.extend(self.declaration());
        }
        if self.errors.is_empty() {
            Ok(statements)
        } else {
            Err(self.errors)
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn at_end(&self) -> bool {
        self.peek().is(EOF)
    }

    fn check(&self, typ: TokenType) -> bool {
        self.peek().is(typ)
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if !self.at_end() {
            self.current += 1;
        }
        token
    }

    fn advance_if(&mut self, typ: TokenType) -> Option<Token> {
        self.check(typ).then(|| self.advance())
    }

    fn expect(&mut self, typ: TokenType, expected: &str) -> ParsingResult<Token> {
        self.advance_if(typ)
            .ok_or_else(|| self.unexpected(expected))
    }

    fn unexpected(&self, expected: &str) -> ParsingError {
        let token = self.peek();
        if token.is(EOF) {
            ParsingError::EarlyEof {
                pos: token.span.start,
                expected: expected.to_string(),
            }
        } else {
            ParsingError::IllegalToken {
                span: token.span,
                found: token.lexeme.clone(),
                expected: expected.to_string(),
            }
        }
    }

    fn identifier(&mut self, expected: &str) -> ParsingResult<Identifier> {
        let token = self.expect(IDENTIFIER, expected)?;
        Ok(Identifier {
            name: token.lexeme,
            span: token.span,
        })
    }

    fn synchronize(&mut self) {
        self.advance();
        while !self.at_end() {
            if self.previous().is(SEMICOLON) || self.peek().typ.starts_statement() {
                return;
            }
            self.advance();
        }
    }

    fn declaration(&mut self) -> Option<Stmt> {
        match self.try_declaration() {
            Ok(stmt) => Some(stmt),
            Err(error) => {
                self.errors.push(error);
                self.synchronize();
                None
            }
        }
    }

    fn try_declaration(&mut self) -> ParsingResult<Stmt> {
        match self.peek().typ {
            CLASS => {
                self.advance();
                self.class_declaration()
            }
            FUN => {
                self.advance();
                Ok(Stmt::Function(Rc::new(self.function("function")?)))
            }
            VAR => {
                self.advance();
                self.var_declaration()
            }
            _ => self.statement(),
        }
    }

    fn class_declaration(&mut self) -> ParsingResult<Stmt> {
        let name = self.identifier("class name")?;
        let superclass = match self.advance_if(LESS) {
            Some(_) => Some(Variable::new(self.identifier("superclass name")?)),
            None => None,
        };
        self.expect(LEFTBRACE, "'{' before class body")?;
        let mut methods = Vec::new();
        while !self.check(RIGHTBRACE) && !self.at_end() {
            methods.push(Rc::new(self.function("method")?));
        }
        self.expect(RIGHTBRACE, "'}' after class body")?;
        Ok(Stmt::Class(ClassDecl {
            name,
            superclass,
            methods,
        }))
    }

    fn function(&mut self, kind: &str) -> ParsingResult<FunctionDecl> {
        let name = self.identifier(&format!("{kind} name"))?;
        self.expect(LEFTPAREN, &format!("'(' after {kind} name"))?;
        let params =
            self.parenthesized_list("parameters", |parser| parser.identifier("parameter name"))?;
        self.expect(LEFTBRACE, &format!("'{{' before {kind} body"))?;
        let body = self.block()?;
        Ok(FunctionDecl { name, params, body })
    }

    fn parenthesized_list<T>(
        &mut self,
        kind: &'static str,
        item: impl Fn(&mut Self) -> ParsingResult<T>,
    ) -> ParsingResult<Vec<T>> {
        let mut items = Vec::new();
        if !self.check(RIGHTPAREN) {
            loop {
                if items.len() >= MAX_ARITY {
                    let pos = self.peek().span.start;
                    self.errors.push(ParsingError::TooManyItems { kind, pos });
                }
                items.push(item(self)?);
                if self.advance_if(COMMA).is_none() {
                    break;
                }
            }
        }
        self.expect(RIGHTPAREN, &format!("')' after {kind}"))?;
        Ok(items)
    }

    fn var_declaration(&mut self) -> ParsingResult<Stmt> {
        let name = self.identifier("variable name")?;
        let initializer = match self.advance_if(EQUAL) {
            Some(_) => Some(self.expression()?),
            None => None,
        };
        self.expect(SEMICOLON, "';' after variable declaration")?;
        Ok(Stmt::Var { name, initializer })
    }

    fn statement(&mut self) -> ParsingResult<Stmt> {
        match self.peek().typ {
            FOR => {
                self.advance();
                self.for_statement()
            }
            IF => {
                self.advance();
                self.if_statement()
            }
            PRINT => {
                self.advance();
                self.print_statement()
            }
            RETURN => {
                self.advance();
                self.return_statement()
            }
            WHILE => {
                self.advance();
                self.while_statement()
            }
            LEFTBRACE => {
                self.advance();
                Ok(Stmt::Block(self.block()?))
            }
            _ => self.expression_statement(),
        }
    }

    fn for_statement(&mut self) -> ParsingResult<Stmt> {
        let keyword = self.previous().span;
        self.expect(LEFTPAREN, "'(' after 'for'")?;
        let initializer = match self.peek().typ {
            SEMICOLON => {
                self.advance();
                None
            }
            VAR => {
                self.advance();
                Some(self.var_declaration()?)
            }
            _ => Some(self.expression_statement()?),
        };
        let condition = match self.check(SEMICOLON) {
            true => PositionedExpr::new(Expr::T, keyword),
            false => self.expression()?,
        };
        self.expect(SEMICOLON, "';' after loop condition")?;
        let increment = match self.check(RIGHTPAREN) {
            true => None,
            false => Some(self.expression()?),
        };
        self.expect(RIGHTPAREN, "')' after for clauses")?;

        let mut body = self.statement()?;
        if let Some(increment) = increment {
            body = Stmt::Block(vec![body, Stmt::Expression(increment)]);
        }
        let mut desugared = Stmt::While {
            condition,
            body: Box::new(body),
        };
        if let Some(initializer) = initializer {
            desugared = Stmt::Block(vec![initializer, desugared]);
        }
        Ok(desugared)
    }

    fn if_statement(&mut self) -> ParsingResult<Stmt> {
        self.expect(LEFTPAREN, "'(' after 'if'")?;
        let condition = self.expression()?;
        self.expect(RIGHTPAREN, "')' after if condition")?;
        let then_branch = Box::new(self.statement()?);
        let else_branch = match self.advance_if(ELSE) {
            Some(_) => Some(Box::new(self.statement()?)),
            None => None,
        };
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn print_statement(&mut self) -> ParsingResult<Stmt> {
        let value = self.expression()?;
        self.expect(SEMICOLON, "';' after value")?;
        Ok(Stmt::Print(value))
    }

    fn return_statement(&mut self) -> ParsingResult<Stmt> {
        let keyword = self.previous().span;
        let value = match self.check(SEMICOLON) {
            true => None,
            false => Some(self.expression()?),
        };
        self.expect(SEMICOLON, "';' after return value")?;
        Ok(Stmt::Return { keyword, value })
    }

    fn while_statement(&mut self) -> ParsingResult<Stmt> {
        self.expect(LEFTPAREN, "'(' after 'while'")?;
        let condition = self.expression()?;
        self.expect(RIGHTPAREN, "')' after condition")?;
        let body = Box::new(self.statement()?);
        Ok(Stmt::While { condition, body })
    }

    fn block(&mut self) -> ParsingResult<Vec<Stmt>> {
        let mut statements = Vec::new();
        while !self.check(RIGHTBRACE) && !self.at_end() {
            statements.extend(self.declaration());
        }
        self.expect(RIGHTBRACE, "'}' after block")?;
        Ok(statements)
    }

    fn expression_statement(&mut self) -> ParsingResult<Stmt> {
        let expr = self.expression()?;
        self.expect(SEMICOLON, "';' after expression")?;
        Ok(Stmt::Expression(expr))
    }

    fn expression(&mut self) -> ExprResult {
        self.assignment()
    }

    fn assignment(&mut self) -> ExprResult {
        let target = self.or()?;
        let Some(equals) = self.advance_if(EQUAL) else {
            return Ok(target);
        };
        let value = Box::new(self.assignment()?);
        let span = target.span.to(value.span);
        let expr = match target.expr {
            Expr::Variable(target) => Expr::Assign { target, value },
            Expr::Get { object, name } => Expr::Set {
                object,
                name,
                value,
            },
            invalid => {
                self.errors.push(ParsingError::InvalidAssignmentTarget {
                    pos: equals.span.start,
                });
                return Ok(PositionedExpr::new(invalid, target.span));
            }
        };
        Ok(PositionedExpr::new(expr, span))
    }

    fn left_associative<O>(
        &mut self,
        operand: Operand<Self>,
        operator: fn(TokenType) -> Option<O>,
        combine: Combinator<O>,
    ) -> ExprResult {
        let mut expr = operand(self)?;
        while let Some(op) = operator(self.peek().typ) {
            self.advance();
            let right = operand(self)?;
            let span = expr.span.to(right.span);
            expr = PositionedExpr::new(combine(Box::new(expr), op, Box::new(right)), span);
        }
        Ok(expr)
    }

    fn or(&mut self) -> ExprResult {
        self.left_associative(Self::and, or_operator, logical)
    }

    fn and(&mut self) -> ExprResult {
        self.left_associative(Self::equality, and_operator, logical)
    }

    fn equality(&mut self) -> ExprResult {
        self.left_associative(Self::comparison, equality_operator, binary)
    }

    fn comparison(&mut self) -> ExprResult {
        self.left_associative(Self::term, comparison_operator, binary)
    }

    fn term(&mut self) -> ExprResult {
        self.left_associative(Self::factor, term_operator, binary)
    }

    fn factor(&mut self) -> ExprResult {
        self.left_associative(Self::unary, factor_operator, binary)
    }

    fn unary(&mut self) -> ExprResult {
        let Some(op) = unary_operator(self.peek().typ) else {
            return self.call();
        };
        let operator = self.advance();
        let operand = self.unary()?;
        let span = operator.span.to(operand.span);
        Ok(PositionedExpr::new(
            Expr::Unary {
                op,
                e1: Box::new(operand),
            },
            span,
        ))
    }

    fn call(&mut self) -> ExprResult {
        let mut expr = self.primary()?;
        loop {
            if self.advance_if(LEFTPAREN).is_some() {
                let args = self.parenthesized_list("arguments", Self::expression)?;
                let span = expr.span.to(self.previous().span);
                let callee = Box::new(expr);
                expr = PositionedExpr::new(Expr::Call { callee, args }, span);
            } else if self.advance_if(DOT).is_some() {
                let name = self.identifier("property name after '.'")?;
                let span = expr.span.to(name.span);
                let object = Box::new(expr);
                expr = PositionedExpr::new(Expr::Get { object, name }, span);
            } else {
                return Ok(expr);
            }
        }
    }

    fn primary(&mut self) -> ExprResult {
        let token = self.peek().clone();
        let expr = match token.typ {
            FALSE => Expr::F,
            TRUE => Expr::T,
            NIL => Expr::Nil,
            NUMBER | STRING => Expr::from(
                token
                    .literal
                    .expect("number and string tokens carry their literal value"),
            ),
            THIS => Expr::This(Variable::new(Identifier {
                name: token.lexeme,
                span: token.span,
            })),
            IDENTIFIER => Expr::Variable(Variable::new(Identifier {
                name: token.lexeme,
                span: token.span,
            })),
            SUPER => return self.super_access(),
            LEFTPAREN => return self.grouping(),
            _ => return Err(self.unexpected("expression")),
        };
        self.advance();
        Ok(PositionedExpr::new(expr, token.span))
    }

    fn super_access(&mut self) -> ExprResult {
        let keyword = self.advance();
        self.expect(DOT, "'.' after 'super'")?;
        let method = self.identifier("superclass method name")?;
        let span = keyword.span.to(method.span);
        let keyword = Variable::new(Identifier {
            name: keyword.lexeme,
            span: keyword.span,
        });
        Ok(PositionedExpr::new(Expr::Super { keyword, method }, span))
    }

    fn grouping(&mut self) -> ExprResult {
        let open = self.advance();
        let inner = self.expression()?;
        let close = self
            .advance_if(RIGHTPAREN)
            .ok_or(ParsingError::UnclosedParenthesis {
                open_paren_start: open.span.start,
            })?;
        Ok(PositionedExpr::new(
            Expr::Grouping(Box::new(inner)),
            open.span.to(close.span),
        ))
    }
}

fn binary(e1: Box<PositionedExpr>, op: BinaryOp, e2: Box<PositionedExpr>) -> Expr {
    Expr::Binary { e1, op, e2 }
}

fn logical(e1: Box<PositionedExpr>, op: LogicalOp, e2: Box<PositionedExpr>) -> Expr {
    Expr::Logical { e1, op, e2 }
}

fn or_operator(typ: TokenType) -> Option<LogicalOp> {
    (typ == OR).then_some(LogicalOp::Or)
}

fn and_operator(typ: TokenType) -> Option<LogicalOp> {
    (typ == AND).then_some(LogicalOp::And)
}

fn equality_operator(typ: TokenType) -> Option<BinaryOp> {
    match typ {
        BANGEQUAL => Some(BinaryOp::NotEqual),
        EQUALEQUAL => Some(BinaryOp::Equalequal),
        _ => None,
    }
}

fn comparison_operator(typ: TokenType) -> Option<BinaryOp> {
    match typ {
        GREATER => Some(BinaryOp::Ge),
        GREATEREQUAL => Some(BinaryOp::Geq),
        LESS => Some(BinaryOp::Le),
        LESSEQUAL => Some(BinaryOp::Leq),
        _ => None,
    }
}

fn term_operator(typ: TokenType) -> Option<BinaryOp> {
    match typ {
        MINUS => Some(BinaryOp::BinMinus),
        PLUS => Some(BinaryOp::Plus),
        _ => None,
    }
}

fn factor_operator(typ: TokenType) -> Option<BinaryOp> {
    match typ {
        SLASH => Some(BinaryOp::Div),
        STAR => Some(BinaryOp::Times),
        _ => None,
    }
}

fn unary_operator(typ: TokenType) -> Option<UnaryOp> {
    match typ {
        BANG => Some(UnaryOp::Not),
        MINUS => Some(UnaryOp::UnaMinus),
        _ => None,
    }
}
