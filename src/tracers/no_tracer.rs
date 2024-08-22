use super::tracer::Tracer;
use crate::state::VMState;
use crate::{execution::Execution, Opcode};

#[derive(Default)]
pub struct NoTracer {}

impl Tracer for NoTracer {
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
