use crate::{eravm_error::EraVmError, state::VMState, Opcode};

use super::tracer::Tracer;

#[derive(Default)]
pub struct StateSaverTracer {
    pub state: Vec<VMState>,
}

impl Tracer for StateSaverTracer {
    fn before_execution(&mut self, _opcode: &Opcode, vm: &VMState) -> Result<(), EraVmError> {
        self.state.push(vm.clone());
        Ok(())
    }
}
