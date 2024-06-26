use u256::U256;
use zkevm_opcode_defs::ethereum_types::Address;
use zkevm_opcode_defs::VmMetaParameters;

use crate::address_operands::{address_operands_read, address_operands_store};
use crate::value::TaggedValue;
use crate::{opcode::Opcode, state::VMState};

// consider moving this function to a utils crate
// taken from matter-labs zk evm implementation
fn address_to_u256(address: &Address) -> U256 {
    let mut buffer = [0u8; 32];
    buffer[12..].copy_from_slice(&address.as_fixed_bytes()[..]);

    U256::from_big_endian(&buffer)
}

pub fn _this(vm: &mut VMState, opcode: &Opcode) {
    let res = TaggedValue::new_raw_integer(address_to_u256(&vm.current_frame.this_address));
    address_operands_store(vm, opcode, res);
    return;
}

pub fn _caller(vm: &mut VMState, opcode: &Opcode) {
    let res = TaggedValue::new_raw_integer(address_to_u256(&vm.current_frame.msg_sender));
    address_operands_store(vm, opcode, res);
    return;
}

pub fn _code_address(vm: &mut VMState, opcode: &Opcode) {
    let res = TaggedValue::new_raw_integer(address_to_u256(&vm.current_frame.code_address));
    address_operands_store(vm, opcode, res);
    return;
}

pub fn _meta(vm: &mut VMState, opcode: &Opcode) {
    let meta_parameters = TaggedValue::new_raw_integer(
        (VmMetaParameters {
            aux_field_0: 0, // TODO: this value is only 0 when not in kernel mode
            heap_size: vm.current_frame.heap.len() as u32,
            aux_heap_size: 0, // TODO: aux heap still not implemented, done here https://github.com/lambdaclass/era_vm/pull/35/
            this_shard_id: 0, // vm.current_frame.this_shard_id
            caller_shard_id: 0, // vm.current_frame.caller_shard_id
            code_shard_id: 0, // vm.current_frame.code_shard_id
        })
        .to_u256(),
    );
    address_operands_store(vm, opcode, meta_parameters);
    return;
}

pub fn _ergs_left(vm: &mut VMState, opcode: &Opcode) {
    let res = TaggedValue::new_raw_integer(U256::from(vm.current_frame.ergs_remaining));
    address_operands_store(vm, opcode, res);
    return;
}

pub fn _sp(vm: &mut VMState, opcode: &Opcode) {
    let sp = vm.current_frame.stack.sp();
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(U256::from(sp)));
    return;
}

pub fn _get_context_u128(vm: &mut VMState, opcode: &Opcode) {
    let res = TaggedValue::new_raw_integer(U256::from(vm.current_frame.context_u128));
    address_operands_store(vm, opcode, res);
    return;
}

pub fn _set_context_u128(vm: &mut VMState, opcode: &Opcode) {
    let (src0, _) = address_operands_read(vm, opcode);
    vm.current_frame.context_u128 = src0.value.as_u128();
    return;
}

pub fn _aux_mutating0(_vm: &mut VMState, _opcode: &Opcode) {
    // unkown behaviour, should not be called
    panic!("aux_mutating0 called");
}

pub fn _increment_tx_number(vm: &mut VMState, _opcode: &Opcode) {
    vm.tx_number_in_block += 1;
    return;
}
