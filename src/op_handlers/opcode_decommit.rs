use crate::{address_operands::{self, address_operands_read}, eravm_error::{EraVmError, HeapError}, state::VMState, store::Storage, value::{FatPointer, TaggedValue}, Opcode};

pub fn opcode_decommit(vm: &mut VMState, opcode: &Opcode, storage: &mut dyn Storage,) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;

    let (code_hash, extra_cost) = (src0.value, src1.value.low_u32());

    let preimage_len_in_bytes =
        zkevm_opcode_defs::system_params::NEW_KERNEL_FRAME_MEMORY_STIPEND;

    if vm.decrease_gas(extra_cost).is_err() {
        vm.set_register(opcode.dst0_index, TaggedValue::zero());
    }

    let code = storage.decommit(code_hash)?.ok_or(EraVmError::DecommitFailed)?;

    let id = vm.heaps.allocate();
    let heap = vm.heaps.get_mut(id).ok_or(HeapError::StoreOutOfBounds)?;
    let mut address = 0;
    for value in code.iter() {
        heap.store(address,*value);
        address += 32;
    }


    let pointer = FatPointer {
        offset: 0,
        page: id,
        start: 0,
        len: preimage_len_in_bytes,
    };

    vm.set_register(opcode.dst0_index, TaggedValue::new_pointer(pointer.encode()));

    Ok(())
}
