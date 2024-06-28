use u256::U256;

use crate::{state::VMState, store::Storage, value::TaggedValue, Opcode};

pub fn _storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index).value;
    let value = vm.get_register(opcode.src1_index).value;
    vm.storage
        .borrow_mut()
        .store((vm.current_context().address, key), value)
        .unwrap();
}

pub fn _storage_read(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index);
    let value = vm
        .storage
        .borrow()
        .read(&(vm.current_context().address, key.value))
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
}

pub fn _transient_storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index).value;
    let value: U256 = vm.get_register(opcode.src1_index).value;
    let address = vm.current_context().address;
    vm.current_context_mut()
        .transient_storage
        .store((address, key), value)
        .unwrap();
}

pub fn _transient_storage_read(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index).value;
    let value = vm
        .current_context()
        .transient_storage
        .read(&(vm.current_context().address, key))
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
}
