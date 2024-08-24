use zkevm_opcode_defs::{BlobSha256Format, ContractCodeSha256Format, VersionedHashLen32};

use crate::{
    address_operands::{address_operands_read, address_operands_store},
    eravm_error::{EraVmError, HeapError},
    execution::Execution,
    state::VMState,
    statistics::{VmStatistics, STORAGE_READ_STORAGE_APPLICATION_CYCLES},
    value::{FatPointer, TaggedValue},
    Opcode,
};

pub fn opcode_decommit(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
    statistics: &mut VmStatistics,
) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;

    let (code_hash, extra_cost) = (src0.value, src1.value.low_u32());

    let preimage_len_in_bytes = zkevm_opcode_defs::system_params::NEW_KERNEL_FRAME_MEMORY_STIPEND;

    let mut buffer = [0u8; 32];
    code_hash.to_big_endian(&mut buffer);

    // gas is payed in advance
    if vm.decrease_gas(extra_cost).is_err()
        || (!ContractCodeSha256Format::is_valid(&buffer) && !BlobSha256Format::is_valid(&buffer))
    {
        // we don't actually return an err here
        vm.set_register(1, TaggedValue::zero());
        return Ok(());
    }

    let (code, was_decommited) = state.decommit(code_hash);
    if was_decommited {
        // refund it
        vm.increase_gas(extra_cost)?;
    } else {
        statistics.storage_application_cycles += STORAGE_READ_STORAGE_APPLICATION_CYCLES;
    }

    let code = code.ok_or(EraVmError::DecommitFailed)?;

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
