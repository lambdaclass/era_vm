use crate::{state::VMState, Opcode};

pub trait Tracer {
    fn before_execution(&mut self, _opcode: &Opcode, _vm: &VMState) {}
}
