use crate::{
    ptr_operator::{ptr_operands_read, ptr_operands_store}, state::VMState, value::FatPointer, Opcode
};

pub fn _ptr_add(vm: &mut VMState, opcode: &Opcode) {
    let (pointer, diff, src0) = ptr_operands_read(vm, opcode, "ptr_add");
    let (new_offset, overflow) = pointer.offset.overflowing_add(diff);
    if overflow {
        panic!("Offset overflow in ptr_add");
    }
    let new_pointer = FatPointer {
        offset: new_offset,
        ..pointer
    };
    ptr_operands_store(vm, opcode, new_pointer, src0);
}
