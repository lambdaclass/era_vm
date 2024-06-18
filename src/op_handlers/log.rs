use u256::U256;

use crate::{state::VMState, Opcode};

pub fn _storage_write(vm: &mut VMState, opcode: &Opcode) {
    let key = vm.get_register(opcode.src0_index);
    let value = vm.get_register(opcode.src1_index);
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
        .read(&key)
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, value);
}
