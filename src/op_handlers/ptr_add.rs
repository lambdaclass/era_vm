

use zkevm_opcode_defs::MAX_OFFSET_FOR_ADD_SUB;

use crate::{address_operands::{ address_operands_read, address_operands_store}, state::VMState, value::{FatPointer, TaggedValue}, Opcode};

pub fn _ptr_add(vm: &mut VMState, opcode: &Opcode) {
    let (src0,src1) = address_operands_read(vm, opcode);

    if !src0.is_pointer || src1.is_pointer {
        panic!("Invalid operands for ptr_add");
    }
    
    let pointer = FatPointer::decode(src0.value);
    if src1.value > MAX_OFFSET_FOR_ADD_SUB{
        panic!("Offset too large for ptr_add");
    }
    let diff = src1.value.low_u32();
    let (new_offset, overflow) = pointer.offset.overflowing_add(diff);
    if overflow {
        panic!("Offset overflow in ptr_add");
    }
    let new_pointer = FatPointer {
        offset: new_offset,
        ..pointer
    };
    let encoded_ptr = new_pointer.encode();    
    let res = TaggedValue::new_pointer(((src1.value >> 128) << 128) | encoded_ptr);
    address_operands_store(vm, opcode, res)
}
