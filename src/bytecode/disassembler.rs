use std::fmt::Write;

use crate::bytecode::{
    chunk::{Chunk, Op},
    heap::{Heap, ObjRef},
    object::Object,
};

pub fn disassemble(heap: &Heap, script: ObjRef) -> String {
    let mut out = String::new();
    let mut pending = vec![script];
    while let Some(function) = pending.pop() {
        let function = heap.function(function);
        let name = function.name.map_or_else(
            || "<script>".to_string(),
            |name| heap.string(name).to_string(),
        );
        writeln!(out, "== {name} ==").expect("writing to a String never fails");
        disassemble_chunk(heap, &function.chunk, &mut out);
        pending.extend(
            function
                .chunk
                .constants
                .iter()
                .filter_map(|constant| constant.as_obj())
                .filter(|&obj| matches!(heap.get(obj), Object::Function(_))),
        );
    }
    out
}

fn disassemble_chunk(heap: &Heap, chunk: &Chunk, out: &mut String) {
    for (offset, op) in chunk.code.iter().enumerate() {
        let position = chunk.spans[offset].start.to_string();
        let mut line = format!("{offset:04} {position:>9} {op:?}");
        if let Some(index) = op.constant_operand() {
            let constant = chunk.constants[index as usize];
            write!(line, " '{}'", heap.display(constant)).expect("writing to a String never fails");
        }
        match *op {
            Op::Jump(jump) | Op::JumpIfFalse(jump) => {
                write!(line, " -> {:04}", offset + 1 + jump as usize)
            }
            Op::Loop(jump) => write!(line, " -> {:04}", offset + 1 - jump as usize),
            _ => Ok(()),
        }
        .expect("writing to a String never fails");
        out.push_str(&line);
        out.push('\n');
    }
}
