use crate::{
    address_operands::{address_operands_read, address_operands_store},
    eravm_error::{EraVmError, HeapError},
    execution::Execution,
    state::VMState,
    value::{FatPointer, TaggedValue},
    Opcode,
};

pub fn opcode_decommit(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;

    let (code_hash, extra_cost) = (src0.value, src1.value.low_u32());

    let preimage_len_in_bytes = zkevm_opcode_defs::system_params::NEW_KERNEL_FRAME_MEMORY_STIPEND;

    vm.decrease_gas(extra_cost)?;

    let code = state
        .decommit(code_hash)
        .ok_or(EraVmError::DecommitFailed)?;

    let code_len_in_bytes = code.len() * 32;
    let id = vm.heaps.allocate();
    let mem_expansion_gas_cost = vm
        .heaps
        .get_mut(id)
        .ok_or(HeapError::StoreOutOfBounds)?
        .expand_memory(code_len_in_bytes as u32);

    vm.decrease_gas(mem_expansion_gas_cost)?;

    let heap = vm.heaps.get_mut(id).ok_or(HeapError::StoreOutOfBounds)?;

    let mut address = 0;
    for value in code.iter() {
        heap.store(address, *value);
        address += 32;
    }

    let pointer = FatPointer {
        offset: 0,
        page: id,
        start: 0,
        len: preimage_len_in_bytes,
    };

    address_operands_store(vm, opcode, TaggedValue::new_pointer(pointer.encode()))?;

    Ok(())
}
