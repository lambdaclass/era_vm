use u256::U256;
use zkevm_opcode_defs::MAX_OFFSET_TO_DEREF_LOW_U32;

use crate::address_operands::address_operands_read;
use crate::eravm_error::{EraVmError, HeapError, OperandError};
use crate::value::TaggedValue;
use crate::{opcode::Opcode, execution::Execution};

pub fn aux_heap_write(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;
    if src0.is_pointer {
        return Err(OperandError::InvalidSrcPointer(opcode.variant).into());
    }

    if src0.value > U256::from(MAX_OFFSET_TO_DEREF_LOW_U32) {
        return Err(OperandError::InvalidSrcAddress(opcode.variant).into());
    }
    let addr = src0.value.low_u32();

    let gas_cost = vm
        .heaps
        .get_mut(vm.current_context()?.aux_heap_id)
        .ok_or(HeapError::ReadOutOfBounds)?
        .expand_memory(addr + 32);

    vm.decrease_gas(gas_cost)?;

    vm.heaps
        .get_mut(vm.current_context()?.aux_heap_id)
        .ok_or(HeapError::ReadOutOfBounds)?
        .store(addr, src1.value);

    if opcode.flag0_set {
        // This flag is set if .inc is present
        vm.set_register(
            opcode.dst0_index,
            TaggedValue::new_raw_integer(U256::from(addr + 32)),
        );
    }
    Ok(())
}
