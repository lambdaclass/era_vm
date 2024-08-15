use crate::eravm_error::{EraVmError, OpcodeError};
use crate::{opcode::Opcode, execution::Execution};

pub fn unimplemented(_vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    eprintln!("Unimplemented instruction: {:?}!", opcode.variant);
    Err(OpcodeError::UnimplementedOpcode.into())
}
