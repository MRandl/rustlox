use std::rc::Rc;

use crate::{
    bytecode::{
        chunk::{Chunk, Op},
        heap::{Heap, ObjRef},
        object::{Function, Object, UpvalueDescriptor},
        value::Value,
    },
    error::{CompileError, CompileErrorKind},
    parsing::{
        expr::{BinaryOp, Expr, Identifier, LogicalOp, PositionedExpr, UnaryOp},
        stmt::{ClassDecl, FunctionDecl, Stmt},
    },
    position::Span,
};

const MAX_LOCALS: usize = u8::MAX as usize + 1;
const MAX_UPVALUES: usize = u8::MAX as usize + 1;
const INITIALIZER: &str = "init";

pub fn compile(statements: &[Stmt], heap: &mut Heap) -> Result<ObjRef, Vec<CompileError>> {
    let mut compiler = Compiler {
        heap,
        functions: vec![FunctionState::new(FunctionKind::Script, None, 0)],
        errors: Vec::new(),
    };
    for statement in statements {
        compiler.statement(statement);
    }
    let end = compiler.state().last_span();
    let script = compiler.finish_function(end);
    if compiler.errors.is_empty() {
        Ok(compiler.heap.insert(Object::Function(script)))
    } else {
        Err(compiler.errors)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionKind {
    Script,
    Function,
    Method,
    Initializer,
}

struct Local {
    name: String,
    depth: usize,
    is_captured: bool,
}

struct FunctionState {
    kind: FunctionKind,
    name: Option<ObjRef>,
    arity: u8,
    chunk: Chunk,
    locals: Vec<Local>,
    upvalues: Vec<UpvalueDescriptor>,
    scope_depth: usize,
}

impl FunctionState {
    fn new(kind: FunctionKind, name: Option<ObjRef>, arity: u8) -> FunctionState {
        let receiver = match kind {
            FunctionKind::Method | FunctionKind::Initializer => "this",
            FunctionKind::Script | FunctionKind::Function => "",
        };
        FunctionState {
            kind,
            name,
            arity,
            chunk: Chunk::default(),
            locals: vec![Local {
                name: receiver.to_string(),
                depth: 0,
                is_captured: false,
            }],
            upvalues: Vec::new(),
            scope_depth: 0,
        }
    }

    fn resolve_local(&self, name: &str) -> Option<u8> {
        self.locals
            .iter()
            .rposition(|local| local.name == name)
            .map(|slot| slot as u8)
    }

    fn last_span(&self) -> Span {
        self.chunk.spans.last().copied().unwrap_or_default()
    }
}

enum Access {
    Local(u8),
    Upvalue(u8),
    Global(u8),
}

struct Compiler<'h> {
    heap: &'h mut Heap,
    functions: Vec<FunctionState>,
    errors: Vec<CompileError>,
}

impl Compiler<'_> {
    fn state(&mut self) -> &mut FunctionState {
        self.functions
            .last_mut()
            .expect("the script function is always being compiled")
    }

    fn report(&mut self, kind: CompileErrorKind, span: Span) {
        self.errors.push(CompileError {
            kind,
            position: span.start,
        });
    }

    fn emit(&mut self, op: Op, span: Span) -> usize {
        self.state().chunk.write(op, span)
    }

    fn make_constant(&mut self, value: Value, span: Span) -> u8 {
        self.state().chunk.add_constant(value).unwrap_or_else(|| {
            self.report(CompileErrorKind::TooManyConstants, span);
            0
        })
    }

    fn identifier_constant(&mut self, name: &str, span: Span) -> u8 {
        let interned = self.heap.intern(name);
        self.make_constant(Value::Obj(interned), span)
    }

    fn emit_constant(&mut self, value: Value, span: Span) {
        let index = self.make_constant(value, span);
        self.emit(Op::Constant(index), span);
    }

    fn emit_jump(&mut self, jump: fn(u16) -> Op, span: Span) -> usize {
        self.emit(jump(u16::MAX), span)
    }

    fn patch_jump(&mut self, at: usize, span: Span) {
        let code = &mut self.state().chunk.code;
        match u16::try_from(code.len() - at - 1) {
            Ok(offset) => code[at] = code[at].with_jump_offset(offset),
            Err(_) => self.report(CompileErrorKind::JumpTooLarge, span),
        }
    }

    fn emit_loop(&mut self, loop_start: usize, span: Span) {
        let distance = self.state().chunk.code.len() - loop_start + 1;
        match u16::try_from(distance) {
            Ok(offset) => {
                self.emit(Op::Loop(offset), span);
            }
            Err(_) => self.report(CompileErrorKind::LoopTooLarge, span),
        }
    }

    fn emit_implicit_return(&mut self, span: Span) {
        if self.state().kind == FunctionKind::Initializer {
            self.emit(Op::GetLocal(0), span);
        } else {
            self.emit(Op::Nil, span);
        }
        self.emit(Op::Return, span);
    }

    fn begin_scope(&mut self) {
        self.state().scope_depth += 1;
    }

    fn end_scope(&mut self, span: Span) {
        let state = self.state();
        state.scope_depth -= 1;
        let depth = state.scope_depth;
        let mut discarded = Vec::new();
        while state.locals.last().is_some_and(|local| local.depth > depth) {
            let local = state.locals.pop().expect("the loop condition saw a local");
            discarded.push(if local.is_captured {
                Op::CloseUpvalue
            } else {
                Op::Pop
            });
        }
        for op in discarded {
            self.emit(op, span);
        }
    }

    fn add_local(&mut self, name: &Identifier) {
        if self.state().locals.len() == MAX_LOCALS {
            self.report(CompileErrorKind::TooManyLocals, name.span);
            return;
        }
        let state = self.state();
        let depth = state.scope_depth;
        state.locals.push(Local {
            name: name.name.clone(),
            depth,
            is_captured: false,
        });
    }

    fn is_local_scope(&mut self) -> bool {
        self.state().scope_depth > 0
    }

    fn define_variable(&mut self, name: &Identifier) {
        if self.is_local_scope() {
            self.add_local(name);
        } else {
            let global = self.identifier_constant(&name.name, name.span);
            self.emit(Op::DefineGlobal(global), name.span);
        }
    }

    fn resolve(&mut self, name: &str, span: Span) -> Access {
        let level = self.functions.len() - 1;
        if let Some(slot) = self.functions[level].resolve_local(name) {
            return Access::Local(slot);
        }
        if let Some(index) = self.resolve_upvalue(level, name, span) {
            return Access::Upvalue(index);
        }
        Access::Global(self.identifier_constant(name, span))
    }

    fn resolve_upvalue(&mut self, level: usize, name: &str, span: Span) -> Option<u8> {
        let enclosing = level.checked_sub(1)?;
        if let Some(slot) = self.functions[enclosing].resolve_local(name) {
            self.functions[enclosing].locals[slot as usize].is_captured = true;
            return Some(self.add_upvalue(level, true, slot, span));
        }
        let index = self.resolve_upvalue(enclosing, name, span)?;
        Some(self.add_upvalue(level, false, index, span))
    }

    fn add_upvalue(&mut self, level: usize, is_local: bool, index: u8, span: Span) -> u8 {
        let upvalues = &mut self.functions[level].upvalues;
        if let Some(existing) = upvalues
            .iter()
            .position(|upvalue| upvalue.is_local == is_local && upvalue.index == index)
        {
            return existing as u8;
        }
        if upvalues.len() == MAX_UPVALUES {
            self.report(CompileErrorKind::TooManyUpvalues, span);
            return 0;
        }
        upvalues.push(UpvalueDescriptor { is_local, index });
        (upvalues.len() - 1) as u8
    }

    fn get_variable(&mut self, name: &str, span: Span) {
        let op = match self.resolve(name, span) {
            Access::Local(slot) => Op::GetLocal(slot),
            Access::Upvalue(index) => Op::GetUpvalue(index),
            Access::Global(constant) => Op::GetGlobal(constant),
        };
        self.emit(op, span);
    }

    fn set_variable(&mut self, name: &str, span: Span) {
        let op = match self.resolve(name, span) {
            Access::Local(slot) => Op::SetLocal(slot),
            Access::Upvalue(index) => Op::SetUpvalue(index),
            Access::Global(constant) => Op::SetGlobal(constant),
        };
        self.emit(op, span);
    }

    fn statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expression(expr) => {
                self.expression(expr);
                self.emit(Op::Pop, expr.span);
            }
            Stmt::Print(expr) => {
                self.expression(expr);
                self.emit(Op::Print, expr.span);
            }
            Stmt::Var { name, initializer } => {
                match initializer {
                    Some(initializer) => self.expression(initializer),
                    None => {
                        self.emit(Op::Nil, name.span);
                    }
                }
                self.define_variable(name);
            }
            Stmt::Block(statements) => {
                self.begin_scope();
                for statement in statements {
                    self.statement(statement);
                }
                let end = self.state().last_span();
                self.end_scope(end);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => self.if_statement(condition, then_branch, else_branch.as_deref()),
            Stmt::While { condition, body } => self.while_statement(condition, body),
            Stmt::Function(declaration) => self.function_declaration(declaration),
            Stmt::Return { keyword, value } => self.return_statement(*keyword, value.as_ref()),
            Stmt::Class(declaration) => self.class_declaration(declaration),
        }
    }

    fn if_statement(
        &mut self,
        condition: &PositionedExpr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) {
        let span = condition.span;
        self.expression(condition);
        let then_jump = self.emit_jump(Op::JumpIfFalse, span);
        self.emit(Op::Pop, span);
        self.statement(then_branch);
        let else_jump = self.emit_jump(Op::Jump, span);
        self.patch_jump(then_jump, span);
        self.emit(Op::Pop, span);
        if let Some(else_branch) = else_branch {
            self.statement(else_branch);
        }
        self.patch_jump(else_jump, span);
    }

    fn while_statement(&mut self, condition: &PositionedExpr, body: &Stmt) {
        let span = condition.span;
        let loop_start = self.state().chunk.code.len();
        self.expression(condition);
        let exit_jump = self.emit_jump(Op::JumpIfFalse, span);
        self.emit(Op::Pop, span);
        self.statement(body);
        self.emit_loop(loop_start, span);
        self.patch_jump(exit_jump, span);
        self.emit(Op::Pop, span);
    }

    fn return_statement(&mut self, keyword: Span, value: Option<&PositionedExpr>) {
        match value {
            Some(value) => {
                self.expression(value);
                self.emit(Op::Return, keyword);
            }
            None => self.emit_implicit_return(keyword),
        }
    }

    fn function_declaration(&mut self, declaration: &FunctionDecl) {
        if self.is_local_scope() {
            self.add_local(&declaration.name);
            self.function(declaration, FunctionKind::Function);
        } else {
            self.function(declaration, FunctionKind::Function);
            self.define_variable(&declaration.name);
        }
    }

    fn function(&mut self, declaration: &FunctionDecl, kind: FunctionKind) {
        let span = declaration.name.span;
        let name = self.heap.intern(&declaration.name.name);
        let arity = declaration.params.len() as u8;
        self.functions
            .push(FunctionState::new(kind, Some(name), arity));
        self.begin_scope();
        for param in &declaration.params {
            self.add_local(param);
        }
        for statement in &declaration.body {
            self.statement(statement);
        }
        let function = self.finish_function(span);
        let function = self.heap.insert(Object::Function(function));
        let index = self.make_constant(Value::Obj(function), span);
        self.emit(Op::Closure(index), span);
    }

    fn finish_function(&mut self, span: Span) -> Function {
        self.emit_implicit_return(span);
        let state = self
            .functions
            .pop()
            .expect("every compiled function pushed its state");
        Function {
            name: state.name,
            arity: state.arity,
            chunk: Rc::new(state.chunk),
            upvalues: state.upvalues,
        }
    }

    fn class_declaration(&mut self, declaration: &ClassDecl) {
        let name = &declaration.name;
        let name_constant = self.identifier_constant(&name.name, name.span);
        self.emit(Op::Class(name_constant), name.span);
        self.define_variable(name);

        if let Some(superclass) = &declaration.superclass {
            let span = superclass.identifier.span;
            self.get_variable(&superclass.identifier.name, span);
            self.begin_scope();
            self.add_local(&Identifier {
                name: "super".to_string(),
                span,
            });
            self.get_variable(&name.name, name.span);
            self.emit(Op::Inherit, span);
        }

        self.get_variable(&name.name, name.span);
        for method in &declaration.methods {
            let kind = match method.name.name.as_str() {
                INITIALIZER => FunctionKind::Initializer,
                _ => FunctionKind::Method,
            };
            let method_name = self.identifier_constant(&method.name.name, method.name.span);
            self.function(method, kind);
            self.emit(Op::Method(method_name), method.name.span);
        }
        self.emit(Op::Pop, name.span);

        if declaration.superclass.is_some() {
            self.end_scope(name.span);
        }
    }

    fn expression(&mut self, pexpr: &PositionedExpr) {
        let span = pexpr.span;
        match &pexpr.expr {
            Expr::Num(n) => self.emit_constant(Value::Number(*n), span),
            Expr::Str(s) => {
                let interned = self.heap.intern(s);
                self.emit_constant(Value::Obj(interned), span);
            }
            Expr::T => {
                self.emit(Op::True, span);
            }
            Expr::F => {
                self.emit(Op::False, span);
            }
            Expr::Nil => {
                self.emit(Op::Nil, span);
            }
            Expr::Grouping(inner) => self.expression(inner),
            Expr::Unary { op, e1 } => {
                self.expression(e1);
                let op = match op {
                    UnaryOp::UnaMinus => Op::Negate,
                    UnaryOp::Not => Op::Not,
                };
                self.emit(op, span);
            }
            Expr::Binary { e1, op, e2 } => {
                self.expression(e1);
                self.expression(e2);
                self.binary_operator(*op, span);
            }
            Expr::Logical { e1, op, e2 } => self.logical(e1, *op, e2),
            Expr::Variable(variable) | Expr::This(variable) => {
                self.get_variable(&variable.identifier.name, variable.identifier.span)
            }
            Expr::Assign { target, value } => {
                self.expression(value);
                self.set_variable(&target.identifier.name, target.identifier.span);
            }
            Expr::Call { callee, args } => self.call(callee, args, span),
            Expr::Get { object, name } => {
                self.expression(object);
                let name_constant = self.identifier_constant(&name.name, name.span);
                self.emit(Op::GetProperty(name_constant), name.span);
            }
            Expr::Set {
                object,
                name,
                value,
            } => {
                self.expression(object);
                self.expression(value);
                let name_constant = self.identifier_constant(&name.name, name.span);
                self.emit(Op::SetProperty(name_constant), name.span);
            }
            Expr::Super { keyword, method } => {
                let name_constant = self.identifier_constant(&method.name, method.span);
                self.get_variable("this", keyword.identifier.span);
                self.get_variable("super", keyword.identifier.span);
                self.emit(Op::GetSuper(name_constant), span);
            }
        }
    }

    fn binary_operator(&mut self, op: BinaryOp, span: Span) {
        let ops: &[Op] = match op {
            BinaryOp::Equalequal => &[Op::Equal],
            BinaryOp::NotEqual => &[Op::Equal, Op::Not],
            BinaryOp::Ge => &[Op::Greater],
            BinaryOp::Geq => &[Op::Less, Op::Not],
            BinaryOp::Le => &[Op::Less],
            BinaryOp::Leq => &[Op::Greater, Op::Not],
            BinaryOp::Plus => &[Op::Add],
            BinaryOp::BinMinus => &[Op::Subtract],
            BinaryOp::Times => &[Op::Multiply],
            BinaryOp::Div => &[Op::Divide],
        };
        for &op in ops {
            self.emit(op, span);
        }
    }

    fn logical(&mut self, left: &PositionedExpr, op: LogicalOp, right: &PositionedExpr) {
        let span = left.span;
        self.expression(left);
        match op {
            LogicalOp::And => {
                let end_jump = self.emit_jump(Op::JumpIfFalse, span);
                self.emit(Op::Pop, span);
                self.expression(right);
                self.patch_jump(end_jump, span);
            }
            LogicalOp::Or => {
                let else_jump = self.emit_jump(Op::JumpIfFalse, span);
                let end_jump = self.emit_jump(Op::Jump, span);
                self.patch_jump(else_jump, span);
                self.emit(Op::Pop, span);
                self.expression(right);
                self.patch_jump(end_jump, span);
            }
        }
    }

    fn arguments(&mut self, args: &[PositionedExpr]) -> u8 {
        for arg in args {
            self.expression(arg);
        }
        args.len() as u8
    }

    fn call(&mut self, callee: &PositionedExpr, args: &[PositionedExpr], span: Span) {
        match &callee.expr {
            Expr::Get { object, name } => {
                self.expression(object);
                let name_constant = self.identifier_constant(&name.name, name.span);
                let argc = self.arguments(args);
                self.emit(Op::Invoke(name_constant, argc), span);
            }
            Expr::Super { keyword, method } => {
                let name_constant = self.identifier_constant(&method.name, method.span);
                self.get_variable("this", keyword.identifier.span);
                let argc = self.arguments(args);
                self.get_variable("super", keyword.identifier.span);
                self.emit(Op::SuperInvoke(name_constant, argc), span);
            }
            _ => {
                self.expression(callee);
                let argc = self.arguments(args);
                self.emit(Op::Call(argc), span);
            }
        }
    }
}
