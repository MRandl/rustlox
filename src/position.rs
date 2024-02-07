use std::fmt::Display;

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub line: u64,
    pub col: u64,
}

impl Position {
    pub fn default() -> Position {
        Position { line: 0, col: 0 }
    }

    pub fn bump(self, n: u64) -> Position {
        Position {
            line: self.line,
            col: self.col + n,
        }
    }

    pub fn brk(self) -> Position {
        Position {
            line: self.line + 1,
            col: 0,
        }
    }

    pub fn pretty_print(self) -> String {
        format!("{}:{}", self.line, self.col)
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{{ line : {}, col : {}}}", self.line, self.col)
    }
}
