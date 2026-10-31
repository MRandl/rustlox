use crate::{
    bytecode::vm::Vm,
    error::LoxError,
    interpreting::interpret::Interpreter,
    lexing::lexer::Lexer,
    parsing::{parser::Parser, stmt::Stmt},
    resolving::resolver::{self, Locals},
};

pub trait Backend {
    fn execute(&mut self, statements: &[Stmt], locals: Locals) -> Result<(), LoxError>;
}

impl Backend for Interpreter {
    fn execute(&mut self, statements: &[Stmt], locals: Locals) -> Result<(), LoxError> {
        self.interpret(statements, locals)
            .map_err(LoxError::Runtime)
    }
}

impl Backend for Vm {
    fn execute(&mut self, statements: &[Stmt], _locals: Locals) -> Result<(), LoxError> {
        self.interpret(statements)
    }
}

pub struct Lox {
    backend: Box<dyn Backend>,
}

impl Lox {
    pub fn new(backend: Box<dyn Backend>) -> Lox {
        Lox { backend }
    }

    pub fn run(&mut self, source: &str) -> Result<(), LoxError> {
        let tokens = Lexer::new(source).tokenize().map_err(LoxError::Lexing)?;
        let statements = Parser::new(tokens).parse().map_err(LoxError::Parsing)?;
        let locals = resolver::resolve(&statements).map_err(LoxError::Resolution)?;
        self.backend.execute(&statements, locals)
    }
}
