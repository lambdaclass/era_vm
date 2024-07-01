use u256::U256;

use crate::{state::VMState, store::Storage, value::TaggedValue, Opcode};

pub fn _storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_frame().contract_address;
    let key = (address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    dbg!("Before storing", &vm.storage);
    vm.storage.contract_storage_store(key, value).unwrap();
    dbg!("After storing", &vm.storage);
}

pub fn _storage_read(vm: &mut VMState, opcode: &Opcode) {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_frame().contract_address;
    let key = (address, key_for_contract_storage);
    let value = vm
        .storage
        .contract_storage_read(&key)
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
}

pub fn _transient_storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_frame().contract_address;
    let key = (address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    vm.current_frame_mut()
        .transient_storage
        .contract_storage_store(key, value)
        .unwrap();
}

pub fn _transient_storage_read(vm: &mut VMState, opcode: &Opcode) {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_frame().contract_address;
    let key = (address, key_for_contract_storage);
    let value = vm
        .current_frame()
        .transient_storage
        .contract_storage_read(&key)
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
}
