use crate::{eravm_error::EraVmError, state::VMState, Opcode};

pub trait Tracer {
    fn before_execution(&mut self, _opcode: &Opcode, _vm: &mut VMState) -> Result<(), EraVmError> {
        Ok(())
    }
}
