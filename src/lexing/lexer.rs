use std::str::Chars;

use peekmore::{PeekMore, PeekMoreIterator};

use crate::{
    error::LexingError,
    lexing::token::{Literal, Token, TokenType, TokenType::*},
    position::{Position, Span},
};

pub struct Lexer<'a> {
    source: PeekMoreIterator<Chars<'a>>,
    position: Position,
    start: Position,
    lexeme: String,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Lexer<'a> {
        Lexer {
            source: source.chars().peekmore(),
            position: Position::default(),
            start: Position::default(),
            lexeme: String::new(),
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Vec<LexingError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        while let Some(scanned) = self.scan_token() {
            match scanned {
                Ok(token) => tokens.push(token),
                Err(error) => errors.push(error),
            }
        }
        tokens.push(self.token(EOF, None));
        if errors.is_empty() {
            Ok(tokens)
        } else {
            Err(errors)
        }
    }

    fn advance(&mut self) -> Option<char> {
        let char = self.source.next()?;
        self.position = if char == '\n' {
            self.position.brk()
        } else {
            self.position.bump(1)
        };
        self.lexeme.push(char);
        Some(char)
    }

    fn peek(&mut self) -> Option<char> {
        self.source.peek().copied()
    }

    fn peek_second(&mut self) -> Option<char> {
        self.source.peek_nth(1).copied()
    }

    fn advance_if(&mut self, expected: char) -> bool {
        let matching = self.peek() == Some(expected);
        if matching {
            self.advance();
        }
        matching
    }

    fn advance_while(&mut self, predicate: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&predicate) {
            self.advance();
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            match (self.peek(), self.peek_second()) {
                (Some(c), _) if c.is_whitespace() => {
                    self.advance();
                }
                (Some('/'), Some('/')) => self.advance_while(|c| c != '\n'),
                _ => return,
            }
        }
    }

    fn token(&self, typ: TokenType, literal: Option<Literal>) -> Token {
        Token {
            typ,
            lexeme: self.lexeme.clone(),
            literal,
            span: Span::new(self.start, self.position),
        }
    }

    fn either(&mut self, second: char, matched: TokenType, unmatched: TokenType) -> TokenType {
        if self.advance_if(second) {
            matched
        } else {
            unmatched
        }
    }

    fn scan_token(&mut self) -> Option<Result<Token, LexingError>> {
        self.skip_trivia();
        self.start = self.position;
        self.lexeme.clear();

        let char = self.advance()?;
        let typ = match char {
            '(' => LEFTPAREN,
            ')' => RIGHTPAREN,
            '{' => LEFTBRACE,
            '}' => RIGHTBRACE,
            ',' => COMMA,
            '.' => DOT,
            '+' => PLUS,
            '-' => MINUS,
            ';' => SEMICOLON,
            '*' => STAR,
            '/' => SLASH,
            '!' => self.either('=', BANGEQUAL, BANG),
            '=' => self.either('=', EQUALEQUAL, EQUAL),
            '<' => self.either('=', LESSEQUAL, LESS),
            '>' => self.either('=', GREATEREQUAL, GREATER),
            '"' => return Some(self.string()),
            c if c.is_ascii_digit() => return Some(Ok(self.number())),
            c if is_identifier_start(c) => return Some(Ok(self.word())),
            character => {
                return Some(Err(LexingError::UnexpectedCharacter {
                    character,
                    position: self.start,
                }))
            }
        };
        Some(Ok(self.token(typ, None)))
    }

    fn string(&mut self) -> Result<Token, LexingError> {
        self.advance_while(|c| c != '"');
        if !self.advance_if('"') {
            return Err(LexingError::UnterminatedString { start: self.start });
        }
        let content = self.lexeme[1..self.lexeme.len() - 1].to_string();
        Ok(self.token(STRING, Some(Literal::Str(content))))
    }

    fn number(&mut self) -> Token {
        self.advance_while(|c| c.is_ascii_digit());
        if self.peek() == Some('.') && self.peek_second().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
            self.advance_while(|c| c.is_ascii_digit());
        }
        let value = self
            .lexeme
            .parse()
            .expect("a lexeme of digits with at most one inner dot is a valid float");
        self.token(NUMBER, Some(Literal::Number(value)))
    }

    fn word(&mut self) -> Token {
        self.advance_while(is_identifier_continuation);
        let typ = TokenType::keyword(&self.lexeme).unwrap_or(IDENTIFIER);
        self.token(typ, None)
    }
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_continuation(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
