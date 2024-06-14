use crate::{
    ptr_operator::{ptr_operands_read, ptr_operands_store},
    state::VMState,
    Opcode,
};

pub fn _ptr_sub(vm: &mut VMState, opcode: &Opcode) {
    let (pointer, diff, src1) = ptr_operands_read(vm, opcode, "ptr_sub");
    let (new_offset, overflow) = pointer.offset.overflowing_sub(diff);
    if overflow {
        panic!("Offset overflow in ptr_sub");
    }
    ptr_operands_store(vm, opcode, new_offset, pointer, src1);
}
