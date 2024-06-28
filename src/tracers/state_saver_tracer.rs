use crate::{state::VMState, Opcode};

use super::tracer::Tracer;

pub struct StateSaverTracer {
    pub state: Vec<VMState>,
}

impl Tracer for StateSaverTracer {
    fn before_execution(&mut self, _opcode: &Opcode, vm: &VMState) {
        self.state.push(vm.clone());
    }
}
