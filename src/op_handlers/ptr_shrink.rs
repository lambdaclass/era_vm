use crate::{
    ptr_operator::{ptr_operands_read, ptr_operands_store}, state::VMState, value::FatPointer, Opcode
};

pub fn _ptr_shrink(vm: &mut VMState, opcode: &Opcode) {
    let (pointer, diff, src0) = ptr_operands_read(vm, opcode, "ptr_shrink");
    let (new_len, overflow) = pointer.len.overflowing_sub(diff);
    if overflow {
        panic!("Len overflow in ptr_shrink");
    }
    let new_pointer = FatPointer {
        len: new_len,
        ..pointer
    };
    ptr_operands_store(vm, opcode, new_pointer, src0);
}
