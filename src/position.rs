#[derive(Clone, Copy)]
pub struct Position {
    line : u64,
    col : u64
}

impl Position {
    pub fn bump(self, n : u64) -> Position {
        Position {
            line: self.line,
            col : self.col + n
        }
    }

    pub fn brk(self) -> Position {
        Position {
            line: self.line + 1,
            col : 0
        }
    }

    pub fn pretty_print(self) -> String {
        format!("{}:{}", self.line, self.col)
    }
}