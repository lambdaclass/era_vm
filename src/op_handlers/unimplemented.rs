use crate::eravm_error::{EraVmError, OpcodeError};
use crate::{opcode::Opcode, state::VMState};

pub fn unimplemented(_vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    eprintln!("Unimplemented instruction: {:?}!", opcode.variant);
    Err(OpcodeError::UnimplementedOpcode.into())
}
