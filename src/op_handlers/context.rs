use u256::U256;
use zkevm_opcode_defs::ethereum_types::Address;
use zkevm_opcode_defs::VmMetaParameters;

use crate::address_operands::{address_operands_read, address_operands_store};
use crate::eravm_error::{EraVmError, HeapError};
use crate::state::VMState;
use crate::value::TaggedValue;
use crate::{execution::Execution, opcode::Opcode};

// consider moving this function to a utils crate
// taken from matter-labs zk evm implementation
fn address_to_u256(address: &Address) -> U256 {
    let mut buffer = [0u8; 32];
    buffer[12..].copy_from_slice(&address.as_fixed_bytes()[..]);

    U256::from_big_endian(&buffer)
}

pub fn this(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let res =
        TaggedValue::new_raw_integer(address_to_u256(&vm.current_context()?.contract_address));
    address_operands_store(vm, opcode, res)
}

pub fn caller(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let res = TaggedValue::new_raw_integer(address_to_u256(&vm.current_context()?.caller));
    address_operands_store(vm, opcode, res)
}

pub fn code_address(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let res = TaggedValue::new_raw_integer(address_to_u256(&vm.current_context()?.code_address));
    address_operands_store(vm, opcode, res)
}

pub fn meta(vm: &mut Execution, opcode: &Opcode, state: &VMState) -> Result<(), EraVmError> {
    let res = TaggedValue::new_raw_integer(
        (VmMetaParameters {
            heap_size: vm
                .heaps
                .get(vm.current_context()?.heap_id)
                .ok_or(HeapError::ReadOutOfBounds)?
                .len() as u32,
            aux_heap_size: vm
                .heaps
                .get(vm.current_context()?.aux_heap_id)
                .ok_or(HeapError::ReadOutOfBounds)?
                .len() as u32,
            this_shard_id: 0,
            caller_shard_id: 0,
            code_shard_id: 0,
            aux_field_0: if vm.current_context()?.is_kernel() {
                state.pubdata() as u32
            } else {
                0
            },
        })
        .to_u256(),
    );
    address_operands_store(vm, opcode, res)
}

pub fn ergs_left(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let res = TaggedValue::new_raw_integer(U256::from(vm.current_frame()?.gas_left.0));
    address_operands_store(vm, opcode, res)
}

pub fn sp(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let sp = vm.current_frame()?.sp;
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(U256::from(sp)))
}

pub fn get_context_u128(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let res = TaggedValue::new_raw_integer(U256::from(vm.current_context()?.context_u128));
    address_operands_store(vm, opcode, res)
}

pub fn set_context_u128(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, _) = address_operands_read(vm, opcode)?;
    vm.register_context_u128 = src0.value.as_u128();
    Ok(())
}

pub fn increment_tx_number(
    vm: &mut Execution,
    _opcode: &Opcode,
    state: &mut VMState,
) -> Result<(), EraVmError> {
    vm.tx_number += 1;
    state.clear_transient_storage();
    Ok(())
}
