use crate::{execution::Execution, state::VMState, Opcode};

pub trait Tracer {
    fn before_decoding(&mut self, _execution: &mut Execution, _state: &mut VMState) {}
    fn after_decoding(
        &mut self,
        _opcode: &Opcode,
        _execution: &mut Execution,
        _state: &mut VMState,
    ) {
    }
    fn before_execution(
        &mut self,
        _opcode: &Opcode,
        _execution: &mut Execution,
        _state: &mut VMState,
    ) {
    }
    fn after_execution(
        &mut self,
        _opcode: &Opcode,
        _execution: &mut Execution,
        _state: &mut VMState,
    ) {
    }
}
