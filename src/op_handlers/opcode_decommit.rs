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

    let code = state
        .decommit(code_hash)
        .ok_or(EraVmError::DecommitFailed)?;

    let code_len_in_bytes = code.len() * 32;
    let id = vm.heaps.allocate();

    let heap = vm.heaps.get_mut(id).ok_or(HeapError::StoreOutOfBounds)?;

    let mem_expansion_gas_cost = heap.expand_memory(code_len_in_bytes as u32);

    heap.store_multiple(0, code);

    vm.decrease_gas(mem_expansion_gas_cost + extra_cost)?;

    let pointer = FatPointer {
        offset: 0,
        page: id,
        start: 0,
        len: preimage_len_in_bytes,
    };

    address_operands_store(vm, opcode, TaggedValue::new_pointer(pointer.encode()))?;

    Ok(())
}
