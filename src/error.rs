use std::fmt::{self, Display, Formatter};

use crate::position::{Position, Span};

#[derive(Debug)]
pub enum LexingError {
    UnterminatedString { start: Position },
    UnexpectedCharacter { character: char, position: Position },
}

impl Display for LexingError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LexingError::UnterminatedString { start } => {
                write!(f, "Error at {start}: Unterminated string.")
            }
            LexingError::UnexpectedCharacter {
                character,
                position,
            } => write!(
                f,
                "Error at {position}: Unexpected character '{character}'."
            ),
        }
    }
}

#[derive(Debug)]
pub enum ParsingError {
    EarlyEof {
        pos: Position,
        expected: String,
    },
    UnclosedParenthesis {
        open_paren_start: Position,
    },
    IllegalToken {
        span: Span,
        found: String,
        expected: String,
    },
    InvalidAssignmentTarget {
        pos: Position,
    },
    TooManyItems {
        kind: &'static str,
        pos: Position,
    },
}

impl Display for ParsingError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ParsingError::EarlyEof { pos, expected } => {
                write!(
                    f,
                    "Error at {pos}: Unexpected end of file. Expect {expected}."
                )
            }
            ParsingError::UnclosedParenthesis { open_paren_start } => write!(
                f,
                "Error at {open_paren_start}: Parenthesis was never closed."
            ),
            ParsingError::IllegalToken {
                span,
                found,
                expected,
            } => write!(
                f,
                "Error at {}: Unexpected '{found}'. Expect {expected}.",
                span.start
            ),
            ParsingError::InvalidAssignmentTarget { pos } => {
                write!(f, "Error at {pos}: Invalid assignment target.")
            }
            ParsingError::TooManyItems { kind, pos } => {
                write!(f, "Error at {pos}: Can't have more than 255 {kind}.")
            }
        }
    }
}

#[derive(Debug)]
pub enum ResolutionErrorKind {
    ReadInOwnInitializer,
    AlreadyDeclared,
    TopLevelReturn,
    ValueReturnedFromInitializer,
    ThisOutsideClass,
    SuperOutsideClass,
    SuperWithoutSuperclass,
    SelfInheritance,
}

#[derive(Debug)]
pub struct ResolutionError {
    pub kind: ResolutionErrorKind,
    pub span: Span,
}

impl Display for ResolutionError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let message = match self.kind {
            ResolutionErrorKind::ReadInOwnInitializer => {
                "Can't read local variable in its own initializer."
            }
            ResolutionErrorKind::AlreadyDeclared => {
                "Already a variable with this name in this scope."
            }
            ResolutionErrorKind::TopLevelReturn => "Can't return from top-level code.",
            ResolutionErrorKind::ValueReturnedFromInitializer => {
                "Can't return a value from an initializer."
            }
            ResolutionErrorKind::ThisOutsideClass => "Can't use 'this' outside of a class.",
            ResolutionErrorKind::SuperOutsideClass => "Can't use 'super' outside of a class.",
            ResolutionErrorKind::SuperWithoutSuperclass => {
                "Can't use 'super' in a class with no superclass."
            }
            ResolutionErrorKind::SelfInheritance => "A class can't inherit from itself.",
        };
        write!(f, "Error at {}: {message}", self.span.start)
    }
}

#[derive(Debug)]
pub enum RuntimeErrorKind {
    OperandMustBeNumber,
    OperandsMustBeNumbers,
    OperandsMustBeNumbersOrStrings,
    UndefinedVariable(String),
    UndefinedProperty(String),
    NotCallable,
    ArityMismatch { expected: usize, got: usize },
    OnlyInstancesHaveProperties,
    OnlyInstancesHaveFields,
    SuperclassMustBeClass,
    OnlyInstancesHaveMethods,
    StackOverflow,
}

const MAX_TRACE_FRAMES: usize = 16;

#[derive(Debug)]
pub struct TraceFrame {
    pub function: Option<String>,
    pub position: Position,
}

impl Display for TraceFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.function {
            Some(name) => write!(f, "[{}] in {name}()", self.position),
            None => write!(f, "[{}] in script", self.position),
        }
    }
}

#[derive(Debug)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub span: Span,
    pub trace: Vec<TraceFrame>,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, span: Span) -> RuntimeError {
        RuntimeError {
            kind,
            span,
            trace: Vec::new(),
        }
    }

    pub fn with_trace(self, trace: Vec<TraceFrame>) -> RuntimeError {
        RuntimeError { trace, ..self }
    }

    fn write_message(&self, f: &mut Formatter) -> fmt::Result {
        match &self.kind {
            RuntimeErrorKind::OperandMustBeNumber => write!(f, "Operand must be a number."),
            RuntimeErrorKind::OperandsMustBeNumbers => write!(f, "Operands must be numbers."),
            RuntimeErrorKind::OperandsMustBeNumbersOrStrings => {
                write!(f, "Operands must be two numbers or two strings.")
            }
            RuntimeErrorKind::UndefinedVariable(name) => {
                write!(f, "Undefined variable '{name}'.")
            }
            RuntimeErrorKind::UndefinedProperty(name) => {
                write!(f, "Undefined property '{name}'.")
            }
            RuntimeErrorKind::NotCallable => write!(f, "Can only call functions and classes."),
            RuntimeErrorKind::ArityMismatch { expected, got } => {
                write!(f, "Expected {expected} arguments but got {got}.")
            }
            RuntimeErrorKind::OnlyInstancesHaveProperties => {
                write!(f, "Only instances have properties.")
            }
            RuntimeErrorKind::OnlyInstancesHaveFields => write!(f, "Only instances have fields."),
            RuntimeErrorKind::SuperclassMustBeClass => write!(f, "Superclass must be a class."),
            RuntimeErrorKind::OnlyInstancesHaveMethods => {
                write!(f, "Only instances have methods.")
            }
            RuntimeErrorKind::StackOverflow => write!(f, "Stack overflow."),
        }
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Runtime error at {}: ", self.span.start)?;
        self.write_message(f)?;
        if self.trace.len() <= MAX_TRACE_FRAMES {
            return self
                .trace
                .iter()
                .try_for_each(|frame| write!(f, "\n{frame}"));
        }
        let (innermost, rest) = self.trace.split_at(MAX_TRACE_FRAMES - 1);
        let (omitted, outermost) = rest.split_at(rest.len() - 1);
        innermost
            .iter()
            .try_for_each(|frame| write!(f, "\n{frame}"))?;
        write!(f, "\n... {} more frames", omitted.len())?;
        write!(f, "\n{}", outermost[0])
    }
}

#[derive(Debug)]
pub enum CompileErrorKind {
    TooManyConstants,
    TooManyLocals,
    TooManyUpvalues,
    JumpTooLarge,
    LoopTooLarge,
}

#[derive(Debug)]
pub struct CompileError {
    pub kind: CompileErrorKind,
    pub position: Position,
}

impl Display for CompileError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let message = match self.kind {
            CompileErrorKind::TooManyConstants => "Too many constants in one chunk.",
            CompileErrorKind::TooManyLocals => "Too many local variables in function.",
            CompileErrorKind::TooManyUpvalues => "Too many closure variables in function.",
            CompileErrorKind::JumpTooLarge => "Too much code to jump over.",
            CompileErrorKind::LoopTooLarge => "Loop body too large.",
        };
        write!(f, "Error at {}: {message}", self.position)
    }
}

#[derive(Debug)]
pub enum LoxError {
    Lexing(Vec<LexingError>),
    Parsing(Vec<ParsingError>),
    Resolution(Vec<ResolutionError>),
    Compile(Vec<CompileError>),
    Runtime(RuntimeError),
}

impl LoxError {
    pub fn exit_code(&self) -> u8 {
        match self {
            LoxError::Runtime(_) => 70,
            _ => 65,
        }
    }
}

fn write_lines<T: Display>(f: &mut Formatter, items: &[T]) -> fmt::Result {
    let lines: Vec<String> = items.iter().map(T::to_string).collect();
    write!(f, "{}", lines.join("\n"))
}

impl Display for LoxError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LoxError::Lexing(errors) => write_lines(f, errors),
            LoxError::Parsing(errors) => write_lines(f, errors),
            LoxError::Resolution(errors) => write_lines(f, errors),
            LoxError::Compile(errors) => write_lines(f, errors),
            LoxError::Runtime(error) => write!(f, "{error}"),
        }
    }
}
