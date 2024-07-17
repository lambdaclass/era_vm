use crate::{eravm_error::EraVmError, state::VMState, Opcode};

pub trait Tracer {
    fn before_execution(&mut self, _opcode: &Opcode, _vm: &VMState) -> Result<(), EraVmError> {
        Ok(())
    }
}
