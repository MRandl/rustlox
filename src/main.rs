use std::{
    env, fs,
    io::{self, BufRead, Write},
    process::ExitCode,
};

use crate::{
    bytecode::vm::Vm,
    interpreting::interpret::Interpreter,
    lox::{Backend, Lox},
};

mod bytecode;
mod clock;
mod error;
mod interpreting;
mod lexing;
mod lox;
mod parsing;
mod position;
mod resolving;

const EXIT_USAGE: u8 = 64;
const EXIT_NO_INPUT: u8 = 66;
const EXIT_IO: u8 = 74;
const USAGE: &str = "Usage: rustlox [--vm] [--disassemble] [script]";

#[derive(Default)]
struct Options {
    vm: bool,
    disassemble: bool,
    script: Option<String>,
}

impl Options {
    fn parse(args: impl Iterator<Item = String>) -> Option<Options> {
        let mut options = Options::default();
        for arg in args {
            match arg.as_str() {
                "--vm" => options.vm = true,
                "--disassemble" => {
                    options.vm = true;
                    options.disassemble = true;
                }
                flag if flag.starts_with("--") => return None,
                _ if options.script.is_some() => return None,
                _ => options.script = Some(arg),
            }
        }
        Some(options)
    }

    fn backend(&self) -> Box<dyn Backend> {
        if self.vm {
            Box::new(Vm::new(self.disassemble))
        } else {
            Box::<Interpreter>::default()
        }
    }
}

fn main() -> ExitCode {
    let Some(options) = Options::parse(env::args().skip(1)) else {
        eprintln!("{USAGE}");
        return ExitCode::from(EXIT_USAGE);
    };
    let lox = Lox::new(options.backend());
    match &options.script {
        Some(path) => run_file(lox, path),
        None => run_prompt(lox),
    }
}

fn run_file(mut lox: Lox, path: &str) -> ExitCode {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Could not read '{path}': {error}");
            return ExitCode::from(EXIT_NO_INPUT);
        }
    };
    match lox.run(&source) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run_prompt(mut lox: Lox) -> ExitCode {
    let mut lines = io::stdin().lock().lines();
    loop {
        print!("> ");
        if io::stdout().flush().is_err() {
            return ExitCode::from(EXIT_IO);
        }
        match lines.next() {
            None => return ExitCode::SUCCESS,
            Some(Err(error)) => {
                eprintln!("{error}");
                return ExitCode::from(EXIT_IO);
            }
            Some(Ok(line)) => {
                if let Err(error) = lox.run(&line) {
                    eprintln!("{error}");
                }
            }
        }
    }
}
