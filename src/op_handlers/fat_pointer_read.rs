use u256::U256;

use crate::address_operands::address_operands_read;
use crate::eravm_error::{EraVmError, HeapError, OperandError};
use crate::value::{FatPointer, TaggedValue};
use crate::{opcode::Opcode, state::VMState};

pub fn fat_pointer_read(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, _) = address_operands_read(vm, opcode)?;
    if !src0.is_pointer {
        return Err(OperandError::InvalidSrcNotPointer(opcode.variant).into());
    }

    // make sure the pointer is a value that can be added without overflow for 32 bits.
    if src0.value.low_u32() > (u32::MAX - 32) {
        vm.set_gas_left(0)?;
        return Err(HeapError::InvalidAddress.into());
    }

    let pointer = FatPointer::decode(src0.value);

    let value = if pointer.offset < pointer.len {
        let heap = vm
            .heaps
            .get_mut(pointer.page)
            .ok_or(HeapError::ReadOutOfBounds)?;

        let gas_cost = heap.expand_memory(pointer.start + pointer.offset + 32);
        let value = heap.read_from_pointer(&pointer);

        vm.decrease_gas(gas_cost)?;

        value
    } else {
        U256::zero()
    };

    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));

    if opcode.alters_vm_flags {
        // This flag is set if .inc is present
        let new_pointer = FatPointer {
            offset: pointer.offset + 32,
            ..pointer
        };

        vm.set_register(
            opcode.dst1_index,
            TaggedValue::new_pointer(new_pointer.encode()),
        );
    };
    Ok(())
}
