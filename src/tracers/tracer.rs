use crate::{execution::Execution, state::VMState, Opcode};

pub trait Tracer {
    fn before_decoding(&mut self, execution: &mut Execution, state: &mut VMState);
    fn after_decoding(&mut self, opcode: &Opcode, execution: &mut Execution, state: &mut VMState);
    fn before_execution(&mut self, opcode: &Opcode, execution: &mut Execution, state: &mut VMState);
    fn after_execution(&mut self, opcode: &Opcode, execution: &mut Execution, state: &mut VMState);
}
