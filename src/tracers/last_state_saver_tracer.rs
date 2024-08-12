use crate::eravm_error::EraVmError;
use crate::{state::VMState, Opcode};

use super::tracer::Tracer;
use u256::H160;

use zkevm_opcode_defs::Opcode as Variant;
use zkevm_opcode_defs::RetOpcode;

#[derive(Debug)]
pub struct LastStateSaverTracer {
    pub vm_state: VMState,
}

impl LastStateSaverTracer {
    pub fn new() -> Self {
        Self {
            vm_state: VMState::new(
                vec![],
                vec![],
                H160::zero(),
                H160::zero(),
                0_u128,
                Default::default(),
                Default::default(),
                0,
                false,
            ),
        }
    }
}

impl Default for LastStateSaverTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracer for LastStateSaverTracer {
    fn before_execution(&mut self, opcode: &Opcode, vm: &mut VMState) -> Result<(), EraVmError> {
        if opcode.variant == Variant::Ret(RetOpcode::Ok) {
            self.vm_state = vm.clone();
        }
        Ok(())
    }
}
