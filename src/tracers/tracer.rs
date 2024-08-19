use crate::{eravm_error::EraVmError, execution::Execution, Opcode};

pub trait Tracer {
    fn before_execution(
        &mut self,
        _opcode: &Opcode,
        _vm: &mut Execution,
    ) -> Result<(), EraVmError> {
        Ok(())
    }
}
