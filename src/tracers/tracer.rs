use crate::{state::VMState, world_state::WorldState, Opcode};

pub trait Tracer {
    fn before_execution(&mut self, _opcode: &Opcode, _vm: &mut VMState, _world_state: &WorldState) {
    }
}
