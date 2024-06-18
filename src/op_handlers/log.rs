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
