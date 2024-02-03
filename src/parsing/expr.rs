
enum UnaryOp {
    UnaMinus, Not
}

enum BinaryOp {
    Equalequal, NotEqual, Le, Leq, Ge, Geq, Plus, BinMinus, Times, Div
}

enum Expr {
    Num (u64),
    Str (String),
    T, F, Nil,

    Unary { e1 : Box<Expr>, op : UnaryOp},
    Binary { e1 : Box<Expr>, op : BinaryOp, e2 : Box<Expr> },
}