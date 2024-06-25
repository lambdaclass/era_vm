use u256::U256;

use crate::{state::VMState, store::Storage, value::TaggedValue, Opcode};

pub fn _storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index).value;
    let value = vm.get_register(opcode.src1_index).value;
    vm.current_frame
        .storage
        .borrow_mut()
        .store(key, value)
        .unwrap();
}

pub fn _storage_read(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index);
    let value = vm
        .current_frame
        .storage
        .borrow()
        .read(&key.value)
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
}

pub fn _transient_storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index).value;
    let value = vm.get_register(opcode.src1_index).value;
    vm.current_frame
        .transient_storage
        .store(key, value)
        .unwrap();
}

pub fn _transient_storage_read(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index).value;
    let value = vm
        .current_frame
        .transient_storage
        .read(&key)
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
}
