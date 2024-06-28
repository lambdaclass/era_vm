use u256::U256;

use crate::{state::VMState, store::Storage, value::TaggedValue, Opcode};

pub fn _storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context().contract_address;
    let key = (address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    vm.current_context().storage.store(key, value).unwrap();
}

pub fn _storage_read(vm: &mut VMState, opcode: &Opcode) {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context().contract_address;
    let key = (address, key_for_contract_storage);
    let value = vm
        .current_context()
        .storage
        .read(&key)
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
}

pub fn _transient_storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context().contract_address;
    let key = (address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    vm.current_context_mut()
        .transient_storage
        .store(key, value)
        .unwrap();
}

pub fn _transient_storage_read(vm: &mut VMState, opcode: &Opcode) {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context().contract_address;
    let key = (address, key_for_contract_storage);
    let value = vm
        .current_context()
        .transient_storage
        .read(&key)
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
}
