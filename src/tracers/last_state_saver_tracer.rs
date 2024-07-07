use crate::{state::VMState, store::Storage, Opcode};

use super::tracer::Tracer;
use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::RetOpcode;

#[derive(Debug)]
pub struct LastStateSaverTracer {
    pub vm_state: VMState,
}

impl LastStateSaverTracer {
    pub fn new() -> Self {
        Self {
            vm_state: VMState::new(),
        }
    }
}

impl Default for LastStateSaverTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracer for LastStateSaverTracer {
    fn before_execution(&mut self, opcode: &Opcode, vm: &mut VMState, _storage: &dyn Storage) {
        if opcode.variant == Variant::Ret(RetOpcode::Ok) {
            self.vm_state = vm.clone();
        }
    }
}
