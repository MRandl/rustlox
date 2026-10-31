use crate::position::Span;

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    LEFTPAREN,
    RIGHTPAREN,
    LEFTBRACE,
    RIGHTBRACE,
    COMMA,
    DOT,
    MINUS,
    PLUS,
    SEMICOLON,
    SLASH,
    STAR,

    BANG,
    BANGEQUAL,
    EQUAL,
    EQUALEQUAL,
    GREATER,
    GREATEREQUAL,
    LESS,
    LESSEQUAL,

    IDENTIFIER,
    STRING,
    NUMBER,

    AND,
    CLASS,
    ELSE,
    FALSE,
    FUN,
    FOR,
    IF,
    NIL,
    OR,
    PRINT,
    RETURN,
    SUPER,
    THIS,
    TRUE,
    VAR,
    WHILE,

    EOF,
}

impl TokenType {
    pub fn keyword(word: &str) -> Option<TokenType> {
        use TokenType::*;
        match word {
            "and" => Some(AND),
            "class" => Some(CLASS),
            "else" => Some(ELSE),
            "false" => Some(FALSE),
            "for" => Some(FOR),
            "fun" => Some(FUN),
            "if" => Some(IF),
            "nil" => Some(NIL),
            "or" => Some(OR),
            "print" => Some(PRINT),
            "return" => Some(RETURN),
            "super" => Some(SUPER),
            "this" => Some(THIS),
            "true" => Some(TRUE),
            "var" => Some(VAR),
            "while" => Some(WHILE),
            _ => None,
        }
    }

    pub fn starts_statement(self) -> bool {
        use TokenType::*;
        matches!(self, CLASS | FUN | VAR | FOR | IF | WHILE | PRINT | RETURN)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Number(f64),
    Str(String),
}

#[derive(Clone, Debug)]
pub struct Token {
    pub typ: TokenType,
    pub lexeme: String,
    pub literal: Option<Literal>,
    pub span: Span,
}

impl Token {
    pub fn is(&self, typ: TokenType) -> bool {
        self.typ == typ
    }
}
